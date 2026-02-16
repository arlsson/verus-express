//
// Module 7: Update engine — Tokio polling for balances and transactions, Tauri event emission.
// Lifecycle: start on unlock, stop on lock. No sensitive data in logs.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::core::auth::SessionManager;
use crate::core::channels::btc::BtcProviderPool;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::channels::{route_get_balances, route_get_transactions};
use crate::core::coins::Channel;
use crate::core::coins::CoinRegistry;
use crate::core::rates::{build_rates_http_client, coinpaprika, ecb, pbaas};
use crate::core::updates::events::{
    BalancesUpdatedPayload, BootstrapUpdatedPayload, RatesUpdatedPayload,
    TransactionsUpdatedPayload, UpdateErrorPayload,
};
use crate::core::updates::params::{
    jitter_duration, BALANCE_REFRESH_SECS, RATES_REFRESH_SECS, TRANSACTION_REFRESH_SECS,
};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

/// Tauri event names (frontend listens via listen()).
pub const EVENT_BALANCES_UPDATED: &str = "wallet://balances-updated";
pub const EVENT_TRANSACTIONS_UPDATED: &str = "wallet://transactions-updated";
pub const EVENT_RATES_UPDATED: &str = "wallet://rates-updated";
pub const EVENT_BOOTSTRAP_UPDATED: &str = "wallet://bootstrap-updated";
pub const EVENT_ERROR: &str = "wallet://error";
const BOOTSTRAP_BALANCE_CONCURRENCY: usize = 4;
const BOOTSTRAP_RATE_CONCURRENCY: usize = 3;
const VRSC_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
const VRSCTEST_SYSTEM_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";
const VRSC_COIN_ID: &str = "VRSC";
const VRSCTEST_COIN_ID: &str = "VRSCTEST";

#[derive(Clone, Debug, Default)]
pub struct UpdateEngineStartConfig {
    pub poll_transactions: bool,
    pub priority_coin_ids: Vec<String>,
    pub priority_channel_ids: Vec<String>,
}

/// Per-channel expiry state for balance and transaction data.
#[derive(Default)]
struct ChannelState {
    last_balance_fetch: Option<Instant>,
    last_tx_fetch: Option<Instant>,
}

/// Update engine: polls VRPC, BTC, ETH and ERC20 channels when unlocked, emits Tauri events.
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

    /// Start polling. Call after successful unlock. Spawns a single task that always runs
    /// balance polling and can optionally run transaction polling.
    pub async fn start(
        &self,
        app_handle: AppHandle,
        session_manager: Arc<Mutex<SessionManager>>,
        coin_registry: Arc<CoinRegistry>,
        vrpc_provider_pool: Arc<VrpcProviderPool>,
        btc_provider_pool: Arc<BtcProviderPool>,
        eth_provider_pool: Arc<EthProviderPool>,
        start_config: UpdateEngineStartConfig,
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
                eth_provider_pool,
                start_config,
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

/// Build list of (coin_id, channel_id) for active channels.
fn active_channels(
    coin_registry: &CoinRegistry,
    is_testnet: bool,
    vrpc_address: &str,
    eth_enabled: bool,
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
                Channel::Eth if eth_enabled => {
                    out.push((c.id.clone(), format!("eth.{}", c.id)));
                }
                Channel::Erc20 if eth_enabled => {
                    out.push((c.id.clone(), format!("erc20.{}", c.id)));
                }
                _ => {}
            }
        }
    }
    out
}

