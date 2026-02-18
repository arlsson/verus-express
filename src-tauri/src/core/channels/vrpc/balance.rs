//
// Module 5: VRPC balance via getaddressbalance. Maps to BalanceResult (decimal strings).

use serde_json::Value;

use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::core::channels::vrpc::VrpcCoinContext;
use crate::types::transaction::BalanceResult;
use crate::types::WalletError;

const SATOSHI_PER_COIN: f64 = 100_000_000.0;

fn satoshi_to_decimal(sat: f64) -> String {
    format!("{:.8}", sat / SATOSHI_PER_COIN)
}

/// Fetch balance for addresses and return as BalanceResult.
pub async fn get_balances(
    provider: &VrpcProvider,
    addresses: &[String],
    coin: &VrpcCoinContext,
) -> Result<BalanceResult, WalletError> {
    if addresses.is_empty() {
        return Ok(BalanceResult {
            confirmed: "0".to_string(),
            pending: "0".to_string(),
            total: "0".to_string(),
        });
    }

    let raw = provider.getaddressbalance(addresses).await?;
    Ok(map_balance_result(&raw, coin))
}

fn map_balance_result(raw: &Value, coin: &VrpcCoinContext) -> BalanceResult {
    // Mobile parity: pending for VRPC balances is represented through tx state, not balance math.
    let confirmed = if coin.currency_id == coin.system_id {
        // Native for the selected scope system: "balance" is satoshi value.
        if let Some(obj) = raw.as_object() {
            let sat = obj
                .get("balance")
                .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
                .unwrap_or(0.0);
            satoshi_to_decimal(sat)
        } else if let Some(n) = raw.as_f64() {
            satoshi_to_decimal(n)
        } else if let Some(n) = raw.as_i64() {
            satoshi_to_decimal(n as f64)
        } else {
            "0".to_string()
        }
    } else {
        // PBaaS: use currencybalance[currency_id] if present.
        if let Some(obj) = raw.as_object() {
            obj.get("currencybalance")
                .and_then(|v| v.get(&coin.currency_id))
                .and_then(|v| value_to_decimal_string(v, coin.decimals))
                .unwrap_or_else(|| "0".to_string())
        } else {
            "0".to_string()
        }
    };

    BalanceResult {
        confirmed: confirmed.clone(),
        pending: "0".to_string(),
        total: confirmed,
    }
}

fn value_to_decimal_string(v: &Value, decimals: u8) -> Option<String> {
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    if let Some(i) = v.as_i64() {
        return Some(format!("{:.*}", decimals as usize, i as f64));
    }
    if let Some(u) = v.as_u64() {
        return Some(format!("{:.*}", decimals as usize, u as f64));
    }
    if let Some(f) = v.as_f64() {
        return Some(format!("{:.*}", decimals as usize, f));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_balance_maps_to_confirmed_with_zero_pending() {
        let coin = VrpcCoinContext {
            currency_id: "iTest".to_string(),
            system_id: "iTest".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let raw = serde_json::json!({
            "balance": 123456789i64,
            "received": 999999999i64
        });

        let mapped = map_balance_result(&raw, &coin);
        assert_eq!(mapped.confirmed, "1.23456789");
        assert_eq!(mapped.pending, "0");
        assert_eq!(mapped.total, "1.23456789");
    }

    #[test]
    fn pbaas_balance_uses_currencybalance_entry() {
        let coin = VrpcCoinContext {
            currency_id: "iPBaaS".to_string(),
            system_id: "iSystem".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let raw = serde_json::json!({
            "balance": 0,
            "currencybalance": {
                "iPBaaS": "42.00000000"
            }
        });

        let mapped = map_balance_result(&raw, &coin);
        assert_eq!(mapped.confirmed, "42.00000000");
        assert_eq!(mapped.pending, "0");
        assert_eq!(mapped.total, "42.00000000");
    }

    #[test]
    fn scope_mismatch_for_native_chain_asset_uses_currencybalance_entry() {
        let coin = VrpcCoinContext {
            currency_id: "iVdex".to_string(),
            system_id: "iVrsc".to_string(),
            decimals: 8,
            seconds_per_block: 60,
        };
        let raw = serde_json::json!({
            "balance": 1665388586i64,
            "currencybalance": {
                "iVrsc": "16.65388586",
                "iVdex": "0.29248137"
            }
        });

        let mapped = map_balance_result(&raw, &coin);
        assert_eq!(mapped.confirmed, "0.29248137");
        assert_eq!(mapped.pending, "0");
        assert_eq!(mapped.total, "0.29248137");
    }
}
