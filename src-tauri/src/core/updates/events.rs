//
// Module 7: Tauri event payloads for update engine. Frontend listens via listen().
// All payloads use camelCase for frontend contract.

use serde::Serialize;

use crate::types::transaction::Transaction;

/// Payload for wallet://balances-updated.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalancesUpdatedPayload {
    pub coin_id: String,
    pub channel: String,
    pub confirmed: String,
    pub pending: String,
    pub total: String,
}

/// Payload for wallet://transactions-updated.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionsUpdatedPayload {
    pub coin_id: String,
    pub channel: String,
    pub transactions: Vec<Transaction>,
}

/// Payload for wallet://info-updated (chain sync info). Optional for first version.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoUpdatedPayload {
    pub coin_id: String,
    pub channel: String,
    pub percent: Option<f64>,
    pub blocks: Option<u64>,
    pub longest_chain: Option<u64>,
    pub syncing: bool,
    pub status_kind: Option<String>,
    pub last_updated: Option<u64>,
    pub last_progress_at: Option<u64>,
    pub stalled: Option<bool>,
    pub scan_rate_blocks_per_sec: Option<f64>,
}

/// Payload for wallet://rates-updated.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RatesUpdatedPayload {
    pub coin_id: String,
    pub rates: std::collections::HashMap<String, f64>,
    pub usd_change_24h_pct: Option<f64>,
}

/// Payload for wallet://bootstrap-updated.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapUpdatedPayload {
    pub in_progress: bool,
}

/// Payload for wallet://tx-send-progress.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TxSendProgressPayload {
    pub channel: String,
    pub coin_id: String,
    pub stage: String,
}

/// Payload for wallet://error. Message must be user-facing, no internal details.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateErrorPayload {
    pub data_type: String,
    pub coin_id: String,
    pub channel: String,
    pub message: String,
}
