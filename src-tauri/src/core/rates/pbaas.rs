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
const MIN_POSITIVE_RESERVE: f64 = 1e-12;

#[derive(Debug, Clone)]
struct RootMarketQuote {
    via_currency_id: String,
    anchor_per_source: f64,
}

pub fn is_pbaas_derivation_candidate(coin: &CoinDefinition) -> bool {
    if coin.proto != Protocol::Vrsc {
        return false;
    }

    let currency_id = coin.currency_id.trim();
    !currency_id.eq_ignore_ascii_case(VRSC_SYSTEM_ID)
        && !currency_id.eq_ignore_ascii_case(VRSCTEST_SYSTEM_ID)
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

    if is_root_system_currency(coin) {
        return derive_root_market_rates(provider, coin, latest_rates).await;
    }

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
        .and_then(|v| v.as_object())
        .or_else(|| {
            currency_result
                .get("lastconfirmedcurrencystate")
                .and_then(|v| v.get("currencies"))
                .and_then(|v| v.as_object())
        })?;

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

fn is_root_system_currency(coin: &CoinDefinition) -> bool {
    let currency_id = coin.currency_id.trim();
    let system_id = coin.system_id.trim();
    !currency_id.is_empty() && currency_id.eq_ignore_ascii_case(system_id)
}

fn anchor_reserve_id_for_coin(coin: &CoinDefinition) -> &'static str {
    if coin.is_testnet {
        VRSCTEST_SYSTEM_ID
    } else {
        VRSC_SYSTEM_ID
    }
}

fn anchor_coin_id_for_coin(coin: &CoinDefinition) -> &'static str {
    if coin.is_testnet {
        VRSCTEST_COIN_ID
    } else {
        VRSC_COIN_ID
    }
}

async fn derive_root_market_rates(
    provider: &VrpcProvider,
    coin: &CoinDefinition,
    latest_rates: &HashMap<String, HashMap<String, f64>>,
) -> Option<HashMap<String, f64>> {
    let anchor_coin_id = anchor_coin_id_for_coin(coin);
    let anchor_rates = latest_rates.get(anchor_coin_id)?;
    let anchor_reserve_id = anchor_reserve_id_for_coin(coin);

    let list_payload = provider.listcurrencies().await.ok()?;
    let market_quote = find_root_market_quote_in_list_payload(
        &list_payload,
        &coin.currency_id,
        anchor_reserve_id,
    )?;

    let estimated_anchor_out = provider
        .estimateconversion(
            &coin.currency_id,
            anchor_reserve_id,
            1.0,
            Some(&market_quote.via_currency_id),
            None,
        )
        .await
        .ok()
        .and_then(|estimate| estimate.get("estimatedcurrencyout").and_then(value_to_f64))
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(market_quote.anchor_per_source);

    let mut derived_rates = HashMap::<String, f64>::new();
    for (fiat, anchor_rate) in anchor_rates {
        if anchor_rate.is_finite() && *anchor_rate > 0.0 {
            derived_rates.insert(fiat.clone(), estimated_anchor_out * *anchor_rate);
        }
    }

    if derived_rates.is_empty() {
        None
    } else {
        Some(derived_rates)
    }
}

