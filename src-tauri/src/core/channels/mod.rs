//
// Module 4 + 5 + 5d + 9: Channel trait and router. Dispatches preflight/send and balance/tx by channel_id; VRPC and BTC.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::core::auth::SessionManager;
use crate::core::channels::btc::BtcProviderPool;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::coins::Channel;
use crate::core::coins::{CoinDefinition, CoinRegistry};
use crate::types::transaction::{
    BalanceResult, PreflightParams, PreflightResult, SendResult, Transaction,
    TransactionHistoryPage,
};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

pub mod btc;
pub mod eth;
mod store;
pub mod vrpc;

pub use store::{PreflightRecord, PreflightStore};

const VRSC_MAINNET_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
const VRSC_TESTNET_SYSTEM_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";
const DEFAULT_TRANSACTION_HISTORY_PAGE_LIMIT: usize = 50;
const MAX_TRANSACTION_HISTORY_PAGE_LIMIT: usize = 100;

#[derive(Debug, Clone)]
pub struct TransactionsFetchResult {
    pub transactions: Vec<Transaction>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
enum TransactionHistoryCursor {
    Vrpc {
        end_block: u64,
        include_pending: bool,
    },
    Btc {
        last_seen_txid: Option<String>,
    },
    Eth {
        page: u32,
    },
    Erc20 {
        page: u32,
    },
}

#[derive(Debug, Clone)]
struct ProtocolPageResult {
    transactions: Vec<Transaction>,
    next_cursor: Option<TransactionHistoryCursor>,
    has_more: bool,
    warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VrpcCursorPayload {
    v: u8,
    k: String,
    end_block: u64,
    include_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BtcCursorPayload {
    v: u8,
    k: String,
    last_seen_txid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EthCursorPayload {
    v: u8,
    k: String,
    page: u32,
}

fn clamp_history_limit(limit: Option<u32>) -> usize {
    let requested = limit.unwrap_or(DEFAULT_TRANSACTION_HISTORY_PAGE_LIMIT as u32);
    requested.clamp(1, MAX_TRANSACTION_HISTORY_PAGE_LIMIT as u32) as usize
}

fn dedupe_transactions_by_txid(transactions: Vec<Transaction>) -> Vec<Transaction> {
    let mut seen = HashSet::<String>::new();
    let mut deduped = Vec::with_capacity(transactions.len());
    for tx in transactions {
        if seen.insert(tx.txid.clone()) {
            deduped.push(tx);
        }
    }
    deduped
}

fn encode_history_cursor(cursor: &TransactionHistoryCursor) -> Result<String, WalletError> {
    let encoded = match cursor {
        TransactionHistoryCursor::Vrpc {
            end_block,
            include_pending,
        } => {
            let payload = VrpcCursorPayload {
                v: 1,
                k: "vrpc".to_string(),
                end_block: *end_block,
                include_pending: *include_pending,
            };
            serde_json::to_vec(&payload)?
        }
        TransactionHistoryCursor::Btc { last_seen_txid } => {
            let payload = BtcCursorPayload {
                v: 1,
                k: "btc".to_string(),
                last_seen_txid: last_seen_txid.clone(),
            };
            serde_json::to_vec(&payload)?
        }
        TransactionHistoryCursor::Eth { page } => {
            let payload = EthCursorPayload {
                v: 1,
                k: "eth".to_string(),
                page: *page,
            };
            serde_json::to_vec(&payload)?
        }
        TransactionHistoryCursor::Erc20 { page } => {
            let payload = EthCursorPayload {
                v: 1,
                k: "erc20".to_string(),
                page: *page,
            };
            serde_json::to_vec(&payload)?
        }
    };

    Ok(URL_SAFE_NO_PAD.encode(encoded))
}

fn decode_history_cursor(cursor: &str) -> Result<TransactionHistoryCursor, WalletError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| WalletError::OperationFailed)?;
    let payload: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| WalletError::OperationFailed)?;
    let kind = payload
        .get("k")
        .and_then(|value| value.as_str())
        .ok_or(WalletError::OperationFailed)?;

