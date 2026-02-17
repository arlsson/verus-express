//
// Module 5: VRPC transaction history via getaddressdeltas and getaddressmempool.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::core::channels::vrpc::{VrpcCoinContext, VrpcTransactionsResult};
use crate::types::transaction::Transaction;
use crate::types::WalletError;

const RESERVE_TRANSFER_DESTINATION_ADDRESS: &str = "RTqQe58LSj2yr5CrwYFwcsAQ1edQwmrkUU";

/// Fetch transaction history for addresses with coin context.
pub async fn get_transactions(
    provider: &VrpcProvider,
    addresses: &[String],
    coin: &VrpcCoinContext,
) -> Result<VrpcTransactionsResult, WalletError> {
    if addresses.is_empty() {
        return Ok(VrpcTransactionsResult {
            transactions: vec![],
            warning: None,
        });
    }

    let deltas = provider.getaddressdeltas(addresses).await?;
    let mut warning_messages: Vec<String> = Vec::new();

    let mempool = match provider.getaddressmempool(addresses).await {
        Ok(v) => Some(v),
        Err(_) => {
            warning_messages.push("Mempool temporarily unavailable".to_string());
            None
        }
    };

    let longest_chain = match provider.getinfo().await {
        Ok(info) => info
            .get("longestchain")
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64))),
        Err(_) => {
            warning_messages.push("Chain info unavailable".to_string());
            None
        }
    };

    let mut ordered_entries: Vec<(Value, bool)> = Vec::new();
    if let Some(Value::Array(arr)) = mempool {
        for entry in arr {
            ordered_entries.push((entry, true));
        }
    }

    let deltas_arr = deltas.as_array().ok_or(WalletError::OperationFailed)?;
    for entry in deltas_arr {
        ordered_entries.push((entry.clone(), false));
    }

    let txs = aggregate_transactions(ordered_entries, addresses, coin, longest_chain);

    let warning = if warning_messages.is_empty() {
        None
    } else {
        Some(warning_messages.join("; "))
    };

    Ok(VrpcTransactionsResult {
        transactions: txs,
        warning,
    })
}

fn aggregate_transactions(
    ordered_entries: Vec<(Value, bool)>,
    owner_addresses: &[String],
    coin: &VrpcCoinContext,
    longest_chain: Option<u64>,
) -> Vec<Transaction> {
    let owner_addresses_lower: Vec<String> = owner_addresses
        .iter()
        .map(|addr| addr.to_ascii_lowercase())
        .collect();
    let mut by_txid: HashMap<String, TxAggregate> = HashMap::new();

    for (entry, is_mempool) in ordered_entries {
        let Some(obj) = entry.as_object() else {
            continue;
        };

        let txid = obj
            .get("txid")
            .or(obj.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if txid.is_empty() {
            continue;
        }

        let delta = extract_delta(obj, coin);
        if delta.abs() < f64::EPSILON {
            continue;
        }

        let height = obj
            .get("height")
            .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|u| u as i64)));
        let blocktime = obj
            .get("blocktime")
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)));
        let address = obj
            .get("address")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let sent_output_addresses = extract_sent_output_addresses(obj, coin);

        let agg = by_txid.entry(txid).or_default();
        agg.delta += delta;
        agg.pending = agg.pending || is_mempool || height.unwrap_or(0) <= 0;
        if let Some(h) = height {
            if h > 0 {
                agg.height = Some(agg.height.map_or(h, |prev| prev.max(h)));
            }
        }
        if agg.blocktime.is_none() {
            agg.blocktime = blocktime;
        }
        if agg.address.is_none() {
            agg.address = address;
        }
        for candidate in sent_output_addresses {
            push_unique_address(&mut agg.sent_output_addresses, candidate);
        }
    }

    let mut txs: Vec<Transaction> = by_txid
        .into_iter()
        .filter_map(|(txid, agg)| {
            if agg.delta.abs() < f64::EPSILON {
                return None;
            }

            let confirmations = if agg.pending {
                0
            } else if let (Some(h), Some(longest)) = (agg.height, longest_chain) {
                if h > 0 && longest >= h as u64 {
                    (longest - h as u64 + 1) as i64
                } else {
                    1
                }
            } else if agg.height.unwrap_or(0) > 0 {
                1
            } else {
                0
            };

            let timestamp = match (agg.blocktime, agg.height, longest_chain) {
                (Some(ts), _, _) => Some(ts),
                (None, Some(h), Some(longest)) if h > 0 && longest >= h as u64 => {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .ok()
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let confirms = longest - h as u64;
                    let estimate_secs = confirms.saturating_mul(coin.seconds_per_block);
                    Some(now.saturating_sub(estimate_secs))
                }
                _ => None,
            };

            let amount = if coin.currency_id == coin.system_id {
                format!("{:.8}", agg.delta / 100_000_000.0)
            } else {
                format!("{:.*}", coin.decimals as usize, agg.delta)
            };

            let scope_address = agg.address.unwrap_or_default();
            let counterparty = select_sent_counterparty(
                &agg.sent_output_addresses,
                &owner_addresses_lower,
                &scope_address,
            )
            .unwrap_or_default();
            let (from_address, to_address) = if agg.delta < 0.0 {
                (scope_address, counterparty)
            } else {
                (counterparty, scope_address)
            };

            Some(Transaction {
                txid,
                amount,
                from_address,
                to_address,
                confirmations,
                timestamp,
                pending: agg.pending,
            })
        })
        .collect();

    txs.sort_by(|a, b| {
        b.pending
            .cmp(&a.pending)
            .then(b.timestamp.unwrap_or(0).cmp(&a.timestamp.unwrap_or(0)))
    });

    txs
}

