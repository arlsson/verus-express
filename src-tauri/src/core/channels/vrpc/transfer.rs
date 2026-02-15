//
// Advanced VRPC transfer preflight (reserve-transfer/sendcurrency family).
// Builds tx templates server-side and stores signing payload by preflight_id.

use std::io::Cursor;
use std::time::Duration;

use bitcoin::consensus::Decodable;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::core::channels::vrpc::preflight::{VrpcInputRef, VrpcPreflightPayload};
use crate::core::channels::vrpc::provider::VrpcProvider;
use crate::types::transaction::PreflightWarning;
use crate::types::{VrpcTransferPreflightParams, VrpcTransferPreflightResult, WalletError};

const SATOSHIS_PER_COIN: i64 = 100_000_000;
const DEFAULT_PARENT_FEE: f64 = 0.0001;
const DEFAULT_PARENT_FEE_SAT: i64 = 10_000;
const TRANSFER_PREFLIGHT_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone)]
struct FundingUtxo {
    txid: String,
    vout: u32,
    satoshis: i64,
    script_pub_key: String,
}

fn parse_i64(value: Option<&Value>) -> Option<i64> {
    let v = value?;
    if let Some(x) = v.as_i64() {
        return Some(x);
    }
    if let Some(x) = v.as_u64() {
        return i64::try_from(x).ok();
    }
    if let Some(x) = v.as_f64() {
        return Some(x as i64);
    }
    if let Some(x) = v.as_str() {
        if let Ok(parsed) = x.parse::<i64>() {
            return Some(parsed);
        }
    }
    None
}

fn parse_u32(value: Option<&Value>) -> Option<u32> {
    parse_i64(value).and_then(|x| (x >= 0).then_some(x as u32))
}

fn parse_string(value: Option<&Value>) -> Option<String> {
    value?.as_str().map(ToString::to_string)
}

fn parse_sendcurrency_hex(raw: Value) -> Result<String, WalletError> {
    if let Some(hex) = raw.as_str() {
        return Ok(hex.to_string());
    }
    if let Some(hex) = parse_string(raw.get("hextx").or(raw.get("hex")).or(raw.get("txhex"))) {
        return Ok(hex);
    }
    Err(WalletError::OperationFailed)
}

fn parse_fee_sat(value: Option<&Value>, fallback_sat: i64) -> i64 {
    let fallback = fallback_sat.max(1);
    let normalize = |candidate: i64| if candidate > 0 { candidate } else { fallback };

    let Some(v) = value else {
        return fallback;
    };

    if let Some(raw_sat) = v.as_i64() {
        return normalize(raw_sat);
    }
    if let Some(raw_sat) = v.as_u64() {
        return i64::try_from(raw_sat)
            .map(normalize)
            .unwrap_or(fallback);
    }
    if let Some(raw_coin) = v.as_f64() {
        let raw_sat = ((raw_coin.max(0.0)) * SATOSHIS_PER_COIN as f64).round() as i64;
        return normalize(raw_sat);
    }
    if let Some(raw_str) = v.as_str() {
        let trimmed = raw_str.trim();
        if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
            if let Ok(raw_coin) = trimmed.parse::<f64>() {
                let raw_sat = ((raw_coin.max(0.0)) * SATOSHIS_PER_COIN as f64).round() as i64;
                return normalize(raw_sat);
            }
            return fallback;
        }
        if let Ok(raw_sat) = trimmed.parse::<i64>() {
            return normalize(raw_sat);
        }
    }

    fallback
}

fn parse_fund_result(raw: Value) -> Result<(String, i64), WalletError> {
    let hex = parse_string(raw.get("hex")).ok_or(WalletError::OperationFailed)?;
    let fee_sat = parse_fee_sat(raw.get("fee"), DEFAULT_PARENT_FEE_SAT);
    Ok((hex, fee_sat))
}

fn parse_funding_utxos(raw: &Value) -> Vec<FundingUtxo> {
    let Some(arr) = raw.as_array() else {
        return vec![];
    };

    arr.iter()
        .filter_map(|entry| {
            let txid = parse_string(entry.get("txid").or(entry.get("outputTxId")))?;
            let vout = parse_u32(entry.get("outputIndex").or(entry.get("vout")))?;
            let satoshis = parse_i64(entry.get("satoshis").or(entry.get("amount"))).unwrap_or(0);
            let script_pub_key = parse_string(entry.get("script").or(entry.get("scriptPubKey")))?;
            let is_spendable = parse_i64(entry.get("isspendable")).unwrap_or(1) != 0;
            if !is_spendable || satoshis <= 0 {
                return None;
            }
            Some(FundingUtxo {
                txid,
                vout,
                satoshis,
                script_pub_key,
            })
        })
        .collect()
}