    match kind {
        "vrpc" => {
            let parsed: VrpcCursorPayload =
                serde_json::from_value(payload).map_err(|_| WalletError::OperationFailed)?;
            if parsed.v != 1 {
                return Err(WalletError::OperationFailed);
            }
            Ok(TransactionHistoryCursor::Vrpc {
                end_block: parsed.end_block,
                include_pending: parsed.include_pending,
            })
        }
        "btc" => {
            let parsed: BtcCursorPayload =
                serde_json::from_value(payload).map_err(|_| WalletError::OperationFailed)?;
            if parsed.v != 1 {
                return Err(WalletError::OperationFailed);
            }
            Ok(TransactionHistoryCursor::Btc {
                last_seen_txid: parsed.last_seen_txid,
            })
        }
        "eth" => {
            let parsed: EthCursorPayload =
                serde_json::from_value(payload).map_err(|_| WalletError::OperationFailed)?;
            if parsed.v != 1 || parsed.page == 0 {
                return Err(WalletError::OperationFailed);
            }
            Ok(TransactionHistoryCursor::Eth { page: parsed.page })
        }
        "erc20" => {
            let parsed: EthCursorPayload =
                serde_json::from_value(payload).map_err(|_| WalletError::OperationFailed)?;
            if parsed.v != 1 || parsed.page == 0 {
                return Err(WalletError::OperationFailed);
            }
            Ok(TransactionHistoryCursor::Erc20 { page: parsed.page })
        }
        _ => Err(WalletError::OperationFailed),
    }
}

/// Channel contract: balance, history, preflight, and send by preflight_id only.
#[async_trait]
pub trait WalletChannel: Send + Sync {
    async fn get_balances(&self, addresses: &[String]) -> Result<BalanceResult, WalletError>;
    async fn get_transactions(&self, addresses: &[String])
        -> Result<Vec<Transaction>, WalletError>;
    async fn preflight_send(&self, params: PreflightParams)
        -> Result<PreflightResult, WalletError>;

    /// Executes a previously validated preflight by handle. Must not sign UI-supplied tx hex.
    async fn send(&self, preflight_id: &str) -> Result<SendResult, WalletError>;
}

fn network_root_system_id(network: WalletNetwork) -> &'static str {
    if matches!(network, WalletNetwork::Testnet) {
        VRSC_TESTNET_SYSTEM_ID
    } else {
        VRSC_MAINNET_SYSTEM_ID
    }
}

fn coin_supports_vrpc(coin: &CoinDefinition) -> bool {
    coin.compatible_channels
        .iter()
        .any(|ch| matches!(ch, Channel::Vrpc))
}

fn is_native_vrpc_system_coin(coin: &CoinDefinition) -> bool {
    coin_supports_vrpc(coin) && coin.currency_id.eq_ignore_ascii_case(&coin.system_id)
}

fn is_allowed_vrpc_scope_system(
    coin_registry: &CoinRegistry,
    system_id: &str,
    network: WalletNetwork,
) -> bool {
    if system_id.eq_ignore_ascii_case(network_root_system_id(network)) {
        return true;
    }

    let is_testnet = matches!(network, WalletNetwork::Testnet);
    coin_registry.get_all().into_iter().any(|coin| {
        coin.is_testnet == is_testnet
            && coin.system_id.eq_ignore_ascii_case(system_id)
            && is_native_vrpc_system_coin(&coin)
    })
}

fn resolve_vrpc_coin_context(
    coin_registry: &CoinRegistry,
    system_id: &str,
    coin_id_hint: Option<&str>,
    network: WalletNetwork,
) -> Result<vrpc::VrpcCoinContext, WalletError> {
    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let coin = if let Some(coin_id) = coin_id_hint {
        let hinted = coin_registry
            .find_by_id(coin_id, is_testnet)
            .ok_or(WalletError::UnsupportedChannel)?;
        if !coin_supports_vrpc(&hinted) {
            return Err(WalletError::UnsupportedChannel);
        }

        let hinted_matches_scope = hinted.system_id.eq_ignore_ascii_case(system_id);
        if !hinted_matches_scope && !is_allowed_vrpc_scope_system(coin_registry, system_id, network)
        {
            return Err(WalletError::UnsupportedChannel);
        }
        hinted
    } else {
        coin_registry
            .find_by_system_id(system_id, is_testnet)
            .ok_or(WalletError::UnsupportedChannel)?
    };

    Ok(vrpc::VrpcCoinContext {
        currency_id: coin.currency_id,
        // Use the channel scope system for native-vs-PBaaS balance/tx parsing.
        system_id: system_id.to_string(),
        decimals: coin.decimals,
        seconds_per_block: coin.seconds_per_block,
    })
}

