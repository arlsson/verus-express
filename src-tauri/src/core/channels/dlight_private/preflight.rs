use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::channels::dlight_private::destination::{
    classify_dlight_destination, DlightDestinationKind,
};
use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::types::transaction::{PreflightParams, PreflightResult};
use crate::types::WalletError;

use super::DlightRuntimeRequest;

const SATOSHIS_PER_COIN: i128 = 100_000_000;
const FIXED_FEE_SATS: i128 = 10_000; // 0.0001

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlightPreflightPayload {
    pub coin_id: String,
    pub destination_kind: DlightDestinationKind,
    pub to_address: String,
    pub from_address: String,
    pub value: String,
    pub fee: String,
    pub memo: Option<String>,
}

pub async fn preflight(
    params: PreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    channel_id: &str,
    request: DlightRuntimeRequest,
) -> Result<PreflightResult, WalletError> {
    let to_address = params.to_address.trim().to_string();
    let destination_kind = classify_dlight_destination(&to_address, request.network)?;

    // Guard at backend as well: block preflight while private sync is incomplete.
    let info = super::get_info(request.clone()).await?;
    if info.syncing {
        let percent = info.percent.unwrap_or(0.0);
        if (percent - 100.0).abs() > f64::EPSILON && (percent + 1.0).abs() > f64::EPSILON {
            return Err(WalletError::DlightSynchronizerNotReady);
        }
    }

    let balances = super::get_balances(request.clone()).await?;
    let confirmed_balance = parse_decimal_to_satoshis(&balances.confirmed)?;
    let _pending_balance = parse_decimal_to_satoshis(&balances.pending)?;
    if confirmed_balance <= 0 {
        return Err(WalletError::InsufficientFunds);
    }

    let submitted_sat = parse_positive_satoshis(&params.amount)?;
    let (value_sat, fee_taken_from_amount, fee_taken_message) =
        resolve_send_value(submitted_sat, confirmed_balance, FIXED_FEE_SATS)?;
    let fee_string = satoshis_to_decimal_string(FIXED_FEE_SATS);
    let value_string = satoshis_to_decimal_string(value_sat);

    let memo = match destination_kind {
        DlightDestinationKind::Shielded => normalize_optional_memo(params.memo),
        DlightDestinationKind::Transparent => None,
    };

    let preflight_id = Uuid::new_v4().to_string();
    let payload = DlightPreflightPayload {
        coin_id: params.coin_id.clone(),
        destination_kind,
        to_address: to_address.clone(),
        from_address: request.scope_address.clone(),
        value: value_string.clone(),
        fee: fee_string.clone(),
        memo: memo.clone(),
    };
    let payload_value = serde_json::to_value(&payload).map_err(|_| WalletError::OperationFailed)?;

    preflight_store.put(
        preflight_id.clone(),
        PreflightRecord {
            channel_id: channel_id.to_string(),
            account_id: account_id.to_string(),
            payload: payload_value,
        },
    );

    Ok(PreflightResult {
        preflight_id,
        fee: fee_string,
        fee_currency: params.coin_id,
        value: value_string,
        amount_submitted: params.amount,
        to_address,
        from_address: request.scope_address,
        fee_taken_from_amount,
        fee_taken_message,
        warnings: vec![],
        memo,
    })
}