fn sat_to_decimal_string(sat: i64) -> String {
    format!("{:.8}", sat as f64 / SATOSHIS_PER_COIN as f64)
}

fn parse_amount_sat(amount: &str) -> Result<i64, WalletError> {
    let parsed = amount
        .trim()
        .parse::<f64>()
        .map_err(|_| WalletError::OperationFailed)?;
    let sat = (parsed * SATOSHIS_PER_COIN as f64).round() as i64;
    if sat <= 0 {
        return Err(WalletError::OperationFailed);
    }
    Ok(sat)
}

async fn normalize_destination(
    provider: &VrpcProvider,
    destination: &str,
) -> Result<(String, Vec<PreflightWarning>), WalletError> {
    let trimmed = destination.trim();
    if trimmed.is_empty() {
        return Err(WalletError::InvalidAddress);
    }
    if trimmed.ends_with('@') {
        let raw = provider.getidentity(trimmed).await?;
        let identity_addr = parse_string(
            raw.get("identity")
                .and_then(|id| id.get("identityaddress"))
                .or(raw.get("identityaddress")),
        )
        .ok_or(WalletError::InvalidAddress)?;

        let warning = PreflightWarning {
            warning_type: "resolved_destination".to_string(),
            message: format!(
                "Destination handle {} resolved to {}.",
                trimmed, identity_addr
            ),
        };
        return Ok((identity_addr, vec![warning]));
    }

    Ok((trimmed.to_string(), Vec::new()))
}