fn resolve_coin_by_channel(
    coin_registry: &CoinRegistry,
    coin_id: &str,
    network: WalletNetwork,
) -> Result<CoinDefinition, WalletError> {
    let is_testnet = matches!(network, WalletNetwork::Testnet);
    coin_registry
        .find_by_id(coin_id, is_testnet)
        .ok_or(WalletError::UnsupportedChannel)
}

/// Route preflight by channel_id prefix. VRPC and BTC use session addresses and providers.
pub async fn route_preflight(
    channel_id: &str,
    params: PreflightParams,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    coin_registry: &CoinRegistry,
    vrpc_provider_pool: &VrpcProviderPool,
    btc_provider_pool: &BtcProviderPool,
    eth_provider_pool: &EthProviderPool,
) -> Result<PreflightResult, WalletError> {
    let prefix = channel_id.split('.').next().unwrap_or("");
    match prefix {
        "vrpc" => {
            let session = session_manager.lock().await;
            let account_id = session
                .active_account_id()
                .ok_or(WalletError::WalletLocked)?
                .to_string();
            let (session_vrpc_address, _, _) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let resolved = vrpc::parse_vrpc_channel_id(channel_id, Some(&session_vrpc_address))?;
            let _coin = resolve_vrpc_coin_context(
                coin_registry,
                &resolved.system_id,
                Some(&params.coin_id),
                network,
            )?;

            // Phase-1 parity: we only own one derived VRPC address in this app.
            if resolved.address != session_vrpc_address {
                return Err(WalletError::InvalidAddress);
            }

            let canonical_channel_id =
                vrpc::canonical_vrpc_channel_id(&resolved.address, &resolved.system_id);
            if !vrpc_provider_pool.has_system_provider(network, &resolved.system_id) {
                println!(
                    "[VRPC] Missing system-specific endpoint for {}. Falling back to network default.",
                    resolved.system_id
                );
            }
            vrpc::preflight(
                params,
                preflight_store,
                &account_id,
                &resolved.address,
                &canonical_channel_id,
                vrpc_provider_pool.for_system(network, &resolved.system_id),
            )
            .await
        }
        "btc" => {
            let session = session_manager.lock().await;
            let account_id = session
                .active_account_id()
                .ok_or(WalletError::WalletLocked)?
                .to_string();
            let (_, _, from_address) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);
            btc::preflight_btc(
                params,
                preflight_store,
                &account_id,
                &from_address,
                channel_id,
                btc_provider_pool.for_network(network),
                network,
            )
            .await
        }
        "eth" => {
            let session = session_manager.lock().await;
            let account_id = session
                .active_account_id()
                .ok_or(WalletError::WalletLocked)?
                .to_string();
            let (_, from_address, _) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let coin_id = eth::parse_coin_channel_id(channel_id, "eth")?;
            let coin = resolve_coin_by_channel(coin_registry, &coin_id, network)?;
            if !coin
                .compatible_channels
                .iter()
                .any(|ch| matches!(ch, crate::core::coins::Channel::Eth))
            {
                return Err(WalletError::UnsupportedChannel);
            }

            eth::preflight_eth(
                params,
                preflight_store,
                &account_id,
                &from_address,
                channel_id,
                eth_provider_pool.for_network(network)?,
            )
            .await
        }
        "erc20" => {
            let session = session_manager.lock().await;
            let account_id = session
                .active_account_id()
                .ok_or(WalletError::WalletLocked)?
                .to_string();
            let (_, from_address, _) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let coin_id = eth::parse_coin_channel_id(channel_id, "erc20")?;
            let coin = resolve_coin_by_channel(coin_registry, &coin_id, network)?;
            if !coin
                .compatible_channels
                .iter()
                .any(|ch| matches!(ch, crate::core::coins::Channel::Erc20))
            {
                return Err(WalletError::UnsupportedChannel);
            }

            eth::preflight_erc20(
                params,
                preflight_store,
                &account_id,
                &from_address,
                channel_id,
                &coin,
                eth_provider_pool.for_network(network)?,
            )
            .await
        }
        _ => Err(WalletError::UnsupportedChannel),
    }
}