fn normalize_priority_entries(entries: &[String]) -> HashSet<String> {
    entries
        .iter()
        .map(|entry| entry.trim().to_ascii_lowercase())
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn dedupe_channel_pairs(channels: &[(String, String)]) -> Vec<(String, String)> {
    let mut seen = HashSet::<String>::new();
    let mut deduped = Vec::<(String, String)>::new();

    for (coin_id, channel_id) in channels {
        let dedupe_key = format!(
            "{}::{}",
            channel_id.trim().to_ascii_lowercase(),
            coin_id.trim().to_ascii_lowercase()
        );
        if seen.insert(dedupe_key) {
            deduped.push((coin_id.clone(), channel_id.clone()));
        }
    }

    deduped
}

fn partition_bootstrap_channels(
    channels: &[(String, String)],
    priority_coin_ids: &HashSet<String>,
    priority_channel_ids: &HashSet<String>,
) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let mut prioritized = Vec::<(String, String)>::new();
    let mut remainder = Vec::<(String, String)>::new();

    for (coin_id, channel_id) in channels {
        let coin_key = coin_id.trim().to_ascii_lowercase();
        let channel_key = channel_id.trim().to_ascii_lowercase();
        if priority_coin_ids.contains(&coin_key) || priority_channel_ids.contains(&channel_key) {
            prioritized.push((coin_id.clone(), channel_id.clone()));
        } else {
            remainder.push((coin_id.clone(), channel_id.clone()));
        }
    }

    (prioritized, remainder)
}

fn prioritized_rate_coins(
    rate_coins: &[crate::core::coins::CoinDefinition],
    prioritized_channels: &[(String, String)],
    priority_coin_ids: &HashSet<String>,
) -> Vec<crate::core::coins::CoinDefinition> {
    let mut prioritized_coin_keys = priority_coin_ids.clone();
    for (coin_id, _) in prioritized_channels {
        prioritized_coin_keys.insert(coin_id.trim().to_ascii_lowercase());
    }

    // Ensure PBaaS anchor assets (VRSC/VRSCTEST) are included in bootstrap priority
    // whenever a PBaaS-derived coin is prioritized.
    let mut anchor_coin_keys = HashSet::<String>::new();
    for coin in rate_coins {
        let coin_key = coin.id.trim().to_ascii_lowercase();
        if !prioritized_coin_keys.contains(&coin_key) || !pbaas::is_pbaas_derivation_candidate(coin)
        {
            continue;
        }

        if let Some(anchor_coin_id) = anchor_coin_id_for_system_id(&coin.system_id) {
            anchor_coin_keys.insert(anchor_coin_id.to_ascii_lowercase());
        }
    }
    prioritized_coin_keys.extend(anchor_coin_keys);

    let mut seen = HashSet::<String>::new();
    let mut prioritized = Vec::<crate::core::coins::CoinDefinition>::new();
    for coin in rate_coins {
        let coin_key = coin.id.trim().to_ascii_lowercase();
        if prioritized_coin_keys.contains(&coin_key) && seen.insert(coin_key) {
            prioritized.push(coin.clone());
        }
    }

    prioritized
}

fn anchor_coin_id_for_system_id(system_id: &str) -> Option<&'static str> {
    match system_id {
        VRSC_SYSTEM_ID => Some(VRSC_COIN_ID),
        VRSCTEST_SYSTEM_ID => Some(VRSCTEST_COIN_ID),
        _ => None,
    }
}

fn emit_bootstrap_updated(app_handle: &AppHandle, in_progress: bool) {
    let payload = BootstrapUpdatedPayload { in_progress };
    if let Err(err) = app_handle.emit(EVENT_BOOTSTRAP_UPDATED, &payload) {
        println!("[UPDATE] Emit bootstrap-updated failed: {:?}", err);
    }
}

