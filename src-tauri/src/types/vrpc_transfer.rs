//
// Advanced VRPC transfer preflight types (reserve-transfer/sendcurrency family).
// Security: preflight returns display data only; signing remains keyed by preflight_id.

use serde::{Deserialize, Serialize};

use crate::types::transaction::PreflightWarning;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VrpcTransferPreflightParams {
    pub coin_id: String,
    pub channel_id: String,
    pub source_address: Option<String>,
    pub destination: String,
    pub amount: String,
    pub convert_to: Option<String>,
    pub export_to: Option<String>,
    pub via: Option<String>,
    pub fee_currency: Option<String>,
    pub fee_satoshis: Option<String>,
    pub preconvert: Option<bool>,
    pub map_to: Option<String>,
    pub vdxf_tag: Option<String>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VrpcTransferPreflightResult {
    pub preflight_id: String,
    pub fee: String,
    pub fee_currency: String,
    pub value: String,
    pub amount_submitted: String,
    pub amount_adjusted: Option<String>,
    pub to_address: String,
    pub from_address: String,
    pub warnings: Vec<PreflightWarning>,
    pub memo: Option<String>,
}