fn build_sendcurrency_output(
    params: &VrpcTransferPreflightParams,
    normalized_destination: &str,
) -> Result<Value, WalletError> {
    let mut out = Map::<String, Value>::new();
    out.insert(
        "currency".to_string(),
        Value::String(params.coin_id.clone()),
    );
    out.insert(
        "amount".to_string(),
        Value::from(
            params
                .amount
                .trim()
                .parse::<f64>()
                .map_err(|_| WalletError::OperationFailed)?,
        ),
    );
    out.insert(
        "address".to_string(),
        Value::String(normalized_destination.to_string()),
    );

    if let Some(v) = &params.convert_to {
        out.insert("convertto".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = &params.export_to {
        out.insert("exportto".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = &params.via {
        out.insert("via".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = &params.fee_currency {
        out.insert("feecurrency".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = &params.fee_satoshis {
        out.insert("feesatoshis".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = params.preconvert {
        out.insert("preconvert".to_string(), Value::Bool(v));
    }
    if let Some(v) = &params.map_to {
        out.insert("mapto".to_string(), Value::String(v.clone()));
    }
    if let Some(v) = &params.vdxf_tag {
        out.insert("vdxftag".to_string(), Value::String(v.clone()));
    }

    Ok(Value::Object(out))
}

fn collect_payload_inputs(
    funded_hex: &str,
    available_utxos: &[FundingUtxo],
) -> Result<Vec<VrpcInputRef>, WalletError> {
    let raw = hex::decode(funded_hex.trim_start_matches("0x"))
        .or_else(|_| hex::decode(funded_hex))
        .map_err(|_| WalletError::OperationFailed)?;
    let mut cursor = Cursor::new(&raw[..]);
    let funded_tx: bitcoin::Transaction = bitcoin::Transaction::consensus_decode(&mut cursor)
        .map_err(|_| WalletError::OperationFailed)?;

    let mut payload_inputs = Vec::new();
    for input in funded_tx.input {
        let in_hash = input.previous_output.txid.to_string();
        let in_vout = input.previous_output.vout;
        let utxo = available_utxos
            .iter()
            .find(|u| u.txid == in_hash && u.vout == in_vout)
            .ok_or(WalletError::OperationFailed)?;
        payload_inputs.push(VrpcInputRef {
            txid: utxo.txid.clone(),
            vout: utxo.vout,
            satoshis: utxo.satoshis,
            script_pub_key: Some(utxo.script_pub_key.clone()),
        });
    }
    Ok(payload_inputs)
}

pub async fn preflight_transfer(
    params: VrpcTransferPreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    from_address: &str,
    channel_id: &str,
    provider: &VrpcProvider,
) -> Result<VrpcTransferPreflightResult, WalletError> {
    let submitted_sat = parse_amount_sat(&params.amount)?;
    let (normalized_destination, mut warnings) =
        normalize_destination(provider, &params.destination).await?;
    let output = build_sendcurrency_output(&params, &normalized_destination)?;

    let sendcurrency_result = provider
        .sendcurrency(from_address, &[output], 1, DEFAULT_PARENT_FEE, true)
        .await?;
    let unfunded_hex = parse_sendcurrency_hex(sendcurrency_result)?;

    let available_utxos = parse_funding_utxos(
        &provider
            .getaddressutxos(&[from_address.to_string()])
            .await?,
    );
    if available_utxos.is_empty() {
        return Err(WalletError::InsufficientFunds);
    }

    let funding_utxos: Vec<Value> = available_utxos
        .iter()
        .map(|utxo| serde_json::json!({"txid": utxo.txid, "voutnum": utxo.vout}))
        .collect();
    let funded_raw = provider
        .fundrawtransaction_with_options(
            &unfunded_hex,
            Some(&funding_utxos),
            Some(from_address),
            Some(DEFAULT_PARENT_FEE),
        )
        .await?;
    let (funded_hex, fee_sat) = parse_fund_result(funded_raw)?;
    let payload_inputs = collect_payload_inputs(&funded_hex, &available_utxos)?;
    if payload_inputs.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    if params.convert_to.is_some() || params.export_to.is_some() {
        warnings.push(PreflightWarning {
            warning_type: "estimated_fee".to_string(),
            message: "Final reserve-transfer amounts may vary slightly after funding and chain execution.".to_string(),
        });
    }

    let preflight_id = Uuid::new_v4().to_string();
    let payload = VrpcPreflightPayload {
        hex: funded_hex,
        inputs: payload_inputs,
        to_address: normalized_destination.clone(),
        from_address: from_address.to_string(),
        value: sat_to_decimal_string(submitted_sat),
        fee: sat_to_decimal_string(fee_sat),
    };
    let payload_value = serde_json::to_value(&payload).map_err(|_| WalletError::OperationFailed)?;

    preflight_store.put_with_ttl(
        preflight_id.clone(),
        PreflightRecord {
            channel_id: channel_id.to_string(),
            account_id: account_id.to_string(),
            payload: payload_value,
        },
        Some(TRANSFER_PREFLIGHT_TTL),
    );

    Ok(VrpcTransferPreflightResult {
        preflight_id,
        fee: sat_to_decimal_string(fee_sat),
        fee_currency: params.coin_id.clone(),
        value: sat_to_decimal_string(submitted_sat),
        amount_submitted: params.amount,
        amount_adjusted: None,
        to_address: normalized_destination,
        from_address: from_address.to_string(),
        warnings,
        memo: params.memo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base_params() -> VrpcTransferPreflightParams {
        VrpcTransferPreflightParams {
            coin_id: "VRSC".to_string(),
            channel_id: "vrpc.Rabc.i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            source_address: None,
            destination: "Rdest".to_string(),
            amount: "1.0".to_string(),
            convert_to: None,
            export_to: None,
            via: None,
            fee_currency: None,
            fee_satoshis: None,
            preconvert: None,
            map_to: None,
            vdxf_tag: None,
            memo: None,
        }
    }

    #[test]
    fn build_sendcurrency_output_includes_optional_route_flags() {
        let mut params = base_params();
        params.convert_to = Some("Bridge.CHIPS".to_string());
        params.export_to = Some("CHIPS".to_string());
        params.via = Some("Bridge.vETH".to_string());
        params.fee_currency = Some("i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string());
        params.fee_satoshis = Some("20000".to_string());
        params.preconvert = Some(true);
        params.map_to = Some("Bridge.vETH".to_string());
        params.vdxf_tag = Some("iTag".to_string());

        let output = build_sendcurrency_output(&params, "Rdest").expect("output");
        assert_eq!(
            output,
            json!({
                "currency": "VRSC",
                "amount": 1.0,
                "address": "Rdest",
                "convertto": "Bridge.CHIPS",
                "exportto": "CHIPS",
                "via": "Bridge.vETH",
                "feecurrency": "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV",
                "feesatoshis": "20000",
                "preconvert": true,
                "mapto": "Bridge.vETH",
                "vdxftag": "iTag"
            })
        );
    }

    #[test]
    fn parse_sendcurrency_hex_supports_multiple_result_shapes() {
        assert_eq!(
            parse_sendcurrency_hex(json!("deadbeef")).expect("string shape"),
            "deadbeef"
        );
        assert_eq!(
            parse_sendcurrency_hex(json!({"hextx": "cafe"})).expect("hextx shape"),
            "cafe"
        );
        assert_eq!(
            parse_sendcurrency_hex(json!({"hex": "babe"})).expect("hex shape"),
            "babe"
        );
    }

    #[test]
    fn parse_fund_result_uses_default_fee_when_non_positive_reported() {
        let (hex, fee_sat) = parse_fund_result(json!({"hex": "deadbeef", "fee": 0})).expect("parse");
        assert_eq!(hex, "deadbeef");
        assert_eq!(fee_sat, DEFAULT_PARENT_FEE_SAT);
    }

    #[test]
    fn parse_fund_result_parses_decimal_coin_fee() {
        let (_, fee_sat) =
            parse_fund_result(json!({"hex": "deadbeef", "fee": 0.0001})).expect("parse");
        assert_eq!(fee_sat, DEFAULT_PARENT_FEE_SAT);
    }
}
