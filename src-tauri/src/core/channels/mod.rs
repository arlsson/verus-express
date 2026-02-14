//
// Module 4 + 5 + 5d + 9: Channel trait and router. Dispatches preflight/send and balance/tx by channel_id; VRPC and BTC.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::core::auth::SessionManager;
use crate::core::channels::btc::BtcProviderPool;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::coins::{CoinDefinition, CoinRegistry};
use crate::types::transaction::{
    BalanceResult, PreflightParams, PreflightResult, SendResult, Transaction,
};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

pub mod btc;
pub mod eth;
mod store;
pub mod vrpc;

pub use store::{PreflightRecord, PreflightStore};

#[derive(Debug, Clone)]
pub struct TransactionsFetchResult {
    pub transactions: Vec<Transaction>,
    pub warning: Option<String>,
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

fn resolve_vrpc_coin_context(
    coin_registry: &CoinRegistry,
    system_id: &str,
    network: WalletNetwork,
) -> Result<vrpc::VrpcCoinContext, WalletError> {
    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let coin = coin_registry
        .find_by_system_id(system_id, is_testnet)
        .ok_or(WalletError::UnsupportedChannel)?;

    Ok(vrpc::VrpcCoinContext {
        currency_id: coin.currency_id,
        system_id: coin.system_id,
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
            let _coin = resolve_vrpc_coin_context(coin_registry, &resolved.system_id, network)?;

            // Phase-1 parity: we only own one derived VRPC address in this app.
            if resolved.address != session_vrpc_address {
                return Err(WalletError::InvalidAddress);
            }

            let canonical_channel_id =
                vrpc::canonical_vrpc_channel_id(&resolved.address, &resolved.system_id);
            vrpc::preflight(
                params,
                preflight_store,
                &account_id,
                &resolved.address,
                &canonical_channel_id,
                vrpc_provider_pool.for_network(network),
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
            if !coin.compatible_channels.iter().any(|ch| matches!(ch, crate::core::coins::Channel::Eth))
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
            let coin = resolve_vrpc_coin_context(coin_registry, &resolved.system_id, network)?;
            let addresses = vec![resolved.address];
            vrpc::get_balances(vrpc_provider_pool.for_network(network), &addresses, &coin).await
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
            if !coin.compatible_channels.iter().any(|ch| matches!(ch, crate::core::coins::Channel::Eth))
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

            eth::get_erc20_balance(eth_provider_pool.for_network(network)?, &eth_address, &coin).await
        }
        _ => Err(WalletError::UnsupportedChannel),
    }
}

/// Route transaction history fetch by channel_id. VRPC uses vrsc address; BTC uses btc address.
pub async fn route_get_transactions(
    channel_id: &str,
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
            let coin = resolve_vrpc_coin_context(coin_registry, &resolved.system_id, network)?;
            let addresses = vec![resolved.address];
            let res =
                vrpc::get_transactions(vrpc_provider_pool.for_network(network), &addresses, &coin)
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
            if !coin.compatible_channels.iter().any(|ch| matches!(ch, crate::core::coins::Channel::Eth))
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