async fn run_bootstrap_balance_fetches(
    app_handle: &AppHandle,
    cancel_token: &CancellationToken,
    prioritized_channels: &[(String, String)],
    session_manager: Arc<Mutex<SessionManager>>,
    coin_registry: Arc<CoinRegistry>,
    vrpc_provider_pool: Arc<VrpcProviderPool>,
    btc_provider_pool: Arc<BtcProviderPool>,
    eth_provider_pool: Arc<EthProviderPool>,
    channel_state: &mut HashMap<String, ChannelState>,
) {
    if prioritized_channels.is_empty() {
        return;
    }

    let semaphore = Arc::new(Semaphore::new(BOOTSTRAP_BALANCE_CONCURRENCY));
    let mut join_set = JoinSet::new();

    for (coin_id, channel_id) in prioritized_channels {
        if cancel_token.is_cancelled() {
            return;
        }

        let permit = match semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => return,
        };

        let coin_id = coin_id.clone();
        let channel_id = channel_id.clone();
        let session_manager = Arc::clone(&session_manager);
        let coin_registry = Arc::clone(&coin_registry);
        let vrpc_provider_pool = Arc::clone(&vrpc_provider_pool);
        let btc_provider_pool = Arc::clone(&btc_provider_pool);
        let eth_provider_pool = Arc::clone(&eth_provider_pool);

        join_set.spawn(async move {
            let _permit = permit;
            let result = route_get_balances(
                &channel_id,
                Some(coin_id.as_str()),
                &session_manager,
                coin_registry.as_ref(),
                vrpc_provider_pool.as_ref(),
                btc_provider_pool.as_ref(),
                eth_provider_pool.as_ref(),
            )
            .await;
            (coin_id, channel_id, result)
        });
    }

    while let Some(task_result) = join_set.join_next().await {
        let (coin_id, channel_id, result) = match task_result {
            Ok(value) => value,
            Err(err) => {
                println!("[UPDATE] Bootstrap balance task join error: {}", err);
                continue;
            }
        };

        match result {
            Ok(balance) => {
                let payload = BalancesUpdatedPayload {
                    coin_id: coin_id.clone(),
                    channel: channel_id.clone(),
                    confirmed: balance.confirmed,
                    pending: balance.pending,
                    total: balance.total,
                };
                if let Err(err) = app_handle.emit(EVENT_BALANCES_UPDATED, &payload) {
                    println!("[UPDATE] Emit balances-updated failed: {:?}", err);
                }
                channel_state
                    .entry(format!("{}::{}", channel_id, coin_id))
                    .or_default()
                    .last_balance_fetch = Some(Instant::now());
            }
            Err(err) => {
                let message = user_facing_error(&err);
                let _ = app_handle.emit(
                    EVENT_ERROR,
                    &UpdateErrorPayload {
                        data_type: "balance".to_string(),
                        coin_id,
                        channel: channel_id,
                        message,
                    },
                );
            }
        }
    }
}

