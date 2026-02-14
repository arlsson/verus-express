//
// CoinPaprika price fetcher for fiat rate updates.
// Endpoint shape: /v1/coins/{id}/ohlcv/latest -> [{ close: <usd_price>, ... }]

use reqwest::Client;
use serde_json::Value;

use crate::core::coins::CoinDefinition;

const COINPAPRIKA_BASE_URL: &str = "https://api.coinpaprika.com";

pub fn infer_coinpaprika_id(coin: &CoinDefinition) -> String {
    if let Some(override_id) = coin.coin_paprika_id.as_deref() {
        if !override_id.trim().is_empty() {
            return override_id.trim().to_string();
        }
    }

    let normalized_name = coin
        .display_name
        .replace(' ', "-")
        .replace('.', "-")
        .to_ascii_lowercase();
    format!("{}-{}", coin.id.to_ascii_lowercase(), normalized_name)
}

pub async fn fetch_usd_close(
    client: &Client,
    coin: &CoinDefinition,
) -> Result<(f64, String), String> {
    let coinpaprika_id = infer_coinpaprika_id(coin);
    let source = format!(
        "{}/v1/coins/{}/ohlcv/latest",
        COINPAPRIKA_BASE_URL, coinpaprika_id
    );

    let response = client
        .get(&source)
        .send()
        .await
        .map_err(|e| format!("coinpaprika request failed: {}", e))?;
    if !response.status().is_success() {
        return Err(format!(
            "coinpaprika returned HTTP {} for {}",
            response.status(),
            coin.id
        ));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|e| format!("coinpaprika json parse failed: {}", e))?;

    let close = payload
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("close"))
        .and_then(value_to_f64)
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| format!("coinpaprika close price missing for {}", coin.id))?;

    Ok((close, source))
}

fn value_to_f64(value: &Value) -> Option<f64> {
    if let Some(v) = value.as_f64() {
        return Some(v);
    }
    if let Some(v) = value.as_i64() {
        return Some(v as f64);
    }
    if let Some(v) = value.as_u64() {
        return Some(v as f64);
    }
    value
        .as_str()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::coins::{Channel, CoinDefinition, Protocol};

    fn sample_coin() -> CoinDefinition {
        CoinDefinition {
            id: "VRSC".to_string(),
            currency_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "VRSC".to_string(),
            display_name: "Verus".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![],
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        }
    }

    #[test]
    fn infer_coinpaprika_id_uses_override_when_present() {
        let mut coin = sample_coin();
        coin.coin_paprika_id = Some("vrsc-verus-coin".to_string());
        assert_eq!(infer_coinpaprika_id(&coin), "vrsc-verus-coin");
    }

    #[test]
    fn infer_coinpaprika_id_falls_back_to_slug() {
        let coin = sample_coin();
        assert_eq!(infer_coinpaprika_id(&coin), "vrsc-verus");
    }
}