/// Route send by preflight_id: lookup record, dispatch by channel. VRPC/BTC: sign with session WIF and broadcast.
pub async fn route_send(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    vrpc_provider_pool: &VrpcProviderPool,
    btc_provider_pool: &BtcProviderPool,
    eth_provider_pool: &EthProviderPool,
) -> Result<SendResult, WalletError> {
    let record = preflight_store
        .get(preflight_id)
        .ok_or(WalletError::InvalidPreflight)?;
    let prefix = record.channel_id.split('.').next().unwrap_or("");
    match prefix {
        "vrpc" => {
            vrpc::send(
                preflight_id,
                preflight_store,
                session_manager,
                vrpc_provider_pool,
            )
            .await
        }
        "btc" => {
            btc::send_btc(
                preflight_id,
                preflight_store,
                session_manager,
                btc_provider_pool,
            )
            .await
        }
        "eth" | "erc20" => {
            eth::send(
                preflight_id,
                preflight_store,
                session_manager,
                eth_provider_pool,
            )
            .await
        }
        _ => Err(WalletError::UnsupportedChannel),
    }
}

/// Route balance fetch by channel_id. VRPC uses vrsc address; BTC uses btc address.
pub async fn route_get_balances(
    channel_id: &str,
    coin_id_hint: Option<&str>,
    session_manager: &Arc<Mutex<SessionManager>>,
    coin_registry: &CoinRegistry,
    vrpc_provider_pool: &VrpcProviderPool,
    btc_provider_pool: &BtcProviderPool,
    eth_provider_pool: &EthProviderPool,
) -> Result<BalanceResult, WalletError> {
    let prefix = channel_id.split('.').next().unwrap_or("");
    match prefix {
        "vrpc" => {
            let session = session_manager.lock().await;
            let (session_vrpc_address, _, _) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let resolved = vrpc::parse_vrpc_channel_id(channel_id, Some(&session_vrpc_address))?;
            let coin = resolve_vrpc_coin_context(
                coin_registry,
                &resolved.system_id,
                coin_id_hint,
                network,
            )?;
            if !vrpc_provider_pool.has_system_provider(network, &resolved.system_id) {
                println!(
                    "[VRPC] Missing system-specific endpoint for {}. Falling back to network default.",
                    resolved.system_id
                );
            }
            let addresses = vec![resolved.address];
            vrpc::get_balances(
                vrpc_provider_pool.for_system(network, &resolved.system_id),
                &addresses,
                &coin,
            )
            .await
        }
        "btc" => {
            let session = session_manager.lock().await;
            let (_, _, from_address) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);
            btc::get_balances_btc(btc_provider_pool.for_network(network), &[from_address]).await
        }
        "eth" => {
            let session = session_manager.lock().await;
            let (_, eth_address, _) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let coin_id = eth::parse_coin_channel_id(channel_id, "eth")?;
            let coin = resolve_coin_by_channel(coin_registry, &coin_id, network)?;
            if !coin
                .compatible_channels
                .iter()
                .any(|ch| matches!(ch, crate::core::coins::Channel::Eth))
            {
                return Err(WalletError::UnsupportedChannel);
            }

            eth::get_eth_balance(eth_provider_pool.for_network(network)?, &eth_address).await
        }
        "erc20" => {
            let session = session_manager.lock().await;
            let (_, eth_address, _) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let coin_id = eth::parse_coin_channel_id(channel_id, "erc20")?;
            let coin = resolve_coin_by_channel(coin_registry, &coin_id, network)?;
            if !coin
                .compatible_channels
                .iter()
                .any(|ch| matches!(ch, crate::core::coins::Channel::Erc20))
            {
                return Err(WalletError::UnsupportedChannel);
            }

            eth::get_erc20_balance(eth_provider_pool.for_network(network)?, &eth_address, &coin)
                .await
        }
        _ => Err(WalletError::UnsupportedChannel),
    }
}

