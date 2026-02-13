//
// Guard session and signed-out identity flow types.

use serde::{Deserialize, Serialize};

use crate::types::identity::{
    IdentityPreflightParams, IdentityPreflightResult, IdentitySendResult,
};
use crate::types::wallet::WalletNetwork;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GuardImportMode {
    Mnemonic24,
    TextAuto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginGuardSessionRequest {
    pub import_text: String,
    pub import_mode: GuardImportMode,
    pub network: WalletNetwork,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginGuardSessionResult {
    pub guard_session_id: String,
    pub secret_kind: String,
    pub vrsc_address: String,
    pub eth_address: String,
    pub btc_address: String,
    pub network: WalletNetwork,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndGuardSessionRequest {
    pub guard_session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndGuardSessionResult {
    pub ended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardIdentityPreflightRequest {
    pub guard_session_id: String,
    pub params: IdentityPreflightParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardIdentitySendRequest {
    pub guard_session_id: String,
    pub preflight_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardIdentityLookupRequest {
    pub guard_session_id: String,
    pub target_identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardIdentityLookupResult {
    pub exists: bool,
}

pub type GuardPreflightResult = IdentityPreflightResult;
pub type GuardSendResult = IdentitySendResult;