fn find_root_market_quote_in_list_payload(
    list_payload: &Value,
    source_currency_id: &str,
    anchor_reserve_id: &str,
) -> Option<RootMarketQuote> {
    let entries = list_payload.as_array()?;

    let mut best: Option<(RootMarketQuote, f64, f64)> = None;
    for entry in entries {
        let definition = entry.get("currencydefinition").unwrap_or(entry);
        let Some(via_currency_id) = definition
            .get("currencyid")
            .or_else(|| definition.get("currency_id"))
            .and_then(value_to_trimmed_string)
        else {
            continue;
        };
        if via_currency_id.eq_ignore_ascii_case(source_currency_id) {
            continue;
        }

        let state = entry
            .get("bestcurrencystate")
            .or_else(|| entry.get("lastconfirmedcurrencystate"));
        let currencies = state
            .and_then(|value| value.get("currencies"))
            .and_then(|value| value.as_object());
        let Some(currencies) = currencies else {
            continue;
        };

        let Some(source_price) =
            extract_last_conversion_price_from_map(currencies, source_currency_id)
        else {
            continue;
        };
        let Some(anchor_price) =
            extract_last_conversion_price_from_map(currencies, anchor_reserve_id)
        else {
            continue;
        };
        if !(source_price.is_finite() && source_price > 0.0) {
            continue;
        }
        if !(anchor_price.is_finite() && anchor_price > 0.0) {
            continue;
        }

        let reserves = state
            .and_then(|value| value.get("reservecurrencies"))
            .and_then(|value| value.as_array());
        let Some(reserves) = reserves else {
            continue;
        };

        let source_reserves =
            extract_reserves_for_currency(reserves, source_currency_id).unwrap_or(0.0);
        let anchor_reserves =
            extract_reserves_for_currency(reserves, anchor_reserve_id).unwrap_or(0.0);
        if source_reserves <= MIN_POSITIVE_RESERVE || anchor_reserves <= MIN_POSITIVE_RESERVE {
            continue;
        }

        let anchor_per_source = anchor_price / source_price;
        if !(anchor_per_source.is_finite() && anchor_per_source > 0.0) {
            continue;
        }

        let quote = RootMarketQuote {
            via_currency_id,
            anchor_per_source,
        };

        match &best {
            Some((_, best_anchor_reserves, best_source_reserves))
                if anchor_reserves < *best_anchor_reserves
                    || (anchor_reserves == *best_anchor_reserves
                        && source_reserves <= *best_source_reserves) => {}
            _ => best = Some((quote, anchor_reserves, source_reserves)),
        }
    }

    best.map(|(quote, _, _)| quote)
}

