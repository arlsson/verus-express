//
// Module 4 + 5 + 5d + 9: Tauri commands for preflight, send, and data (balances, transaction history).
// Thin wrappers; VRPC and BTC channels wired. State<Arc<CoinRegistry>> and State<Arc<BtcProvider>> for Module 7.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::core::auth::SessionManager;
use crate::core::channels::btc::BtcProviderPool;
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::channels::{
    route_get_balances, route_get_transactions, route_preflight, route_send, PreflightStore,
};
use crate::core::coins::CoinRegistry;
use crate::core::updates::{UpdateErrorPayload, EVENT_ERROR};
use crate::types::{
    BalanceResult, PreflightParams, PreflightResult, SendRequest, SendResult, Transaction,
    WalletError,
};

/// Preflight a send by channel. Requires unlocked session. Returns PreflightResult with preflight_id (no tx hex).
#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_send(
    params: PreflightParams,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    btc_provider_pool: State<'_, Arc<BtcProviderPool>>,
) -> Result<PreflightResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    drop(session);

    println!("[TX] Preflight requested: channel_id={}", params.channel_id);
    let channel_id = params.channel_id.clone();
    route_preflight(
        &channel_id,
        params,
        &preflight_store,
        &session_manager,
        coin_registry.as_ref(),
        vrpc_provider_pool.inner().as_ref(),
        btc_provider_pool.inner().as_ref(),
    )
    .await
}

/// Send by preflight_id only. Requires unlocked session. Does not accept tx hex or signing data from UI.
#[tauri::command(rename_all = "snake_case")]
pub async fn send_transaction(
    request: SendRequest,
    preflight_store: State<'_, PreflightStore>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    btc_provider_pool: State<'_, Arc<BtcProviderPool>>,
) -> Result<SendResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    drop(session);

    println!("[TX] Send requested: preflight_id={}", request.preflight_id);
    route_send(
        &request.preflight_id,
        &preflight_store,
        &session_manager,
        vrpc_provider_pool.inner().as_ref(),
        btc_provider_pool.inner().as_ref(),
    )
    .await
}

/// Fetch balance for the given channel. Requires unlocked session. VRPC and BTC supported.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_balances(
    channel_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    btc_provider_pool: State<'_, Arc<BtcProviderPool>>,
) -> Result<BalanceResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    drop(session);

    route_get_balances(
        &channel_id,
        &session_manager,
        coin_registry.as_ref(),
        vrpc_provider_pool.inner().as_ref(),
        btc_provider_pool.inner().as_ref(),
    )
    .await
}

/// Fetch transaction history for the given channel. Requires unlocked session. VRPC and BTC supported.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_transaction_history(
    channel_id: String,
    app_handle: AppHandle,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    btc_provider_pool: State<'_, Arc<BtcProviderPool>>,
) -> Result<Vec<Transaction>, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    drop(session);

    let res = route_get_transactions(
        &channel_id,
        &session_manager,
        coin_registry.as_ref(),
        vrpc_provider_pool.inner().as_ref(),
        btc_provider_pool.inner().as_ref(),
    )
    .await?;
    if let Some(warning) = res.warning {
        let _ = app_handle.emit(
            EVENT_ERROR,
            &UpdateErrorPayload {
                data_type: "transactions_warning".to_string(),
                coin_id: String::new(),
                channel: channel_id.clone(),
                message: warning,
            },
        );
    }
    Ok(res.transactions)
}