/// Route transaction history fetch by channel_id. VRPC uses vrsc address; BTC uses btc address.
pub async fn route_get_transactions(
    channel_id: &str,
    coin_id_hint: Option<&str>,
    session_manager: &Arc<Mutex<SessionManager>>,
    coin_registry: &CoinRegistry,
    vrpc_provider_pool: &VrpcProviderPool,
    btc_provider_pool: &BtcProviderPool,
    eth_provider_pool: &EthProviderPool,
) -> Result<TransactionsFetchResult, WalletError> {
    let prefix = channel_id.split('.').next().unwrap_or("");
    match prefix {
        "vrpc" => {
            let session = session_manager.lock().await;
            let (session_vrpc_address, _, _) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let resolved = vrpc::parse_vrpc_channel_id(channel_id, Some(&session_vrpc_address))?;
            let coin = resolve_vrpc_coin_context(
                coin_registry,
                &resolved.system_id,
                coin_id_hint,
                network,
            )?;
            if !vrpc_provider_pool.has_system_provider(network, &resolved.system_id) {
                println!(
                    "[VRPC] Missing system-specific endpoint for {}. Falling back to network default.",
                    resolved.system_id
                );
            }
            let addresses = vec![resolved.address];
            let res = vrpc::get_transactions(
                vrpc_provider_pool.for_system(network, &resolved.system_id),
                &addresses,
                &coin,
            )
            .await?;
            Ok(TransactionsFetchResult {
                transactions: res.transactions,
                warning: res.warning,
            })
        }
        "btc" => {
            let session = session_manager.lock().await;
            let (_, _, from_address) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);
            let txs =
                btc::get_transactions_btc(btc_provider_pool.for_network(network), &[from_address])
                    .await?;
            Ok(TransactionsFetchResult {
                transactions: txs,
                warning: None,
            })
        }
        "eth" => {
            let session = session_manager.lock().await;
            let (_, eth_address, _) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let coin_id = eth::parse_coin_channel_id(channel_id, "eth")?;
            let coin = resolve_coin_by_channel(coin_registry, &coin_id, network)?;
            if !coin
                .compatible_channels
                .iter()
                .any(|ch| matches!(ch, crate::core::coins::Channel::Eth))
            {
                return Err(WalletError::UnsupportedChannel);
            }

            let txs = eth::get_eth_transactions(
                eth_provider_pool.for_network(network)?,
                network,
                &eth_address,
            )
            .await?;
            Ok(TransactionsFetchResult {
                transactions: txs,
                warning: None,
            })
        }
        "erc20" => {
            let session = session_manager.lock().await;
            let (_, eth_address, _) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let coin_id = eth::parse_coin_channel_id(channel_id, "erc20")?;
            let coin = resolve_coin_by_channel(coin_registry, &coin_id, network)?;
            if !coin
                .compatible_channels
                .iter()
                .any(|ch| matches!(ch, crate::core::coins::Channel::Erc20))
            {
                return Err(WalletError::UnsupportedChannel);
            }

            let txs = eth::get_erc20_transactions(
                eth_provider_pool.for_network(network)?,
                network,
                &eth_address,
                &coin,
            )
            .await?;
            Ok(TransactionsFetchResult {
                transactions: txs,
                warning: None,
            })
        }
        _ => Err(WalletError::UnsupportedChannel),
    }
}

