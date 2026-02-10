//
// Module 7: Update engine — Tokio polling for balances and transactions, Tauri event emission.
// Lifecycle: start on unlock, stop on lock. No sensitive data in logs.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::core::auth::SessionManager;
use crate::core::channels::btc::BtcProviderPool;
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::channels::{route_get_balances, route_get_transactions};
use crate::core::coins::Channel;
use crate::core::coins::CoinRegistry;
use crate::core::updates::events::{
    BalancesUpdatedPayload, TransactionsUpdatedPayload, UpdateErrorPayload,
};
use crate::core::updates::params::{
    jitter_duration, BALANCE_REFRESH_SECS, TRANSACTION_REFRESH_SECS,
};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

/// Tauri event names (frontend listens via listen()).
pub const EVENT_BALANCES_UPDATED: &str = "wallet://balances-updated";
pub const EVENT_TRANSACTIONS_UPDATED: &str = "wallet://transactions-updated";
pub const EVENT_ERROR: &str = "wallet://error";

/// Per-channel expiry state for balance and transaction data.
#[derive(Default)]
struct ChannelState {
    last_balance_fetch: Option<Instant>,
    last_tx_fetch: Option<Instant>,
}

/// Update engine: polls VRPC and BTC channels when unlocked, emits Tauri events.
/// Hold in tauri::State; start() from start_update_engine, stop() from lock_wallet.
pub struct UpdateEngine {
    cancel_token: Mutex<Option<CancellationToken>>,
    task_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    channel_state: Mutex<HashMap<String, ChannelState>>,
}

impl UpdateEngine {
    pub fn new() -> Self {
        Self {
            cancel_token: Mutex::new(None),
            task_handle: Mutex::new(None),
            channel_state: Mutex::new(HashMap::new()),
        }
    }

    /// Start polling. Call after successful unlock. Spawns a single task that runs
    /// balance and transaction passes with jitter; stops when cancel token is triggered.
    pub async fn start(
        &self,
        app_handle: AppHandle,
        session_manager: Arc<Mutex<SessionManager>>,
        coin_registry: Arc<CoinRegistry>,
        vrpc_provider_pool: Arc<VrpcProviderPool>,
        btc_provider_pool: Arc<BtcProviderPool>,
    ) {
        self.stop().await;

        let token = CancellationToken::new();
        let child = token.child_token();

        let session_manager = Arc::clone(&session_manager);
        let task_handle = tokio::spawn(async move {
            run_update_loop(
                child,
                app_handle,
                session_manager,
                coin_registry,
                vrpc_provider_pool,
                btc_provider_pool,
            )
            .await;
        });

        *self.cancel_token.lock().await = Some(token);
        *self.task_handle.lock().await = Some(task_handle);
        println!("[UPDATE] Engine started");
    }

    /// Stop polling and wait for the task to finish. Call before lock.
    pub async fn stop(&self) {
        let mut token_guard = self.cancel_token.lock().await;
        if let Some(token) = token_guard.take() {
            token.cancel();
            drop(token_guard);
            let mut handle_guard = self.task_handle.lock().await;
            if let Some(handle) = handle_guard.take() {
                // Abort to avoid waiting on long in-flight network requests.
                handle.abort();
                let _ = handle.await;
            }
            println!("[UPDATE] Engine stopped");
        }
    }
}

/// Build list of (coin_id, channel_id) for coins that support Vrpc or Btc.
fn active_channels(
    coin_registry: &CoinRegistry,
    is_testnet: bool,
    vrpc_address: &str,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for c in coin_registry.get_all() {
        if c.is_testnet != is_testnet {
            continue;
        }
        for ch in &c.compatible_channels {
            match ch {
                Channel::Vrpc => {
                    out.push((
                        c.id.clone(),
                        format!("vrpc.{}.{}", vrpc_address, c.system_id),
                    ));
                }
                Channel::Btc => {
                    out.push((c.id.clone(), format!("btc.{}", c.id)));
                }
                _ => {}
            }
        }
    }
    out
}

