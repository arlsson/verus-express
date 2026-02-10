//
// Module 5: VRPC preflight — validate address, build/fund tx, store record (incl. to/from/value/fee for send), return PreflightResult.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::types::transaction::{PreflightParams, PreflightResult, PreflightWarning};
use crate::types::WalletError;

/// Payload stored in PreflightStore for VRPC send. Not sent to frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrpcPreflightPayload {
    pub hex: String,
    #[serde(rename = "inputs")]
    pub inputs: Vec<VrpcInputRef>,
    pub to_address: String,
    pub from_address: String,
    pub value: String,
    pub fee: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrpcInputRef {
    pub txid: String,
    pub vout: u32,
    pub satoshis: i64,
    #[serde(default)]
    pub script_pub_key: Option<String>,
}

/// Minimal address check: non-empty, reasonable length.
fn validate_address(addr: &str) -> Result<(), WalletError> {
    let trimmed = addr.trim();
    if trimmed.is_empty() || trimmed.len() > 200 {
        return Err(WalletError::InvalidAddress);
    }
    // R-address, i-address, or VerusID: allow alphanumeric and @.
    if !trimmed
        .chars()
        .all(|c| c.is_alphanumeric() || c == '@' || c == '.' || c == '_' || c == '-')
    {
        return Err(WalletError::InvalidAddress);
    }
    Ok(())
}

/// Run VRPC preflight: validate, build/fund tx, store record, return UI result.
pub async fn preflight(
    params: PreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    from_address: &str,
    channel_id: &str,
    provider: &VrpcProvider,
) -> Result<PreflightResult, WalletError> {
    validate_address(&params.to_address)?;
    validate_address(from_address)?;

    let amount_sat = params
        .amount
        .trim()
        .parse::<f64>()
        .map_err(|_| WalletError::OperationFailed)?
        * 100_000_000.0;
    if amount_sat <= 0.0 {
        return Err(WalletError::OperationFailed);
    }

    let addresses = vec![from_address.to_string()];
    let utxos_raw = provider.getaddressutxos(&addresses).await?;

    let utxos: Vec<(String, u32, i64)> = parse_utxos(&utxos_raw)?;
    if utxos.is_empty() {
        return Err(WalletError::InsufficientFunds);
    }

    let fee_estimate = 10_000_i64;
    let needed = (amount_sat as i64) + fee_estimate;
    let mut selected = Vec::new();
    let mut total: i64 = 0;
    for (txid, vout, satoshis) in utxos {
        selected.push((txid.clone(), vout, satoshis));
        total += satoshis;
        if total >= needed {
            break;
        }
    }
    if total < (amount_sat as i64) {
        return Err(WalletError::InsufficientFunds);
    }

    let inputs: Vec<Value> = selected
        .iter()
        .map(|(txid, vout, _)| serde_json::json!({"txid": txid, "vout": *vout}))
        .collect();
    let amount_vrsc = amount_sat / 100_000_000.0;
    let outputs = serde_json::json!({ params.to_address.clone(): amount_vrsc });

    let hex_unfunded = provider
        .createrawtransaction(&inputs, &outputs)
        .await?
        .as_str()
        .ok_or(WalletError::OperationFailed)?
        .to_string();

    let funded = provider.fundrawtransaction(&hex_unfunded).await?;
    let funded_hex = funded
        .get("hex")
        .and_then(|v| v.as_str())
        .ok_or(WalletError::OperationFailed)?
        .to_string();
    let fee_sat = funded
        .get("fee")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        .unwrap_or(fee_estimate as f64);

    let payload_inputs: Vec<VrpcInputRef> = selected
        .iter()
        .map(|(txid, vout, satoshis)| VrpcInputRef {
            txid: txid.clone(),
            vout: *vout,
            satoshis: *satoshis,
            script_pub_key: None,
        })
        .collect();

    let preflight_id = Uuid::new_v4().to_string();
    let fee_str = format!("{:.8}", fee_sat / 100_000_000.0);
    let value_str = format!("{:.8}", amount_sat / 100_000_000.0);
    let payload = VrpcPreflightPayload {
        hex: funded_hex,
        inputs: payload_inputs,
        to_address: params.to_address.clone(),
        from_address: from_address.to_string(),
        value: value_str.clone(),
        fee: fee_str.clone(),
    };
    let payload_value = serde_json::to_value(&payload).map_err(|_| WalletError::OperationFailed)?;

    let record = PreflightRecord {
        channel_id: channel_id.to_string(),
        account_id: account_id.to_string(),
        payload: payload_value,
    };
    preflight_store.put(preflight_id.clone(), record);

    Ok(PreflightResult {
        preflight_id,
        fee: fee_str,
        fee_currency: params.coin_id.clone(),
        value: value_str.clone(),
        amount_submitted: params.amount,
        to_address: params.to_address.clone(),
        from_address: from_address.to_string(),
        fee_taken_from_amount: false,
        fee_taken_message: None,
        warnings: vec![],
        memo: params.memo,
    })
}

fn parse_utxos(raw: &Value) -> Result<Vec<(String, u32, i64)>, WalletError> {
    let arr = raw.as_array().ok_or(WalletError::OperationFailed)?;
    let mut out = Vec::new();
    for entry in arr {
        let obj = entry.as_object().ok_or(WalletError::OperationFailed)?;
        let txid = obj
            .get("txid")
            .or(obj.get("outputTxId"))
            .and_then(|v| v.as_str())
            .ok_or(WalletError::OperationFailed)?
            .to_string();
        let vout = obj
            .get("vout")
            .or(obj.get("outputIndex"))
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)))
            .ok_or(WalletError::OperationFailed)? as u32;
        let satoshis = obj
            .get("satoshis")
            .or(obj.get("amount"))
            .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            .unwrap_or(0);
        out.push((txid, vout, satoshis));
    }
    Ok(out)
}
