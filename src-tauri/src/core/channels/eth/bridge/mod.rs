//
// ETH bridge channel scaffolding.
// Phase-1 scope: typed boundaries and deterministic not-implemented behavior.

pub mod delegator;
pub mod fees;
pub mod paths;
pub mod preflight;
pub mod reserve_transfer;
pub mod send;
pub mod token_mapping;

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::core::auth::SessionManager;
use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::vrpc::VrpcProvider;
use crate::core::channels::PreflightStore;
use crate::types::bridge::{
    BridgeConversionPathRequest, BridgeConversionPathsResult, BridgeTransferPreflightParams,
    BridgeTransferPreflightResult,
};
use crate::types::transaction::SendResult;
use crate::types::WalletError;

pub async fn get_conversion_paths(
    request: &BridgeConversionPathRequest,
    vrpc_provider: &VrpcProvider,
) -> Result<BridgeConversionPathsResult, WalletError> {
    paths::get_conversion_paths(request, vrpc_provider).await
}

pub async fn preflight(
    params: BridgeTransferPreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    from_address: &str,
    channel_id: &str,
    provider: &EthNetworkProvider,
) -> Result<BridgeTransferPreflightResult, WalletError> {
    preflight::preflight(
        params,
        preflight_store,
        account_id,
        from_address,
        channel_id,
        provider,
    )
    .await
}

pub async fn send(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    provider_pool: &EthProviderPool,
) -> Result<SendResult, WalletError> {
    send::send(
        preflight_id,
        preflight_store,
        session_manager,
        provider_pool,
    )
    .await
}