fn extract_last_conversion_price_from_map(
    currencies: &serde_json::Map<String, Value>,
    currency_id: &str,
) -> Option<f64> {
    currencies
        .iter()
        .find(|(key, _)| key.trim().eq_ignore_ascii_case(currency_id.trim()))
        .and_then(|(_, value)| value.get("lastconversionprice").and_then(value_to_f64))
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn extract_reserves_for_currency(reserves: &[Value], currency_id: &str) -> Option<f64> {
    reserves.iter().find_map(|reserve| {
        let object = reserve.as_object()?;
        let reserve_currency_id = object.get("currencyid").and_then(value_to_trimmed_string)?;
        if !reserve_currency_id.eq_ignore_ascii_case(currency_id.trim()) {
            return None;
        }
        object
            .get("reserves")
            .and_then(value_to_f64)
            .filter(|value| value.is_finite() && *value > 0.0)
    })
}

fn value_to_trimmed_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
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
            dlight_endpoints: None,
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

    #[test]
    fn derive_from_currency_result_uses_lastconfirmed_state_when_best_state_missing() {
        let coin = sample_pbaas_coin();
        let currency_result = serde_json::json!({
          "lastconfirmedcurrencystate": {
            "currencies": {
              "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV": {
                "lastconversionprice": 1.0
              }
            }
          }
        });
        let latest_rates = HashMap::from([(
            "VRSC".to_string(),
            HashMap::from([("USD".to_string(), 1.8)]),
        )]);

        let derived = derive_from_currency_result(&coin, &currency_result, &latest_rates)
            .expect("derived rates");
        assert_eq!(derived.get("USD"), Some(&1.8));
    }

    #[test]
    fn pbaas_derivation_candidate_includes_root_system_currency() {
        let mut coin = sample_pbaas_coin();
        coin.currency_id = "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N".to_string();
        coin.system_id = "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N".to_string();

        assert!(is_pbaas_derivation_candidate(&coin));
    }

    #[test]
    fn pbaas_derivation_candidate_excludes_anchor_currencies() {
        let mut vrsc = sample_pbaas_coin();
        vrsc.currency_id = VRSC_SYSTEM_ID.to_string();
        vrsc.system_id = VRSC_SYSTEM_ID.to_string();

        let mut vrsctest = sample_pbaas_coin();
        vrsctest.currency_id = VRSCTEST_SYSTEM_ID.to_string();
        vrsctest.system_id = VRSCTEST_SYSTEM_ID.to_string();

        assert!(!is_pbaas_derivation_candidate(&vrsc));
        assert!(!is_pbaas_derivation_candidate(&vrsctest));
    }

    #[test]
    fn derive_from_currency_result_supports_root_system_currency() {
        let mut coin = sample_pbaas_coin();
        coin.currency_id = "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N".to_string();
        coin.system_id = "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N".to_string();

        let currency_result = serde_json::json!({
          "bestcurrencystate": {
            "currencies": {
              "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV": {
                "lastconversionprice": 1.0
              }
            }
          }
        });
        let latest_rates = HashMap::from([(
            "VRSC".to_string(),
            HashMap::from([("USD".to_string(), 2.4)]),
        )]);

        let derived = derive_from_currency_result(&coin, &currency_result, &latest_rates)
            .expect("derived rates");
        assert_eq!(derived.get("USD"), Some(&2.4));
    }

    #[test]
    fn root_market_quote_ignores_zero_reserve_baskets() {
        let list_payload = serde_json::json!([
          {
            "currencydefinition": { "currencyid": "iZeroReservesBasket" },
            "bestcurrencystate": {
              "currencies": {
                "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N": { "lastconversionprice": 0.0002 },
                "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV": { "lastconversionprice": 0.00004 }
              },
              "reservecurrencies": [
                { "currencyid": "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N", "reserves": 0.0 },
                { "currencyid": "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV", "reserves": 0.0 }
              ]
            }
          },
          {
            "currencydefinition": { "currencyid": "iLiquidBasket" },
            "bestcurrencystate": {
              "currencies": {
                "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N": { "lastconversionprice": 71.90538515 },
                "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV": { "lastconversionprice": 45.19207169 }
              },
              "reservecurrencies": [
                { "currencyid": "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N", "reserves": 108322.97303243 },
                { "currencyid": "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV", "reserves": 340388.07046252 }
              ]
            }
          }
        ]);

        let quote = find_root_market_quote_in_list_payload(
            &list_payload,
            "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N",
            VRSC_SYSTEM_ID,
        )
        .expect("quote");
        assert_eq!(quote.via_currency_id, "iLiquidBasket");
        assert!((quote.anchor_per_source - 0.6285).abs() < 0.001);
    }

    #[test]
    fn root_market_quote_prefers_more_liquid_anchor_reserve() {
        let list_payload = serde_json::json!([
          {
            "currencydefinition": { "currencyid": "iSmallerPool" },
            "bestcurrencystate": {
              "currencies": {
                "iSource": { "lastconversionprice": 2.0 },
                "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV": { "lastconversionprice": 1.0 }
              },
              "reservecurrencies": [
                { "currencyid": "iSource", "reserves": 2000.0 },
                { "currencyid": "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV", "reserves": 1000.0 }
              ]
            }
          },
          {
            "currencydefinition": { "currencyid": "iLargerPool" },
            "bestcurrencystate": {
              "currencies": {
                "iSource": { "lastconversionprice": 4.0 },
                "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV": { "lastconversionprice": 3.0 }
              },
              "reservecurrencies": [
                { "currencyid": "iSource", "reserves": 8000.0 },
                { "currencyid": "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV", "reserves": 5000.0 }
              ]
            }
          }
        ]);

        let quote =
            find_root_market_quote_in_list_payload(&list_payload, "iSource", VRSC_SYSTEM_ID)
                .expect("quote");
        assert_eq!(quote.via_currency_id, "iLargerPool");
        assert_eq!(quote.anchor_per_source, 0.75);
    }
}