fn normalize_optional_memo(memo: Option<String>) -> Option<String> {
    memo.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn resolve_send_value(
    submitted_sat: i128,
    confirmed_balance: i128,
    fee_sat: i128,
) -> Result<(i128, bool, Option<String>), WalletError> {
    if submitted_sat <= 0 || confirmed_balance <= 0 || fee_sat <= 0 {
        return Err(WalletError::OperationFailed);
    }

    let deducted_amount = submitted_sat.saturating_add(fee_sat);

    if deducted_amount == confirmed_balance.saturating_add(fee_sat) {
        let adjusted = submitted_sat.saturating_sub(fee_sat);
        if adjusted <= 0 {
            return Err(WalletError::InsufficientFunds);
        }
        return Ok((
            adjusted,
            true,
            Some(
                "Fee was deducted from the submitted amount due to available balance.".to_string(),
            ),
        ));
    }

    if deducted_amount > confirmed_balance {
        return Err(WalletError::InsufficientFunds);
    }

    Ok((submitted_sat, false, None))
}

fn parse_positive_satoshis(value: &str) -> Result<i128, WalletError> {
    let parsed = parse_decimal_to_satoshis(value)?;
    if parsed <= 0 {
        return Err(WalletError::OperationFailed);
    }
    Ok(parsed)
}

fn parse_decimal_to_satoshis(value: &str) -> Result<i128, WalletError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    let (is_negative, numeric) = if let Some(rest) = trimmed.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        (false, rest)
    } else {
        (false, trimmed)
    };
    if numeric.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    let mut parts = numeric.split('.');
    let whole_part = parts.next().unwrap_or_default();
    let frac_part = parts.next();
    if parts.next().is_some() {
        return Err(WalletError::OperationFailed);
    }

    if !whole_part.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(WalletError::OperationFailed);
    }

    let whole_sat = if whole_part.is_empty() {
        0i128
    } else {
        whole_part
            .parse::<i128>()
            .map_err(|_| WalletError::OperationFailed)?
            .checked_mul(SATOSHIS_PER_COIN)
            .ok_or(WalletError::OperationFailed)?
    };

    let mut frac_sat = 0i128;
    if let Some(frac) = frac_part {
        if !frac.chars().all(|ch| ch.is_ascii_digit()) || frac.len() > 8 {
            return Err(WalletError::OperationFailed);
        }
        if !frac.is_empty() {
            let padded = format!("{frac:0<8}");
            frac_sat = padded
                .parse::<i128>()
                .map_err(|_| WalletError::OperationFailed)?;
        }
    }

    let combined = whole_sat
        .checked_add(frac_sat)
        .ok_or(WalletError::OperationFailed)?;
    Ok(if is_negative { -combined } else { combined })
}

fn satoshis_to_decimal_string(value: i128) -> String {
    let is_negative = value.is_negative();
    let absolute = value.unsigned_abs();
    let whole = absolute / SATOSHIS_PER_COIN as u128;
    let frac = absolute % SATOSHIS_PER_COIN as u128;
    if is_negative {
        format!("-{whole}.{frac:08}")
    } else {
        format!("{whole}.{frac:08}")
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_decimal_to_satoshis, resolve_send_value, FIXED_FEE_SATS};
    use crate::types::WalletError;

    #[test]
    fn parse_decimal_to_satoshis_handles_valid_values() {
        assert_eq!(parse_decimal_to_satoshis("1").expect("parse"), 100_000_000);
        assert_eq!(parse_decimal_to_satoshis("0.00000001").expect("parse"), 1);
        assert_eq!(
            parse_decimal_to_satoshis("10.25000000").expect("parse"),
            1_025_000_000
        );
    }

    #[test]
    fn parse_decimal_to_satoshis_rejects_invalid_precision() {
        assert!(matches!(
            parse_decimal_to_satoshis("0.000000001"),
            Err(WalletError::OperationFailed)
        ));
        assert!(matches!(
            parse_decimal_to_satoshis("not-a-number"),
            Err(WalletError::OperationFailed)
        ));
    }

    #[test]
    fn resolve_send_value_deducts_fee_on_max_send() {
        let submitted = 2_000_000i128;
        let confirmed = 2_000_000i128;
        let (value, fee_taken, message) =
            resolve_send_value(submitted, confirmed, FIXED_FEE_SATS).expect("max send");

        assert_eq!(value, submitted - FIXED_FEE_SATS);
        assert!(fee_taken);
        assert!(message.is_some());
    }

    #[test]
    fn resolve_send_value_rejects_insufficient_funds() {
        let result = resolve_send_value(1_000_000, 500_000, FIXED_FEE_SATS);
        assert!(matches!(result, Err(WalletError::InsufficientFunds)));
    }
}
