//
// Identity transaction commands (update/revoke/recover).
// Security: preflight stores backend-owned payload; send accepts preflight_id only.

use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::core::auth::SessionManager;
use crate::core::channels::vrpc::identity as vrpc_identity;
use crate::core::channels::vrpc::{self, VrpcProviderPool};
use crate::core::channels::PreflightStore;
use crate::core::coins::CoinRegistry;
use crate::types::wallet::WalletNetwork;
use crate::types::{
    IdentityPreflightParams, IdentityPreflightResult, IdentitySendRequest, IdentitySendResult,
    WalletError,
};

/// Preflight identity operation on VRPC channel.
#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_identity_update(
    params: IdentityPreflightParams,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<IdentityPreflightResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let account_id = session
        .active_account_id()
        .ok_or(WalletError::WalletLocked)?
        .to_string();
    let (session_vrpc_address, _, _) = session.get_addresses()?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    drop(session);

    let resolved = vrpc::parse_vrpc_channel_id(&params.channel_id, Some(&session_vrpc_address))?;
    if resolved.address != session_vrpc_address {
        return Err(WalletError::InvalidAddress);
    }

    let is_testnet = matches!(network, WalletNetwork::Testnet);
    if coin_registry
        .find_by_system_id(&resolved.system_id, is_testnet)
        .is_none()
    {
        return Err(WalletError::UnsupportedChannel);
    }

    let canonical_channel_id =
        vrpc::canonical_vrpc_channel_id(&resolved.address, &resolved.system_id);

    vrpc_identity::preflight(
        params,
        &preflight_store,
        &account_id,
        &resolved.address,
        &canonical_channel_id,
        vrpc_provider_pool.for_network(network),
    )
    .await
}

/// Broadcast identity operation by preflight id.
#[tauri::command(rename_all = "snake_case")]
pub async fn send_identity_update(
    request: IdentitySendRequest,
    preflight_store: State<'_, PreflightStore>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<IdentitySendResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    drop(session);

    vrpc_identity::send(
        &request.preflight_id,
        &preflight_store,
        &session_manager,
        vrpc_provider_pool.inner().as_ref(),
    )
    .await
}