async fn run_bootstrap_rate_fetches(
    app_handle: &AppHandle,
    cancel_token: &CancellationToken,
    prioritized_coins: &[crate::core::coins::CoinDefinition],
    active_network: WalletNetwork,
    rates_http_client: reqwest::Client,
    vrpc_provider_pool: Arc<VrpcProviderPool>,
    latest_rates: &mut HashMap<String, HashMap<String, f64>>,
    coin_rates_state: &mut HashMap<String, Instant>,
) {
    if prioritized_coins.is_empty() {
        return;
    }

    let usd_reference_rates = match ecb::fetch_usd_reference_rates(&rates_http_client).await {
        Ok(rates) => rates,
        Err(err) => {
            println!("[UPDATE] ECB rates unavailable during bootstrap: {}", err);
            HashMap::from([(ecb::USD.to_string(), 1.0)])
        }
    };

    let semaphore = Arc::new(Semaphore::new(BOOTSTRAP_RATE_CONCURRENCY));
    let mut direct_join_set = JoinSet::new();

    for coin in prioritized_coins {
        if cancel_token.is_cancelled() {
            return;
        }

        let permit = match semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => return,
        };
        let coin = coin.clone();
        let rates_http_client = rates_http_client.clone();
        let usd_reference_rates = usd_reference_rates.clone();
        direct_join_set.spawn(async move {
            let _permit = permit;

            let mut resolved_rates: Option<HashMap<String, f64>> = None;
            let mut usd_change_24h_pct: Option<f64> = None;
            let mut rate_error: Option<String> = None;

            match coinpaprika::fetch_usd_metrics(&rates_http_client, &coin).await {
                Ok(metrics) => {
                    let rates = ecb::build_coin_fiat_rates(metrics.usd_price, &usd_reference_rates);
                    if !rates.is_empty() {
                        usd_change_24h_pct = metrics.usd_change_24h_pct;
                        resolved_rates = Some(rates);
                    }
                }
                Err(err) => {
                    rate_error = Some(err);
                }
            }

            (coin, resolved_rates, usd_change_24h_pct, rate_error)
        });
    }

    let mut unresolved_pbaas = Vec::<(crate::core::coins::CoinDefinition, Option<String>)>::new();

    while let Some(task_result) = direct_join_set.join_next().await {
        let (coin, resolved_rates, usd_change_24h_pct, rate_error) = match task_result {
            Ok(value) => value,
            Err(err) => {
                println!("[UPDATE] Bootstrap direct rate task join error: {}", err);
                continue;
            }
        };
        let coin_id = coin.id.clone();

        if let Some(rates) = resolved_rates {
            let payload = RatesUpdatedPayload {
                coin_id: coin_id.clone(),
                rates: rates.clone(),
                usd_change_24h_pct,
            };
            if let Err(err) = app_handle.emit(EVENT_RATES_UPDATED, &payload) {
                println!("[UPDATE] Emit rates-updated failed: {:?}", err);
            } else {
                latest_rates.insert(coin_id.clone(), rates);
                // Avoid rate loop re-fetching this coin in the same cycle.
                coin_rates_state.insert(coin_id, Instant::now());
            }
            continue;
        }

        if pbaas::is_pbaas_derivation_candidate(&coin) {
            unresolved_pbaas.push((coin, rate_error));
        } else if let Some(rate_error) = rate_error {
            println!(
                "[UPDATE] Fiat rate unavailable during bootstrap for {}: {}",
                coin_id, rate_error
            );
        }
    }

    if unresolved_pbaas.is_empty() {
        return;
    }

    println!(
        "[UPDATE] Bootstrap PBaaS derivation retry candidates={}",
        unresolved_pbaas.len()
    );

    let latest_rates_snapshot = latest_rates.clone();
    let semaphore = Arc::new(Semaphore::new(BOOTSTRAP_RATE_CONCURRENCY));
    let mut derive_join_set = JoinSet::new();

    for (coin, direct_error) in unresolved_pbaas {
        if cancel_token.is_cancelled() {
            return;
        }

        let permit = match semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => return,
        };
        let vrpc_provider_pool = Arc::clone(&vrpc_provider_pool);
        let latest_rates_snapshot = latest_rates_snapshot.clone();

        derive_join_set.spawn(async move {
            let _permit = permit;
            let coin_id = coin.id.clone();
            let resolved_rates = pbaas::derive_pbaas_rates(
                vrpc_provider_pool.for_network(active_network),
                &coin,
                &latest_rates_snapshot,
            )
            .await;

            (coin_id, resolved_rates, direct_error)
        });
    }

    while let Some(task_result) = derive_join_set.join_next().await {
        let (coin_id, resolved_rates, direct_error) = match task_result {
            Ok(value) => value,
            Err(err) => {
                println!("[UPDATE] Bootstrap PBaaS derive task join error: {}", err);
                continue;
            }
        };

        if let Some(rates) = resolved_rates {
            let payload = RatesUpdatedPayload {
                coin_id: coin_id.clone(),
                rates: rates.clone(),
                usd_change_24h_pct: None,
            };
            if let Err(err) = app_handle.emit(EVENT_RATES_UPDATED, &payload) {
                println!("[UPDATE] Emit rates-updated failed: {:?}", err);
            } else {
                latest_rates.insert(coin_id.clone(), rates);
                coin_rates_state.insert(coin_id, Instant::now());
            }
            continue;
        }

        if let Some(rate_error) = direct_error {
            println!(
                "[UPDATE] Fiat rate unavailable during bootstrap for {}: {}",
                coin_id, rate_error
            );
        } else {
            println!(
                "[UPDATE] Fiat rate unavailable during bootstrap for {}",
                coin_id
            );
        }
    }
}