async fn run_update_loop(
    cancel_token: CancellationToken,
    app_handle: AppHandle,
    session_manager: Arc<Mutex<SessionManager>>,
    coin_registry: Arc<CoinRegistry>,
    vrpc_provider_pool: Arc<VrpcProviderPool>,
    btc_provider_pool: Arc<BtcProviderPool>,
) {
    let mut channel_state: HashMap<String, ChannelState> = HashMap::new();

    loop {
        if cancel_token.is_cancelled() {
            break;
        }

        let session = session_manager.lock().await;
        if !session.is_unlocked() {
            drop(session);
            tokio::select! {
                _ = cancel_token.cancelled() => break,
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {}
            }
            continue;
        }
        let (session_vrpc_address, _, _) = match session.get_addresses() {
            Ok(v) => v,
            Err(_) => {
                drop(session);
                tokio::select! {
                    _ = cancel_token.cancelled() => break,
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {}
                }
                continue;
            }
        };
        let is_testnet = matches!(session.active_network(), Some(WalletNetwork::Testnet));
        drop(session);

        let channels = active_channels(&coin_registry, is_testnet, &session_vrpc_address);
        if channels.is_empty() {
            tokio::time::sleep(jitter_duration(30)).await;
            continue;
        }

        let now = Instant::now();

        for (coin_id, channel_id) in &channels {
            if cancel_token.is_cancelled() {
                return;
            }

            let needs_balance = {
                let state = channel_state.entry(channel_id.clone()).or_default();
                state.last_balance_fetch.map_or(true, |t| {
                    now.duration_since(t).as_secs() >= BALANCE_REFRESH_SECS
                })
            };

            if needs_balance {
                match route_get_balances(
                    channel_id,
                    &session_manager,
                    coin_registry.as_ref(),
                    vrpc_provider_pool.as_ref(),
                    btc_provider_pool.as_ref(),
                )
                .await
                {
                    Ok(bal) => {
                        let payload = BalancesUpdatedPayload {
                            coin_id: coin_id.clone(),
                            channel: channel_id.clone(),
                            confirmed: bal.confirmed,
                            pending: bal.pending,
                            total: bal.total,
                        };
                        if let Err(e) = app_handle.emit(EVENT_BALANCES_UPDATED, &payload) {
                            println!("[UPDATE] Emit balances-updated failed: {:?}", e);
                        }
                        channel_state
                            .entry(channel_id.clone())
                            .or_default()
                            .last_balance_fetch = Some(Instant::now());
                    }
                    Err(e) => {
                        let message = user_facing_error(&e);
                        let _ = app_handle.emit(
                            EVENT_ERROR,
                            &UpdateErrorPayload {
                                data_type: "balance".to_string(),
                                coin_id: coin_id.clone(),
                                channel: channel_id.clone(),
                                message,
                            },
                        );
                    }
                }
                tokio::time::sleep(jitter_duration(2)).await;
            }
        }

        for (coin_id, channel_id) in &channels {
            if cancel_token.is_cancelled() {
                return;
            }

            let state = channel_state.entry(channel_id.clone()).or_default();
            let needs_tx = state.last_tx_fetch.map_or(true, |t| {
                now.duration_since(t).as_secs() >= TRANSACTION_REFRESH_SECS
            });

            if needs_tx {
                match route_get_transactions(
                    channel_id,
                    &session_manager,
                    coin_registry.as_ref(),
                    vrpc_provider_pool.as_ref(),
                    btc_provider_pool.as_ref(),
                )
                .await
                {
                    Ok(txs) => {
                        let payload = TransactionsUpdatedPayload {
                            coin_id: coin_id.clone(),
                            channel: channel_id.clone(),
                            transactions: txs.transactions,
                        };
                        if let Err(e) = app_handle.emit(EVENT_TRANSACTIONS_UPDATED, &payload) {
                            println!("[UPDATE] Emit transactions-updated failed: {:?}", e);
                        }
                        if let Some(warning) = txs.warning {
                            let _ = app_handle.emit(
                                EVENT_ERROR,
                                &UpdateErrorPayload {
                                    data_type: "transactions_warning".to_string(),
                                    coin_id: coin_id.clone(),
                                    channel: channel_id.clone(),
                                    message: warning,
                                },
                            );
                        }
                        state.last_tx_fetch = Some(Instant::now());
                    }
                    Err(e) => {
                        let message = user_facing_error(&e);
                        let _ = app_handle.emit(
                            EVENT_ERROR,
                            &UpdateErrorPayload {
                                data_type: "transactions".to_string(),
                                coin_id: coin_id.clone(),
                                channel: channel_id.clone(),
                                message,
                            },
                        );
                    }
                }
                tokio::time::sleep(jitter_duration(2)).await;
            }
        }

        let sleep_secs = 60u64.min(BALANCE_REFRESH_SECS / 2);
        tokio::select! {
            _ = cancel_token.cancelled() => break,
            _ = tokio::time::sleep(jitter_duration(sleep_secs)) => {}
        }
    }
}

fn user_facing_error(e: &WalletError) -> String {
    match e {
        WalletError::WalletLocked => "Wallet is locked".to_string(),
        WalletError::UnsupportedChannel => "Unsupported channel".to_string(),
        WalletError::InvalidPreflight => "Invalid preflight".to_string(),
        WalletError::NetworkError => "Network error".to_string(),
        WalletError::OperationFailed => "Temporarily unavailable".to_string(),
        _ => "Temporarily unavailable".to_string(),
    }
}
