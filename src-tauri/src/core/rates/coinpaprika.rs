//
// CoinPaprika price fetcher for fiat rate updates.
// Endpoint shape: /v1/tickers/{id} -> { quotes: { USD: { price, percent_change_24h } } }

use std::collections::HashMap;
use std::sync::OnceLock;

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::core::coins::CoinDefinition;

const COINPAPRIKA_BASE_URL: &str = "https://api.coinpaprika.com";
const VERUS_CATALOG_JSON: &str =
    include_str!("../../../../src/lib/coins/verusCoinCatalog.generated.json");

static CATALOG_COINPAPRIKA_OVERRIDES: OnceLock<HashMap<String, String>> = OnceLock::new();

pub struct CoinPaprikaUsdMetrics {
    pub usd_price: f64,
    pub usd_change_24h_pct: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CoinPaprikaIdSource {
    ExplicitOverride,
    CatalogOverride,
    InferredSlug,
}

impl CoinPaprikaIdSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitOverride => "coin.coin_paprika_id",
            Self::CatalogOverride => "verus_catalog",
            Self::InferredSlug => "inferred_slug",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogCoinEntry {
    id: String,
    currency_id: String,
    is_testnet: bool,
    coin_paprika_id: Option<String>,
}

fn catalog_lookup_key(value: &str, is_testnet: bool) -> String {
    format!(
        "{}:{}",
        if is_testnet { "testnet" } else { "mainnet" },
        value.to_ascii_lowercase()
    )
}

fn load_catalog_coinpaprika_overrides() -> HashMap<String, String> {
    let entries = match serde_json::from_str::<Vec<CatalogCoinEntry>>(VERUS_CATALOG_JSON) {
        Ok(entries) => entries,
        Err(err) => {
            println!(
                "[RATES] Failed to parse verus coin catalog for CoinPaprika overrides: {}",
                err
            );
            return HashMap::new();
        }
    };

    let mut overrides = HashMap::<String, String>::new();
    for entry in entries {
        let Some(coin_paprika_id) = entry.coin_paprika_id.as_deref() else {
            continue;
        };

        let normalized_override = coin_paprika_id.trim();
        if normalized_override.is_empty() {
            continue;
        }

        for key in [&entry.id, &entry.currency_id] {
            let normalized_key = key.trim();
            if normalized_key.is_empty() {
                continue;
            }
            overrides.insert(
                catalog_lookup_key(normalized_key, entry.is_testnet),
                normalized_override.to_string(),
            );
        }
    }

    overrides
}

fn catalog_coinpaprika_id_for_coin(coin: &CoinDefinition) -> Option<String> {
    let overrides = CATALOG_COINPAPRIKA_OVERRIDES.get_or_init(load_catalog_coinpaprika_overrides);
    for key in [&coin.id, &coin.currency_id] {
        let normalized = key.trim();
        if normalized.is_empty() {
            continue;
        }

        let lookup = catalog_lookup_key(normalized, coin.is_testnet);
        if let Some(hit) = overrides.get(&lookup) {
            return Some(hit.clone());
        }
    }

    None
}

pub fn infer_coinpaprika_id(coin: &CoinDefinition) -> String {
    resolve_coinpaprika_id(coin).0
}

fn resolve_coinpaprika_id(coin: &CoinDefinition) -> (String, CoinPaprikaIdSource) {
    if let Some(override_id) = coin.coin_paprika_id.as_deref() {
        if !override_id.trim().is_empty() {
            return (
                override_id.trim().to_string(),
                CoinPaprikaIdSource::ExplicitOverride,
            );
        }
    }

    if let Some(catalog_override) = catalog_coinpaprika_id_for_coin(coin) {
        return (catalog_override, CoinPaprikaIdSource::CatalogOverride);
    }

    let normalized_name = coin
        .display_name
        .replace(' ', "-")
        .replace('.', "-")
        .to_ascii_lowercase();
    (
        format!("{}-{}", coin.id.to_ascii_lowercase(), normalized_name),
        CoinPaprikaIdSource::InferredSlug,
    )
}

pub async fn fetch_usd_metrics(
    client: &Client,
    coin: &CoinDefinition,
) -> Result<CoinPaprikaUsdMetrics, String> {
    let (coinpaprika_id, id_source) = resolve_coinpaprika_id(coin);
    let source = format!("{}/v1/tickers/{}", COINPAPRIKA_BASE_URL, coinpaprika_id);

    let response = client
        .get(&source)
        .send()
        .await
        .map_err(|e| format!("coinpaprika request failed: {}", e))?;
    if !response.status().is_success() {
        return Err(format!(
            "coinpaprika returned HTTP {} for {} (resolved_id={} via {})",
            response.status(),
            coin.id,
            coinpaprika_id,
            id_source.as_str()
        ));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|e| format!("coinpaprika json parse failed: {}", e))?;

    let (usd_price, usd_change_24h_pct) =
        parse_usd_metrics_from_ticker_payload(&payload, &coin.id)?;

    Ok(CoinPaprikaUsdMetrics {
        usd_price,
        usd_change_24h_pct,
    })
}

fn parse_usd_metrics_from_ticker_payload(
    payload: &Value,
    coin_id: &str,
) -> Result<(f64, Option<f64>), String> {
    let usd = payload
        .get("quotes")
        .and_then(|q| q.get("USD"))
        .ok_or_else(|| format!("coinpaprika USD quote missing for {}", coin_id))?;

    let usd_price = usd
        .get("price")
        .and_then(value_to_f64)
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| format!("coinpaprika USD price missing for {}", coin_id))?;

