//
// PBaaS fallback rate derivation.
// If direct price feeds fail, derive fiat from bestcurrencystate lastconversionprice
// and known reserve anchor rates (VRSC / VRSCTEST).

use std::collections::HashMap;

use serde_json::Value;

use crate::core::channels::vrpc::VrpcProvider;
use crate::core::coins::{CoinDefinition, Protocol};

const VRSC_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
const VRSCTEST_SYSTEM_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";
const VRSC_COIN_ID: &str = "VRSC";
const VRSCTEST_COIN_ID: &str = "VRSCTEST";

pub fn is_pbaas_derivation_candidate(coin: &CoinDefinition) -> bool {
    coin.proto == Protocol::Vrsc && coin.currency_id != coin.system_id
}

pub async fn derive_pbaas_rates(
    provider: &VrpcProvider,
    coin: &CoinDefinition,
    latest_rates: &HashMap<String, HashMap<String, f64>>,
) -> Option<HashMap<String, f64>> {
    if !is_pbaas_derivation_candidate(coin) {
        return None;
    }

    let payload = provider.getcurrency(&coin.currency_id).await.ok()?;
    let result = payload.get("result").unwrap_or(&payload);
    derive_from_currency_result(coin, result, latest_rates)
}

pub fn derive_from_currency_result(
    coin: &CoinDefinition,
    currency_result: &Value,
    latest_rates: &HashMap<String, HashMap<String, f64>>,
) -> Option<HashMap<String, f64>> {
    if !is_pbaas_derivation_candidate(coin) {
        return None;
    }

    let reserve_states = currency_result
        .get("bestcurrencystate")
        .and_then(|v| v.get("currencies"))
        .and_then(|v| v.as_object())?;

    for (reserve_id, reserve_state) in reserve_states {
        let Some(last_conversion_price) = reserve_state
            .get("lastconversionprice")
            .and_then(value_to_f64)
            .filter(|v| v.is_finite() && *v > 0.0)
        else {
            continue;
        };
        let Some(anchor_coin_id) = map_reserve_id_to_anchor_coin_id(reserve_id) else {
            continue;
        };
        let Some(anchor_rates) = latest_rates.get(anchor_coin_id) else {
            continue;
        };

        let mut derived_rates = HashMap::<String, f64>::new();
        for (fiat, anchor_rate) in anchor_rates {
            if anchor_rate.is_finite() && *anchor_rate > 0.0 {
                derived_rates.insert(fiat.clone(), last_conversion_price * *anchor_rate);
            }
        }

        if !derived_rates.is_empty() {
            return Some(derived_rates);
        }
    }

    None
}

fn map_reserve_id_to_anchor_coin_id(reserve_id: &str) -> Option<&'static str> {
    match reserve_id {
        VRSC_SYSTEM_ID => Some(VRSC_COIN_ID),
        VRSCTEST_SYSTEM_ID => Some(VRSCTEST_COIN_ID),
        _ => None,
    }
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
    use crate::core::coins::{Channel, Protocol};

    fn sample_pbaas_coin() -> CoinDefinition {
        CoinDefinition {
            id: "iPBaaS".to_string(),
            currency_id: "iPBaaS".to_string(),
            system_id: "iSystem".to_string(),
            display_ticker: "PB".to_string(),
            display_name: "PBaaS".to_string(),
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
    fn derive_from_currency_result_uses_anchor_rates() {
        let coin = sample_pbaas_coin();
        let currency_result = serde_json::json!({
          "bestcurrencystate": {
            "currencies": {
              "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV": {
                "lastconversionprice": 2.5
              }
            }
          }
        });
        let latest_rates = HashMap::from([(
            "VRSC".to_string(),
            HashMap::from([("USD".to_string(), 1.2), ("EUR".to_string(), 1.1)]),
        )]);

        let derived = derive_from_currency_result(&coin, &currency_result, &latest_rates)
            .expect("derived rates");
        assert_eq!(derived.get("USD"), Some(&(3.0)));
        assert_eq!(derived.get("EUR"), Some(&(2.75)));
    }

    #[test]
    fn derive_from_currency_result_returns_none_without_anchor() {
        let coin = sample_pbaas_coin();
        let currency_result = serde_json::json!({
          "bestcurrencystate": {
            "currencies": {
              "iUnknownReserve": {
                "lastconversionprice": 2.5
              }
            }
          }
        });
        let latest_rates = HashMap::new();
        assert!(derive_from_currency_result(&coin, &currency_result, &latest_rates).is_none());
    }
}