/// Route paged transaction history fetch by channel_id.
pub async fn route_get_transactions_page(
    channel_id: &str,
    coin_id_hint: Option<&str>,
    cursor: Option<&str>,
    limit: Option<u32>,
    session_manager: &Arc<Mutex<SessionManager>>,
    coin_registry: &CoinRegistry,
    vrpc_provider_pool: &VrpcProviderPool,
    btc_provider_pool: &BtcProviderPool,
    eth_provider_pool: &EthProviderPool,
) -> Result<TransactionHistoryPage, WalletError> {
    let requested_limit = clamp_history_limit(limit);
    let decoded_cursor = if let Some(raw_cursor) = cursor {
        Some(decode_history_cursor(raw_cursor)?)
    } else {
        None
    };

    let prefix = channel_id.split('.').next().unwrap_or("");
    let mut page = match prefix {
        "vrpc" => {
            let session = session_manager.lock().await;
            let (session_vrpc_address, _, _) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let resolved = vrpc::parse_vrpc_channel_id(channel_id, Some(&session_vrpc_address))?;
            let coin = resolve_vrpc_coin_context(
                coin_registry,
                &resolved.system_id,
                coin_id_hint,
                network,
            )?;
            if !vrpc_provider_pool.has_system_provider(network, &resolved.system_id) {
                println!(
                    "[VRPC] Missing system-specific endpoint for {}. Falling back to network default.",
                    resolved.system_id
                );
            }
            let addresses = vec![resolved.address];
            let vrpc_cursor = match decoded_cursor {
                Some(TransactionHistoryCursor::Vrpc {
                    end_block,
                    include_pending,
                }) => Some(vrpc::VrpcHistoryCursor {
                    end_block,
                    include_pending,
                }),
                Some(_) => return Err(WalletError::OperationFailed),
                None => None,
            };
            let res = vrpc::get_transactions_page(
                vrpc_provider_pool.for_system(network, &resolved.system_id),
                &addresses,
                &coin,
                vrpc_cursor,
                requested_limit,
            )
            .await?;

            ProtocolPageResult {
                transactions: res.transactions,
                next_cursor: res.next_cursor.map(|cursor| TransactionHistoryCursor::Vrpc {
                    end_block: cursor.end_block,
                    include_pending: cursor.include_pending,
                }),
                has_more: res.has_more,
                warning: res.warning,
            }
        }
        "btc" => {
            let session = session_manager.lock().await;
            let (_, _, from_address) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let btc_cursor = match decoded_cursor {
                Some(TransactionHistoryCursor::Btc { ref last_seen_txid }) => {
                    last_seen_txid.clone()
                }
                Some(_) => return Err(WalletError::OperationFailed),
                None => None,
            };

            let res = btc::get_transactions_page_btc(
                btc_provider_pool.for_network(network),
                &[from_address],
                btc_cursor.as_deref(),
                requested_limit,
            )
            .await?;

            ProtocolPageResult {
                transactions: res.transactions,
                next_cursor: if res.has_more {
                    Some(TransactionHistoryCursor::Btc {
                        last_seen_txid: res.next_last_seen_txid,
                    })
                } else {
                    None
                },
                has_more: res.has_more,
                warning: None,
            }
        }
        "eth" => {
            let session = session_manager.lock().await;
            let (_, eth_address, _) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let coin_id = eth::parse_coin_channel_id(channel_id, "eth")?;
            let coin = resolve_coin_by_channel(coin_registry, &coin_id, network)?;
            if !coin
                .compatible_channels
                .iter()
                .any(|ch| matches!(ch, crate::core::coins::Channel::Eth))
            {
                return Err(WalletError::UnsupportedChannel);
            }

            let page_number = match decoded_cursor {
                Some(TransactionHistoryCursor::Eth { page }) => page,
                Some(_) => return Err(WalletError::OperationFailed),
                None => 1,
            };

            let res = eth::get_eth_transactions_page(
                eth_provider_pool.for_network(network)?,
                network,
                &eth_address,
                page_number,
                requested_limit,
            )
            .await?;

            ProtocolPageResult {
                transactions: res.transactions,
                next_cursor: if res.has_more {
                    Some(TransactionHistoryCursor::Eth {
                        page: res.next_page,
                    })
                } else {
                    None
                },
                has_more: res.has_more,
                warning: None,
            }
        }
        "erc20" => {
            let session = session_manager.lock().await;
            let (_, eth_address, _) = session.get_addresses()?;
            let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
            drop(session);

            let coin_id = eth::parse_coin_channel_id(channel_id, "erc20")?;
            let coin = resolve_coin_by_channel(coin_registry, &coin_id, network)?;
            if !coin
                .compatible_channels
                .iter()
                .any(|ch| matches!(ch, crate::core::coins::Channel::Erc20))
            {
                return Err(WalletError::UnsupportedChannel);
            }

            let page_number = match decoded_cursor {
                Some(TransactionHistoryCursor::Erc20 { page }) => page,
                Some(_) => return Err(WalletError::OperationFailed),
                None => 1,
            };

            let res = eth::get_erc20_transactions_page(
                eth_provider_pool.for_network(network)?,
                network,
                &eth_address,
                &coin,
                page_number,
                requested_limit,
            )
            .await?;

            ProtocolPageResult {
                transactions: res.transactions,
                next_cursor: if res.has_more {
                    Some(TransactionHistoryCursor::Erc20 {
                        page: res.next_page,
                    })
                } else {
                    None
                },
                has_more: res.has_more,
                warning: None,
            }
        }
        _ => return Err(WalletError::UnsupportedChannel),
    };

    page.transactions = dedupe_transactions_by_txid(page.transactions);
    if page.transactions.len() > requested_limit {
        page.transactions.truncate(requested_limit);
    }

    let next_cursor = if page.has_more {
        page.next_cursor
            .as_ref()
            .map(encode_history_cursor)
            .transpose()?
    } else {
        None
    };

    println!(
        "[TX][PAGE] channel={} limit={} returned={} has_more={} next_cursor={}",
        channel_id,
        requested_limit,
        page.transactions.len(),
        page.has_more,
        next_cursor.as_ref().map(|_| "yes").unwrap_or("no")
    );

    Ok(TransactionHistoryPage {
        transactions: page.transactions,
        next_cursor,
        has_more: page.has_more,
        warning: page.warning,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        clamp_history_limit, decode_history_cursor, encode_history_cursor, resolve_vrpc_coin_context,
        TransactionHistoryCursor,
    };
    use crate::core::coins::{Channel, CoinDefinition, CoinRegistry, Protocol};
    use crate::types::wallet::WalletNetwork;

    const VRSC_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
    const VETH_SYSTEM_ID: &str = "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X";
    const CHIPS_SYSTEM_ID: &str = "iJ3WZocnjG9ufv7GKUA4LijQno5gTMb7tP";

    fn sample_vrpc_coin(
        id: &str,
        currency_id: &str,
        system_id: &str,
        ticker: &str,
        name: &str,
    ) -> CoinDefinition {
        CoinDefinition {
            id: id.to_string(),
            currency_id: currency_id.to_string(),
            system_id: system_id.to_string(),
            display_ticker: ticker.to_string(),
            display_name: name.to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec!["https://api.verus.services/".to_string()],
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        }
    }

    #[test]
    fn resolve_vrpc_coin_context_allows_root_scope_for_vrpc_token() {
        let registry = CoinRegistry::new();
        registry
            .add_coin(sample_vrpc_coin(
                "vUSDC",
                "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd",
                VETH_SYSTEM_ID,
                "vUSDC.vETH",
                "USDC on Verus",
            ))
            .expect("add vUSDC");

        let context = resolve_vrpc_coin_context(
            &registry,
            VRSC_SYSTEM_ID,
            Some("vUSDC"),
            WalletNetwork::Mainnet,
        )
        .expect("resolve context");
        assert_eq!(context.currency_id, "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd");
    }

    #[test]
    fn resolve_vrpc_coin_context_uses_scope_system_for_native_detection() {
        let registry = CoinRegistry::new();
        registry
            .add_coin(sample_vrpc_coin(
                "vDEX",
                "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N",
                "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N",
                "vDEX",
                "vDEX",
            ))
            .expect("add vDEX");

        let context = resolve_vrpc_coin_context(
            &registry,
            VRSC_SYSTEM_ID,
            Some("vDEX"),
            WalletNetwork::Mainnet,
        )
        .expect("resolve context");

        assert_eq!(context.currency_id, "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N");
        assert_eq!(context.system_id, VRSC_SYSTEM_ID);
    }

    #[test]
    fn resolve_vrpc_coin_context_allows_native_chain_scope_for_vrpc_token() {
        let registry = CoinRegistry::new();
        registry
            .add_coin(sample_vrpc_coin(
                "vUSDC",
                "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd",
                VETH_SYSTEM_ID,
                "vUSDC.vETH",
                "USDC on Verus",
            ))
            .expect("add vUSDC");
        registry
            .add_coin(sample_vrpc_coin(
                "CHIPS",
                CHIPS_SYSTEM_ID,
                CHIPS_SYSTEM_ID,
                "CHIPS",
                "CHIPS",
            ))
            .expect("add CHIPS");

        let context = resolve_vrpc_coin_context(
            &registry,
            CHIPS_SYSTEM_ID,
            Some("vUSDC"),
            WalletNetwork::Mainnet,
        )
        .expect("resolve context");
        assert_eq!(context.currency_id, "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd");
    }

    #[test]
    fn resolve_vrpc_coin_context_rejects_unrelated_non_native_scope() {
        let registry = CoinRegistry::new();
        registry
            .add_coin(sample_vrpc_coin(
                "vUSDC",
                "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd",
                VETH_SYSTEM_ID,
                "vUSDC.vETH",
                "USDC on Verus",
            ))
            .expect("add vUSDC");

        let result = resolve_vrpc_coin_context(
            &registry,
            "iUnrelatedSystem",
            Some("vUSDC"),
            WalletNetwork::Mainnet,
        );
        assert!(result.is_err());
    }

    #[test]
    fn clamp_history_limit_enforces_bounds() {
        assert_eq!(clamp_history_limit(None), 50);
        assert_eq!(clamp_history_limit(Some(0)), 1);
        assert_eq!(clamp_history_limit(Some(1)), 1);
        assert_eq!(clamp_history_limit(Some(50)), 50);
        assert_eq!(clamp_history_limit(Some(500)), 100);
    }

    #[test]
    fn transaction_history_cursor_round_trip_for_all_kinds() {
        let cases = vec![
            TransactionHistoryCursor::Vrpc {
                end_block: 123_456,
                include_pending: false,
            },
            TransactionHistoryCursor::Btc {
                last_seen_txid: Some("abcd".to_string()),
            },
            TransactionHistoryCursor::Eth { page: 3 },
            TransactionHistoryCursor::Erc20 { page: 8 },
        ];

        for case in cases {
            let encoded = encode_history_cursor(&case).expect("encode cursor");
            let decoded = decode_history_cursor(&encoded).expect("decode cursor");
            match (case, decoded) {
                (
                    TransactionHistoryCursor::Vrpc {
                        end_block: expected_end,
                        include_pending: expected_pending,
                    },
                    TransactionHistoryCursor::Vrpc {
                        end_block: actual_end,
                        include_pending: actual_pending,
                    },
                ) => {
                    assert_eq!(expected_end, actual_end);
                    assert_eq!(expected_pending, actual_pending);
                }
                (
                    TransactionHistoryCursor::Btc {
                        last_seen_txid: expected_txid,
                    },
                    TransactionHistoryCursor::Btc {
                        last_seen_txid: actual_txid,
                    },
                ) => {
                    assert_eq!(expected_txid, actual_txid);
                }
                (
                    TransactionHistoryCursor::Eth {
                        page: expected_page,
                    },
                    TransactionHistoryCursor::Eth { page: actual_page },
                ) => {
                    assert_eq!(expected_page, actual_page);
                }
                (
                    TransactionHistoryCursor::Erc20 {
                        page: expected_page,
                    },
                    TransactionHistoryCursor::Erc20 { page: actual_page },
                ) => {
                    assert_eq!(expected_page, actual_page);
                }
                _ => panic!("decoded cursor kind mismatch"),
            }
        }
    }
}