#[derive(Default)]
struct TxAggregate {
    delta: f64,
    pending: bool,
    height: Option<i64>,
    blocktime: Option<u64>,
    address: Option<String>,
    sent_output_addresses: Vec<String>,
}

fn extract_delta(obj: &serde_json::Map<String, Value>, coin: &VrpcCoinContext) -> f64 {
    if coin.currency_id == coin.system_id {
        return obj
            .get("satoshis")
            .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            .unwrap_or(0) as f64;
    }

    obj.get("currencyvalues")
        .and_then(|cv| cv.get(&coin.currency_id))
        .and_then(value_as_f64)
        .unwrap_or(0.0)
}

fn value_as_f64(v: &Value) -> Option<f64> {
    if let Some(f) = v.as_f64() {
        return Some(f);
    }
    if let Some(i) = v.as_i64() {
        return Some(i as f64);
    }
    if let Some(u) = v.as_u64() {
        return Some(u as f64);
    }
    if let Some(s) = v.as_str() {
        return s.parse::<f64>().ok();
    }
    None
}

fn extract_sent_output_addresses(
    obj: &serde_json::Map<String, Value>,
    coin: &VrpcCoinContext,
) -> Vec<String> {
    let mut addresses = Vec::new();

    let Some(outputs) = obj
        .get("sent")
        .and_then(|value| value.get("outputs"))
        .and_then(|value| value.as_array())
    else {
        return addresses;
    };

    for output in outputs {
        let include_output = match output.get("amounts") {
            Some(Value::Object(amounts)) => amounts
                .get(&coin.currency_id)
                .and_then(value_as_f64)
                .map(|amount| amount > 0.0)
                .unwrap_or(false),
            Some(_) => false,
            None => true,
        };

        if !include_output {
            continue;
        }

        let Some(raw_addresses) = output.get("addresses") else {
            continue;
        };

        match raw_addresses {
            Value::String(address) => {
                push_unique_address(&mut addresses, address.to_string());
            }
            Value::Array(items) => {
                for item in items {
                    if let Some(address) = item.as_str() {
                        push_unique_address(&mut addresses, address.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    addresses
}

fn push_unique_address(addresses: &mut Vec<String>, candidate: String) {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return;
    }

    if addresses
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(trimmed))
    {
        return;
    }

    addresses.push(trimmed.to_string());
}

fn select_sent_counterparty(
    sent_output_addresses: &[String],
    owner_addresses_lower: &[String],
    scope_address: &str,
) -> Option<String> {
    for candidate in sent_output_addresses {
        let candidate_lower = candidate.to_ascii_lowercase();
        if candidate.eq_ignore_ascii_case(RESERVE_TRANSFER_DESTINATION_ADDRESS) {
            continue;
        }
        if candidate.eq_ignore_ascii_case(scope_address) {
            continue;
        }
        if owner_addresses_lower
            .iter()
            .any(|owner| owner == &candidate_lower)
        {
            continue;
        }
        return Some(candidate.clone());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_same_txid_deltas_and_drops_zero_net() {
        let coin = VrpcCoinContext {
            currency_id: "iNative".to_string(),
            system_id: "iNative".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let owner_addresses = vec!["Rwallet".to_string()];

        let entries = vec![
            (
                serde_json::json!({
                    "txid": "tx-1",
                    "satoshis": 100000000,
                    "height": 10
                }),
                false,
            ),
            (
                serde_json::json!({
                    "txid": "tx-1",
                    "satoshis": -50000000,
                    "height": 10
                }),
                false,
            ),
            (
                serde_json::json!({
                    "txid": "tx-zero",
                    "satoshis": 1000,
                    "height": 11
                }),
                false,
            ),
            (
                serde_json::json!({
                    "txid": "tx-zero",
                    "satoshis": -1000,
                    "height": 11
                }),
                false,
            ),
        ];

        let txs = aggregate_transactions(entries, &owner_addresses, &coin, Some(20));
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].txid, "tx-1");
        assert_eq!(txs[0].amount, "0.50000000");
        assert_eq!(txs[0].confirmations, 11);
    }

    #[test]
    fn mempool_entries_are_pending() {
        let coin = VrpcCoinContext {
            currency_id: "iNative".to_string(),
            system_id: "iNative".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let owner_addresses = vec!["Rwallet".to_string()];
        let entries = vec![(
            serde_json::json!({
                "txid": "tx-mempool",
                "satoshis": 100000,
                "height": -1
            }),
            true,
        )];

        let txs = aggregate_transactions(entries, &owner_addresses, &coin, Some(100));
        assert_eq!(txs.len(), 1);
        assert!(txs[0].pending);
        assert_eq!(txs[0].confirmations, 0);
    }

    #[test]
    fn outgoing_uses_sent_output_counterparty() {
        let coin = VrpcCoinContext {
            currency_id: "iNative".to_string(),
            system_id: "iNative".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let owner_addresses = vec!["Rwallet".to_string()];
        let entries = vec![(
            serde_json::json!({
                "txid": "tx-sent",
                "satoshis": -100000000,
                "height": 12,
                "address": "Rwallet",
                "sent": {
                    "outputs": [
                        {
                            "addresses": "Rwallet",
                            "amounts": { "iNative": 0.5 }
                        },
                        {
                            "addresses": "Rrecipient",
                            "amounts": { "iNative": 0.5 }
                        }
                    ]
                }
            }),
            false,
        )];

        let txs = aggregate_transactions(entries, &owner_addresses, &coin, Some(20));
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].from_address, "Rwallet");
        assert_eq!(txs[0].to_address, "Rrecipient");
    }

    #[test]
    fn outgoing_filters_reserve_transfer_destination() {
        let coin = VrpcCoinContext {
            currency_id: "iNative".to_string(),
            system_id: "iNative".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let owner_addresses = vec!["Rwallet".to_string()];
        let entries = vec![(
            serde_json::json!({
                "txid": "tx-reserve",
                "satoshis": -50000000,
                "height": 12,
                "address": "Rwallet",
                "sent": {
                    "outputs": [
                        {
                            "addresses": "Rwallet",
                            "amounts": { "iNative": 0.25 }
                        },
                        {
                            "addresses": "RTqQe58LSj2yr5CrwYFwcsAQ1edQwmrkUU",
                            "amounts": { "iNative": 0.25 }
                        }
                    ]
                }
            }),
            false,
        )];

        let txs = aggregate_transactions(entries, &owner_addresses, &coin, Some(20));
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].from_address, "Rwallet");
        assert!(txs[0].to_address.is_empty());
    }

    #[test]
    fn outgoing_ignores_counterparty_with_zero_amount_for_selected_coin() {
        let coin = VrpcCoinContext {
            currency_id: "iNative".to_string(),
            system_id: "iNative".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let owner_addresses = vec!["Rwallet".to_string()];
        let entries = vec![(
            serde_json::json!({
                "txid": "tx-zero-counterparty",
                "satoshis": -100000000,
                "height": 12,
                "address": "Rwallet",
                "sent": {
                    "outputs": [
                        {
                            "addresses": "Rwallet",
                            "amounts": { "iNative": 0.9998 }
                        },
                        {
                            "addresses": "Rexternal",
                            "amounts": {
                                "iNative": 0.0,
                                "iOther": 1.0
                            }
                        }
                    ]
                }
            }),
            false,
        )];

        let txs = aggregate_transactions(entries, &owner_addresses, &coin, Some(20));
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].from_address, "Rwallet");
        assert!(txs[0].to_address.is_empty());
    }
}
