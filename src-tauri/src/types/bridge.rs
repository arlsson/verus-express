//
// Bridge command-surface types (backend-first; UI-agnostic).
// Phase-1 scope: typed contracts and VRPC bridge preflight adapter.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::transaction::PreflightWarning;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeConversionPathRequest {
    pub coin_id: String,
    pub channel_id: String,
    pub source_currency: String,
    pub destination_currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeConversionPathQuote {
    pub destination_id: String,
    pub destination_display_name: Option<String>,
    pub destination_display_ticker: Option<String>,
    pub convert_to: Option<String>,
    pub convert_to_display_name: Option<String>,
    pub export_to: Option<String>,
    pub export_to_display_name: Option<String>,
    pub via: Option<String>,
    pub via_display_name: Option<String>,
    pub map_to: Option<String>,
    pub price: Option<String>,
    pub via_price_in_root: Option<String>,
    pub dest_price_in_via: Option<String>,
    pub gateway: bool,
    pub mapping: bool,
    pub bounceback: bool,
    pub eth_destination: bool,
    #[serde(default)]
    pub prelaunch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeConversionPathsResult {
    pub source_currency: String,
    pub paths: HashMap<String, Vec<BridgeConversionPathQuote>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeConversionEstimateRequest {
    pub coin_id: String,
    pub channel_id: String,
    pub source_currency: String,
    pub convert_to: String,
    pub amount: String,
    pub via: Option<String>,
    pub preconvert: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeConversionEstimateResult {
    pub estimated_currency_out: Option<String>,
    pub price: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeTransferPreflightParams {
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
pub struct BridgeTransferRoute {
    pub convert_to: Option<String>,
    pub export_to: Option<String>,
    pub via: Option<String>,
    pub map_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeExecutionHint {
    pub engine: String,
    pub requires_token_approval: bool,
    pub bridge_contract: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeTransferPreflightResult {
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
    pub route: BridgeTransferRoute,
    pub execution: BridgeExecutionHint,
}
