//
// Guard session and signed-out identity flow types.

use serde::{Deserialize, Serialize};

use crate::types::identity::{
    IdentityPreflightParams, IdentityPreflightResult, IdentitySendResult,
};
use crate::types::wallet::WalletNetwork;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginGuardSessionRequest {
    pub import_text: String,
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

pub type GuardPreflightResult = IdentityPreflightResult;
pub type GuardSendResult = IdentitySendResult;