    let usd_change_24h_pct = usd
        .get("percent_change_24h")
        .and_then(value_to_f64)
        .filter(|v| v.is_finite());

    Ok((usd_price, usd_change_24h_pct))
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
    use serde_json::json;
    use std::time::Duration;

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

    fn sample_unlisted_coin() -> CoinDefinition {
        CoinDefinition {
            id: "FAKE".to_string(),
            currency_id: "FAKE".to_string(),
            system_id: "FAKE".to_string(),
            display_ticker: "FAKE".to_string(),
            display_name: "Fake Coin".to_string(),
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
        let coin = sample_unlisted_coin();
        assert_eq!(infer_coinpaprika_id(&coin), "fake-fake-coin");
    }

    #[test]
    fn infer_coinpaprika_id_uses_catalog_override_when_available() {
        let coin = CoinDefinition {
            id: "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd".to_string(),
            currency_id: "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd".to_string(),
            system_id: "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X".to_string(),
            display_ticker: "vUSDC.vETH".to_string(),
            display_name: "USDC on Verus".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![],
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: Some("USDC".to_string()),
            is_testnet: false,
        };

        assert_eq!(infer_coinpaprika_id(&coin), "usdc-usd-coin");
    }

    #[test]
    fn infer_coinpaprika_id_uses_catalog_override_for_dynamic_registry_shapes() {
        let cases = [
            (
                CoinDefinition {
                    id: "iGBs4DWztRNvNEJBt4mqHszLxfKTNHTkhM".to_string(),
                    currency_id: "iGBs4DWztRNvNEJBt4mqHszLxfKTNHTkhM".to_string(),
                    system_id: "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X".to_string(),
                    display_ticker: "DAI".to_string(),
                    display_name: "DAI.vETH".to_string(),
                    coin_paprika_id: None,
                    proto: Protocol::Vrsc,
                    compatible_channels: vec![Channel::Vrpc],
                    decimals: 8,
                    vrpc_endpoints: vec!["https://api.verus.services/".to_string()],
                    electrum_endpoints: None,
                    seconds_per_block: 60,
                    mapped_to: None,
                    is_testnet: false,
                },
                "dai-dai",
            ),
            (
                CoinDefinition {
                    id: "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd".to_string(),
                    currency_id: "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd".to_string(),
                    system_id: "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X".to_string(),
                    display_ticker: "vUSDC".to_string(),
                    display_name: "vUSDC.vETH".to_string(),
                    coin_paprika_id: None,
                    proto: Protocol::Vrsc,
                    compatible_channels: vec![Channel::Vrpc],
                    decimals: 8,
                    vrpc_endpoints: vec!["https://api.verus.services/".to_string()],
                    electrum_endpoints: None,
                    seconds_per_block: 60,
                    mapped_to: None,
                    is_testnet: false,
                },
                "usdc-usd-coin",
            ),
            (
                CoinDefinition {
                    id: "iC5TQFrFXSYLQGkiZ8FYmZHFJzaRF5CYgE".to_string(),
                    currency_id: "iC5TQFrFXSYLQGkiZ8FYmZHFJzaRF5CYgE".to_string(),
                    system_id: "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X".to_string(),
                    display_ticker: "EURC".to_string(),
                    display_name: "EURC.vETH".to_string(),
                    coin_paprika_id: None,
                    proto: Protocol::Vrsc,
                    compatible_channels: vec![Channel::Vrpc],
                    decimals: 8,
                    vrpc_endpoints: vec!["https://api.verus.services/".to_string()],
                    electrum_endpoints: None,
                    seconds_per_block: 60,
                    mapped_to: None,
                    is_testnet: false,
                },
                "eurc-eurc",
            ),
        ];

        for (coin, expected) in cases {
            assert_eq!(infer_coinpaprika_id(&coin), expected);
        }
    }

    #[test]
    fn infer_coinpaprika_id_does_not_leak_system_level_override() {
        let bridge = CoinDefinition {
            id: "i3f7tSctFkiPpiedY8QR5Tep9p4qDVebDx".to_string(),
            currency_id: "i3f7tSctFkiPpiedY8QR5Tep9p4qDVebDx".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "Bridge.vETH".to_string(),
            display_name: "Bridge.vETH".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec!["https://api.verus.services/".to_string()],
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: Some("0xE6052Dcc60573561ECef2D9A4C0FEA6d3aC5B9A2".to_string()),
            is_testnet: false,
        };
        let pure = CoinDefinition {
            id: "iHax5qYQGbcMGqJKKrPorpzUBX2oFFXGnY".to_string(),
            currency_id: "iHax5qYQGbcMGqJKKrPorpzUBX2oFFXGnY".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "Pure".to_string(),
            display_name: "Pure".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec!["https://api.verus.services/".to_string()],
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        };

        let bridge_id = infer_coinpaprika_id(&bridge);
        let pure_id = infer_coinpaprika_id(&pure);
        assert_ne!(bridge_id, "vrsc-verus-coin");
        assert_ne!(pure_id, "vrsc-verus-coin");
        assert_eq!(bridge_id, "i3f7tsctfkippiedy8qr5tep9p4qdvebdx-bridge-veth");
        assert_eq!(pure_id, "ihax5qyqgbcmgqjkkrporpzubx2offxgny-pure");
    }

    #[test]
    fn embedded_catalog_contains_coinpaprika_overrides_for_bridge_assets() {
        assert!(VERUS_CATALOG_JSON.contains("\"coinPaprikaId\": \"usdc-usd-coin\""));
        assert!(VERUS_CATALOG_JSON.contains("\"coinPaprikaId\": \"eurc-eurc\""));
        assert!(VERUS_CATALOG_JSON.contains("\"coinPaprikaId\": \"dai-dai\""));
    }

    #[tokio::test]
    #[ignore = "network smoke test"]
    async fn fetch_usd_metrics_resolves_bridge_assets_via_catalog_override_smoke() {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("http client");

        let cases = vec![
            CoinDefinition {
                id: "iGBs4DWztRNvNEJBt4mqHszLxfKTNHTkhM".to_string(),
                currency_id: "iGBs4DWztRNvNEJBt4mqHszLxfKTNHTkhM".to_string(),
                system_id: "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X".to_string(),
                display_ticker: "DAI".to_string(),
                display_name: "DAI.vETH".to_string(),
                coin_paprika_id: None,
                proto: Protocol::Vrsc,
                compatible_channels: vec![Channel::Vrpc],
                decimals: 8,
                vrpc_endpoints: vec!["https://api.verus.services/".to_string()],
                electrum_endpoints: None,
                seconds_per_block: 60,
                mapped_to: None,
                is_testnet: false,
            },
            CoinDefinition {
                id: "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd".to_string(),
                currency_id: "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd".to_string(),
                system_id: "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X".to_string(),
                display_ticker: "vUSDC".to_string(),
                display_name: "vUSDC.vETH".to_string(),
                coin_paprika_id: None,
                proto: Protocol::Vrsc,
                compatible_channels: vec![Channel::Vrpc],
                decimals: 8,
                vrpc_endpoints: vec!["https://api.verus.services/".to_string()],
                electrum_endpoints: None,
                seconds_per_block: 60,
                mapped_to: None,
                is_testnet: false,
            },
            CoinDefinition {
                id: "iC5TQFrFXSYLQGkiZ8FYmZHFJzaRF5CYgE".to_string(),
                currency_id: "iC5TQFrFXSYLQGkiZ8FYmZHFJzaRF5CYgE".to_string(),
                system_id: "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X".to_string(),
                display_ticker: "EURC".to_string(),
                display_name: "EURC.vETH".to_string(),
                coin_paprika_id: None,
                proto: Protocol::Vrsc,
                compatible_channels: vec![Channel::Vrpc],
                decimals: 8,
                vrpc_endpoints: vec!["https://api.verus.services/".to_string()],
                electrum_endpoints: None,
                seconds_per_block: 60,
                mapped_to: None,
                is_testnet: false,
            },
        ];

        for coin in cases {
            let metrics = fetch_usd_metrics(&client, &coin)
                .await
                .expect("coinpaprika fetch should succeed");
            assert!(
                metrics.usd_price > 0.0,
                "usd price should be positive for {}",
                coin.id
            );
        }
    }

    #[test]
    fn extracts_metrics_from_ticker_payload() {
        let payload = json!({
            "quotes": {
                "USD": {
                    "price": 2.315,
                    "percent_change_24h": -1.84
                }
            }
        });

        let (usd_price, usd_change_24h_pct) =
            parse_usd_metrics_from_ticker_payload(&payload, "VRSC").expect("metrics should parse");

        assert_eq!(usd_price, 2.315);
        assert_eq!(usd_change_24h_pct, Some(-1.84));
    }

    #[test]
    fn missing_percent_change_24h_is_allowed() {
        let payload = json!({
            "quotes": {
                "USD": {
                    "price": 2.315
                }
            }
        });

        let (usd_price, usd_change_24h_pct) =
            parse_usd_metrics_from_ticker_payload(&payload, "VRSC").expect("metrics should parse");

        assert_eq!(usd_price, 2.315);
        assert_eq!(usd_change_24h_pct, None);
    }

    #[test]
    fn missing_or_invalid_price_is_rejected() {
        let payload = json!({
            "quotes": {
                "USD": {
                    "price": 0
                }
            }
        });

        let err = parse_usd_metrics_from_ticker_payload(&payload, "VRSC")
            .expect_err("zero USD price should fail");
        assert_eq!(err, "coinpaprika USD price missing for VRSC");
    }
}