async fn run_update_loop(
    cancel_token: CancellationToken,
    app_handle: AppHandle,
    session_manager: Arc<Mutex<SessionManager>>,
    coin_registry: Arc<CoinRegistry>,
    vrpc_provider_pool: Arc<VrpcProviderPool>,
    btc_provider_pool: Arc<BtcProviderPool>,
    eth_provider_pool: Arc<EthProviderPool>,
    start_config: UpdateEngineStartConfig,
) {
    let mut channel_state: HashMap<String, ChannelState> = HashMap::new();
    let mut coin_rates_state: HashMap<String, Instant> = HashMap::new();
    let mut latest_rates: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let rates_http_client = build_rates_http_client();
    let poll_transactions = start_config.poll_transactions;
    let priority_coin_ids = normalize_priority_entries(&start_config.priority_coin_ids);
    let priority_channel_ids = normalize_priority_entries(&start_config.priority_channel_ids);
    let mut bootstrap_completed = false;
    emit_bootstrap_updated(&app_handle, true);

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
        let active_network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
        let is_testnet = matches!(active_network, WalletNetwork::Testnet);
        drop(session);

        let raw_channels = active_channels(
            &coin_registry,
            is_testnet,
            &session_vrpc_address,
            eth_provider_pool.is_enabled(),
        );
        let channels = dedupe_channel_pairs(&raw_channels);
        if channels.is_empty() {
            if !bootstrap_completed {
                emit_bootstrap_updated(&app_handle, false);
                bootstrap_completed = true;
            }
            tokio::time::sleep(jitter_duration(30)).await;
            continue;
        }

        if !bootstrap_completed {
            let bootstrap_started_at = Instant::now();
            let (prioritized_channels, remainder_channels) =
                partition_bootstrap_channels(&channels, &priority_coin_ids, &priority_channel_ids);
            println!(
                "[UPDATE] Bootstrap start: prioritized_channels={} remaining_channels={}",
                prioritized_channels.len(),
                remainder_channels.len()
            );

            let balance_bootstrap_started_at = Instant::now();
            run_bootstrap_balance_fetches(
                &app_handle,
                &cancel_token,
                &prioritized_channels,
                Arc::clone(&session_manager),
                Arc::clone(&coin_registry),
                Arc::clone(&vrpc_provider_pool),
                Arc::clone(&btc_provider_pool),
                Arc::clone(&eth_provider_pool),
                &mut channel_state,
            )
            .await;
            let balance_bootstrap_elapsed = balance_bootstrap_started_at.elapsed();

            let rate_coins = coin_registry
                .get_all()
                .into_iter()
                .filter(|coin| coin.is_testnet == is_testnet)
                .collect::<Vec<_>>();
            let prioritized_coins =
                prioritized_rate_coins(&rate_coins, &prioritized_channels, &priority_coin_ids);

            let rate_bootstrap_started_at = Instant::now();
            run_bootstrap_rate_fetches(
                &app_handle,
                &cancel_token,
                &prioritized_coins,
                active_network,
                rates_http_client.clone(),
                Arc::clone(&vrpc_provider_pool),
                &mut latest_rates,
                &mut coin_rates_state,
            )
            .await;
            let rate_bootstrap_elapsed = rate_bootstrap_started_at.elapsed();
            let bootstrap_elapsed = bootstrap_started_at.elapsed();

            println!(
                "[UPDATE] Bootstrap complete: prioritized_channels={} prioritized_rates={} balance_ms={} rate_ms={} total_ms={}",
                prioritized_channels.len(),
                prioritized_coins.len(),
                balance_bootstrap_elapsed.as_millis(),
                rate_bootstrap_elapsed.as_millis(),
                bootstrap_elapsed.as_millis()
            );

            emit_bootstrap_updated(&app_handle, false);
            bootstrap_completed = true;
        }

        let now = Instant::now();

        for (coin_id, channel_id) in &channels {
            if cancel_token.is_cancelled() {
                return;
            }
            let channel_state_key = format!("{}::{}", channel_id, coin_id);

            let needs_balance = {
                let state = channel_state.entry(channel_state_key.clone()).or_default();
                state.last_balance_fetch.map_or(true, |t| {
                    now.duration_since(t).as_secs() >= BALANCE_REFRESH_SECS
                })
            };

            if needs_balance {
                match route_get_balances(
                    channel_id,
                    Some(coin_id.as_str()),
                    &session_manager,
                    coin_registry.as_ref(),
                    vrpc_provider_pool.as_ref(),
                    btc_provider_pool.as_ref(),
                    eth_provider_pool.as_ref(),
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
                            .entry(channel_state_key.clone())
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

        if poll_transactions {
            for (coin_id, channel_id) in &channels {
                if cancel_token.is_cancelled() {
                    return;
                }
                let channel_state_key = format!("{}::{}", channel_id, coin_id);

                let state = channel_state.entry(channel_state_key.clone()).or_default();
                let needs_tx = state.last_tx_fetch.map_or(true, |t| {
                    now.duration_since(t).as_secs() >= TRANSACTION_REFRESH_SECS
                });

                if needs_tx {
                    match route_get_transactions(
                        channel_id,
                        Some(coin_id.as_str()),
                        &session_manager,
                        coin_registry.as_ref(),
                        vrpc_provider_pool.as_ref(),
                        btc_provider_pool.as_ref(),
                        eth_provider_pool.as_ref(),
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
        }

        let rate_coins = coin_registry
            .get_all()
            .into_iter()
            .filter(|coin| coin.is_testnet == is_testnet)
            .collect::<Vec<_>>();

        let needs_any_rates = rate_coins.iter().any(|coin| {
            coin_rates_state.get(&coin.id).map_or(true, |t| {
                now.duration_since(*t).as_secs() >= RATES_REFRESH_SECS
            })
        });

        if needs_any_rates {
            let usd_reference_rates = match ecb::fetch_usd_reference_rates(&rates_http_client).await
            {
                Ok(rates) => rates,
                Err(err) => {
                    println!("[UPDATE] ECB rates unavailable: {}", err);
                    HashMap::from([(ecb::USD.to_string(), 1.0)])
                }
            };

            for coin in &rate_coins {
                if cancel_token.is_cancelled() {
                    return;
                }

                let needs_rates = coin_rates_state.get(&coin.id).map_or(true, |t| {
                    now.duration_since(*t).as_secs() >= RATES_REFRESH_SECS
                });
                if !needs_rates {
                    continue;
                }

                let mut resolved_rates: Option<HashMap<String, f64>> = None;
                let mut usd_change_24h_pct: Option<f64> = None;

                match coinpaprika::fetch_usd_metrics(&rates_http_client, coin).await {
                    Ok(metrics) => {
                        let rates =
                            ecb::build_coin_fiat_rates(metrics.usd_price, &usd_reference_rates);
                        if !rates.is_empty() {
                            usd_change_24h_pct = metrics.usd_change_24h_pct;
                            resolved_rates = Some(rates);
                        }
                    }
                    Err(err) => {
                        if pbaas::is_pbaas_derivation_candidate(coin) {
                            resolved_rates = pbaas::derive_pbaas_rates(
                                vrpc_provider_pool.for_network(active_network),
                                coin,
                                &latest_rates,
                            )
                            .await;
                        }

                        if resolved_rates.is_none() {
                            println!("[UPDATE] Fiat rate unavailable for {}: {}", coin.id, err);
                        }
                    }
                }

                if let Some(rates) = resolved_rates {
                    let payload = RatesUpdatedPayload {
                        coin_id: coin.id.clone(),
                        rates: rates.clone(),
                        usd_change_24h_pct,
                    };
                    if let Err(e) = app_handle.emit(EVENT_RATES_UPDATED, &payload) {
                        println!("[UPDATE] Emit rates-updated failed: {:?}", e);
                    } else {
                        latest_rates.insert(coin.id.clone(), rates);
                    }
                }

                // Throttle retries after both success and failure.
                coin_rates_state.insert(coin.id.clone(), Instant::now());
                tokio::time::sleep(jitter_duration(1)).await;
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
        WalletError::EthNotConfigured => "Ethereum channels are not configured".to_string(),
        WalletError::OperationFailed => "Temporarily unavailable".to_string(),
        _ => "Temporarily unavailable".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        active_channels, dedupe_channel_pairs, partition_bootstrap_channels, prioritized_rate_coins,
    };
    use crate::core::coins::{Channel, CoinDefinition, CoinRegistry, Protocol};
    use std::collections::HashSet;

    #[test]
    fn active_channels_includes_eth_and_erc20_when_eth_enabled() {
        let registry = CoinRegistry::new();
        let channels = active_channels(&registry, false, "RtestAddress", true);

        assert!(channels
            .iter()
            .any(|(coin_id, channel_id)| coin_id == "ETH" && channel_id == "eth.ETH"));
        assert!(channels
            .iter()
            .any(|(coin_id, channel_id)| coin_id == "USDC" && channel_id == "erc20.USDC"));
    }

    #[test]
    fn active_channels_omits_eth_and_erc20_when_eth_disabled() {
        let registry = CoinRegistry::new();
        let channels = active_channels(&registry, false, "RtestAddress", false);

        assert!(!channels
            .iter()
            .any(|(_, channel_id)| channel_id.starts_with("eth.")));
        assert!(!channels
            .iter()
            .any(|(_, channel_id)| channel_id.starts_with("erc20.")));
    }

    #[test]
    fn active_channels_respects_testnet_network() {
        let registry = CoinRegistry::new();
        let channels = active_channels(&registry, true, "RtestAddress", true);

        assert!(channels
            .iter()
            .any(|(coin_id, channel_id)| coin_id == "GETH" && channel_id == "eth.GETH"));
        assert!(!channels
            .iter()
            .any(|(coin_id, _)| coin_id == "ETH" || coin_id == "USDC"));
    }

    #[test]
    fn dedupe_channel_pairs_preserves_first_seen_order() {
        let channels = vec![
            ("VRSC".to_string(), "vrpc.Raddr.iSystem".to_string()),
            ("VRSC".to_string(), "vrpc.Raddr.iSystem".to_string()),
            ("BTC".to_string(), "btc.BTC".to_string()),
            ("btc".to_string(), "BTC.btc".to_string()),
        ];

        let deduped = dedupe_channel_pairs(&channels);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].0, "VRSC");
        assert_eq!(deduped[1].0, "BTC");
    }

    #[test]
    fn partition_bootstrap_channels_keeps_priority_order() {
        let channels = vec![
            ("VRSC".to_string(), "vrpc.Raddr.iSystem".to_string()),
            ("BTC".to_string(), "btc.BTC".to_string()),
            ("ETH".to_string(), "eth.ETH".to_string()),
        ];
        let priority_coin_ids = HashSet::from(["btc".to_string()]);
        let priority_channel_ids = HashSet::from(["eth.eth".to_string()]);

        let (prioritized, remainder) =
            partition_bootstrap_channels(&channels, &priority_coin_ids, &priority_channel_ids);

        assert_eq!(prioritized.len(), 2);
        assert_eq!(prioritized[0].0, "BTC");
        assert_eq!(prioritized[1].0, "ETH");
        assert_eq!(remainder.len(), 1);
        assert_eq!(remainder[0].0, "VRSC");
    }

    #[test]
    fn partition_bootstrap_channels_without_priorities_falls_back_to_remainder() {
        let channels = vec![
            ("VRSC".to_string(), "vrpc.Raddr.iSystem".to_string()),
            ("BTC".to_string(), "btc.BTC".to_string()),
        ];
        let priority_coin_ids = HashSet::new();
        let priority_channel_ids = HashSet::new();

        let (prioritized, remainder) =
            partition_bootstrap_channels(&channels, &priority_coin_ids, &priority_channel_ids);

        assert!(prioritized.is_empty());
        assert_eq!(remainder, channels);
    }

    #[test]
    fn prioritized_rate_coins_adds_vrsc_anchor_for_prioritized_pbaas() {
        let vrsc = CoinDefinition {
            id: "VRSC".to_string(),
            currency_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "VRSC".to_string(),
            display_name: "Verus".to_string(),
            coin_paprika_id: Some("vrsc-verus-coin".to_string()),
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![],
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        };
        let pure = CoinDefinition {
            id: "iHax5qYQGbcMGqJKKrPorpzUBX2oFFXGnY".to_string(),
            currency_id: "iHax5qYQGbcMGqJKKrPorpzUBX2oFFXGnY".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "Pure".to_string(),
            display_name: "Pure".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![],
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        };
        let rate_coins = vec![vrsc.clone(), pure.clone()];
        let prioritized_channels = vec![(pure.id.clone(), "vrpc.Raddr.iSystem".to_string())];

        let prioritized =
            prioritized_rate_coins(&rate_coins, &prioritized_channels, &HashSet::new());
        let prioritized_ids = prioritized
            .into_iter()
            .map(|coin| coin.id)
            .collect::<HashSet<_>>();

        assert!(prioritized_ids.contains(&pure.id));
        assert!(prioritized_ids.contains(&vrsc.id));
    }
}
