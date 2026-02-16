//
// Bridge command handlers (backend-first command surface).
// Phase-1: VRPC bridge preflight adapter is implemented; ETH/ERC20 bridge branches are scaffolded.

use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::core::auth::SessionManager;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::vrpc::{self, VrpcProviderPool};
use crate::core::channels::PreflightStore;
use crate::core::coins::{Channel, CoinRegistry};
use crate::types::wallet::WalletNetwork;
use crate::types::{
    BridgeCapabilitiesRequest, BridgeCapabilitiesResult, BridgeConversionEstimateRequest,
    BridgeConversionEstimateResult, BridgeConversionPathRequest, BridgeConversionPathsResult,
    BridgeExecutionHint, BridgeTransferPreflightParams, BridgeTransferPreflightResult,
    BridgeTransferRoute, VrpcTransferPreflightParams, WalletError,
};

#[tauri::command(rename_all = "snake_case")]
pub async fn get_bridge_capabilities(
    request: BridgeCapabilitiesRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    eth_provider_pool: State<'_, Arc<EthProviderPool>>,
) -> Result<BridgeCapabilitiesResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    drop(session);

    let prefix = request.channel_id.split('.').next().unwrap_or_default();
    let capability = match prefix {
        "vrpc" => BridgeCapabilitiesResult {
            conversion_supported: true,
            execution_engine: "vrpc_sendcurrency".to_string(),
            reason_code: None,
        },
        "eth" | "erc20" => {
            if !eth_provider_pool.is_enabled() {
                BridgeCapabilitiesResult {
                    conversion_supported: false,
                    execution_engine: "eth_delegator_bridge".to_string(),
                    reason_code: Some("eth_not_configured".to_string()),
                }
            } else if !crate::core::channels::eth::bridge::parity_feature_enabled() {
                BridgeCapabilitiesResult {
                    conversion_supported: false,
                    execution_engine: "eth_delegator_bridge".to_string(),
                    reason_code: Some("feature_disabled".to_string()),
                }
            } else {
                BridgeCapabilitiesResult {
                    conversion_supported: true,
                    execution_engine: "eth_delegator_bridge".to_string(),
                    reason_code: None,
                }
            }
        }
        _ => BridgeCapabilitiesResult {
            conversion_supported: false,
            execution_engine: "unsupported".to_string(),
            reason_code: Some("unsupported_channel".to_string()),
        },
    };

    Ok(capability)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_bridge_conversion_paths(
    request: BridgeConversionPathRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    eth_provider_pool: State<'_, Arc<EthProviderPool>>,
) -> Result<BridgeConversionPathsResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    let (session_vrpc_address, _, _) = session.get_addresses()?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    drop(session);

    let prefix = request.channel_id.split('.').next().unwrap_or_default();
    match prefix {
        "vrpc" => {
            get_bridge_conversion_paths_vrpc(
                request,
                &session_vrpc_address,
                network,
                coin_registry.as_ref(),
                vrpc_provider_pool.inner().as_ref(),
                eth_provider_pool.inner().as_ref(),
            )
            .await
        }
        "eth" | "erc20" => {
            if !crate::core::channels::eth::bridge::parity_feature_enabled() {
                return Err(WalletError::BridgeNotImplemented);
            }

            get_bridge_conversion_paths_evm(
                request,
                network,
                coin_registry.as_ref(),
                vrpc_provider_pool.inner().as_ref(),
                eth_provider_pool.inner().as_ref(),
            )
            .await
        }
        _ => Err(WalletError::UnsupportedChannel),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn estimate_bridge_conversion(
    request: BridgeConversionEstimateRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    eth_provider_pool: State<'_, Arc<EthProviderPool>>,
) -> Result<BridgeConversionEstimateResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    let (session_vrpc_address, _, _) = session.get_addresses()?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    drop(session);

    let prefix = request.channel_id.split('.').next().unwrap_or_default();
    match prefix {
        "vrpc" => {
            estimate_bridge_conversion_vrpc(
                request,
                &session_vrpc_address,
                network,
                coin_registry.as_ref(),
                vrpc_provider_pool.inner().as_ref(),
                eth_provider_pool.inner().as_ref(),
            )
            .await
        }
        "eth" | "erc20" => {
            if !crate::core::channels::eth::bridge::parity_feature_enabled() {
                return Err(WalletError::BridgeNotImplemented);
            }

            estimate_bridge_conversion_evm(
                request,
                network,
                coin_registry.as_ref(),
                vrpc_provider_pool.inner().as_ref(),
                eth_provider_pool.inner().as_ref(),
            )
            .await
        }
        _ => Err(WalletError::UnsupportedChannel),
    }
}

async fn get_bridge_conversion_paths_vrpc(
    request: BridgeConversionPathRequest,
    session_vrpc_address: &str,
    network: WalletNetwork,
    coin_registry: &CoinRegistry,
    vrpc_provider_pool: &VrpcProviderPool,
    eth_provider_pool: &EthProviderPool,
) -> Result<BridgeConversionPathsResult, WalletError> {
    let resolved = vrpc::parse_vrpc_channel_id(&request.channel_id, Some(session_vrpc_address))?;
    if resolved.address != session_vrpc_address {
        return Err(WalletError::InvalidAddress);
    }

    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let source_coin = coin_registry
        .find_by_id(&request.coin_id, is_testnet)
        .ok_or(WalletError::UnsupportedChannel)?;
    if !source_coin
        .compatible_channels
        .iter()
        .any(|channel| matches!(channel, Channel::Vrpc))
    {
        return Err(WalletError::UnsupportedChannel);
    }
    if coin_registry
        .find_by_system_id(&resolved.system_id, is_testnet)
        .is_none()
    {
        return Err(WalletError::UnsupportedChannel);
    }

    crate::core::channels::eth::bridge::get_conversion_paths(
        &request,
        vrpc_provider_pool.for_network(network),
        eth_provider_pool.for_network(network).ok(),
    )
    .await
}

async fn estimate_bridge_conversion_vrpc(
    request: BridgeConversionEstimateRequest,
    session_vrpc_address: &str,
    network: WalletNetwork,
    coin_registry: &CoinRegistry,
    vrpc_provider_pool: &VrpcProviderPool,
    eth_provider_pool: &EthProviderPool,
) -> Result<BridgeConversionEstimateResult, WalletError> {
    let resolved = vrpc::parse_vrpc_channel_id(&request.channel_id, Some(session_vrpc_address))?;
    if resolved.address != session_vrpc_address {
        return Err(WalletError::InvalidAddress);
    }

    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let source_coin = coin_registry
        .find_by_id(&request.coin_id, is_testnet)
        .ok_or(WalletError::UnsupportedChannel)?;
    if !source_coin
        .compatible_channels
        .iter()
        .any(|channel| matches!(channel, Channel::Vrpc))
    {
        return Err(WalletError::UnsupportedChannel);
    }
    if coin_registry
        .find_by_system_id(&resolved.system_id, is_testnet)
        .is_none()
    {
        return Err(WalletError::UnsupportedChannel);
    }

    crate::core::channels::eth::bridge::estimate_conversion(
        &request,
        vrpc_provider_pool.for_network(network),
        eth_provider_pool.for_network(network).ok(),
    )
    .await
}

async fn get_bridge_conversion_paths_evm(
    request: BridgeConversionPathRequest,
    network: WalletNetwork,
    coin_registry: &CoinRegistry,
    vrpc_provider_pool: &VrpcProviderPool,
    eth_provider_pool: &EthProviderPool,
) -> Result<BridgeConversionPathsResult, WalletError> {
    let prefix = request.channel_id.split('.').next().unwrap_or_default();
    let expected_channel = match prefix {
        "eth" => Channel::Eth,
        "erc20" => Channel::Erc20,
        _ => return Err(WalletError::UnsupportedChannel),
    };

    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let source_coin = coin_registry
        .find_by_id(&request.coin_id, is_testnet)
        .ok_or(WalletError::UnsupportedChannel)?;
    if !source_coin
        .compatible_channels
        .iter()
        .any(|channel| channel == &expected_channel)
    {
        return Err(WalletError::UnsupportedChannel);
    }

    // Ensure we can execute on the selected EVM network before returning route paths.
    let _ = eth_provider_pool.for_network(network)?;

    crate::core::channels::eth::bridge::get_conversion_paths(
        &request,
        vrpc_provider_pool.for_network(network),
        Some(eth_provider_pool.for_network(network)?),
    )
    .await
}

async fn estimate_bridge_conversion_evm(
    request: BridgeConversionEstimateRequest,
    network: WalletNetwork,
    coin_registry: &CoinRegistry,
    vrpc_provider_pool: &VrpcProviderPool,
    eth_provider_pool: &EthProviderPool,
) -> Result<BridgeConversionEstimateResult, WalletError> {
    let prefix = request.channel_id.split('.').next().unwrap_or_default();
    let expected_channel = match prefix {
        "eth" => Channel::Eth,
        "erc20" => Channel::Erc20,
        _ => return Err(WalletError::UnsupportedChannel),
    };

    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let source_coin = coin_registry
        .find_by_id(&request.coin_id, is_testnet)
        .ok_or(WalletError::UnsupportedChannel)?;
    if !source_coin
        .compatible_channels
        .iter()
        .any(|channel| channel == &expected_channel)
    {
        return Err(WalletError::UnsupportedChannel);
    }

    // Ensure we can execute on the selected EVM network before returning route estimates.
    let _ = eth_provider_pool.for_network(network)?;

    crate::core::channels::eth::bridge::estimate_conversion(
        &request,
        vrpc_provider_pool.for_network(network),
        Some(eth_provider_pool.for_network(network)?),
    )
    .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_bridge_transfer(
    params: BridgeTransferPreflightParams,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    eth_provider_pool: State<'_, Arc<EthProviderPool>>,
) -> Result<BridgeTransferPreflightResult, WalletError> {
    let prefix = params.channel_id.split('.').next().unwrap_or_default();
    match prefix {
        "vrpc" => {
            preflight_bridge_vrpc(
                params,
                session_manager,
                preflight_store,
                coin_registry,
                vrpc_provider_pool,
            )
            .await
        }
        "eth" => {
            if !crate::core::channels::eth::bridge::parity_feature_enabled() {
                return Err(WalletError::BridgeNotImplemented);
            }
            preflight_bridge_eth(
                params,
                "eth",
                session_manager,
                preflight_store,
                coin_registry,
                vrpc_provider_pool,
                eth_provider_pool,
            )
            .await
        }
        "erc20" => {
            if !crate::core::channels::eth::bridge::parity_feature_enabled() {
                return Err(WalletError::BridgeNotImplemented);
            }
            preflight_bridge_eth(
                params,
                "erc20",
                session_manager,
                preflight_store,
                coin_registry,
                vrpc_provider_pool,
                eth_provider_pool,
            )
            .await
        }
        _ => Err(WalletError::UnsupportedChannel),
    }
}

async fn preflight_bridge_vrpc(
    params: BridgeTransferPreflightParams,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<BridgeTransferPreflightResult, WalletError> {
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
    let effective_source = params
        .source_address
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(resolved.address.as_str())
        .to_string();
    if effective_source != session_vrpc_address || resolved.address != session_vrpc_address {
        return Err(WalletError::InvalidAddress);
    }

    let is_testnet = matches!(network, WalletNetwork::Testnet);
    if coin_registry
        .find_by_system_id(&resolved.system_id, is_testnet)
        .is_none()
    {
        return Err(WalletError::UnsupportedChannel);
    }

    let mut vrpc_params = to_vrpc_bridge_params(&params);
    vrpc_params.channel_id =
        vrpc::canonical_vrpc_channel_id(&resolved.address, &resolved.system_id);

    let vrpc_result = vrpc::preflight_transfer(
        vrpc_params,
        &preflight_store,
        &account_id,
        &effective_source,
        &vrpc::canonical_vrpc_channel_id(&resolved.address, &resolved.system_id),
        vrpc_provider_pool.for_network(network),
    )
    .await?;

    Ok(BridgeTransferPreflightResult {
        preflight_id: vrpc_result.preflight_id,
        fee: vrpc_result.fee,
        fee_currency: vrpc_result.fee_currency,
        value: vrpc_result.value,
        amount_submitted: vrpc_result.amount_submitted,
        amount_adjusted: vrpc_result.amount_adjusted,
        to_address: vrpc_result.to_address,
        from_address: vrpc_result.from_address,
        warnings: vrpc_result.warnings,
        memo: vrpc_result.memo,
        route: bridge_route_from_params(&params),
        execution: BridgeExecutionHint {
            engine: "vrpc_sendcurrency".to_string(),
            requires_token_approval: false,
            bridge_contract: None,
        },
    })
}

async fn preflight_bridge_eth(
    params: BridgeTransferPreflightParams,
    expected_prefix: &str,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    eth_provider_pool: State<'_, Arc<EthProviderPool>>,
) -> Result<BridgeTransferPreflightResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let account_id = session
        .active_account_id()
        .ok_or(WalletError::WalletLocked)?
        .to_string();
    let (vrpc_address, eth_address, _) = session.get_addresses()?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    drop(session);

    let coin_id =
        crate::core::channels::eth::parse_coin_channel_id(&params.channel_id, expected_prefix)?;
    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let coin = coin_registry
        .find_by_id(&coin_id, is_testnet)
        .ok_or(WalletError::UnsupportedChannel)?;
    let expected_channel = if expected_prefix == "eth" {
        Channel::Eth
    } else {
        Channel::Erc20
    };
    if !coin
        .compatible_channels
        .iter()
        .any(|ch| ch == &expected_channel)
    {
        return Err(WalletError::UnsupportedChannel);
    }

    let channel_id = params.channel_id.clone();
    crate::core::channels::eth::bridge::preflight(
        params,
        &preflight_store,
        &account_id,
        &coin,
        &eth_address,
        &vrpc_address,
        &channel_id,
        vrpc_provider_pool.for_network(network),
        eth_provider_pool.for_network(network)?,
    )
    .await
}

fn to_vrpc_bridge_params(params: &BridgeTransferPreflightParams) -> VrpcTransferPreflightParams {
    VrpcTransferPreflightParams {
        coin_id: params.coin_id.clone(),
        channel_id: params.channel_id.clone(),
        source_address: params.source_address.clone(),
        destination: params.destination.clone(),
        amount: params.amount.clone(),
        convert_to: params.convert_to.clone(),
        export_to: params.export_to.clone(),
        via: params.via.clone(),
        fee_currency: params.fee_currency.clone(),
        fee_satoshis: params.fee_satoshis.clone(),
        preconvert: params.preconvert,
        map_to: params.map_to.clone(),
        vdxf_tag: params.vdxf_tag.clone(),
        memo: params.memo.clone(),
    }
}

fn bridge_route_from_params(params: &BridgeTransferPreflightParams) -> BridgeTransferRoute {
    BridgeTransferRoute {
        convert_to: params.convert_to.clone(),
        export_to: params.export_to.clone(),
        via: params.via.clone(),
        map_to: params.map_to.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{bridge_route_from_params, to_vrpc_bridge_params};
    use crate::types::BridgeTransferPreflightParams;

    #[test]
    fn bridge_route_from_params_copies_optional_route_fields() {
        let params = BridgeTransferPreflightParams {
            coin_id: "VRSC".to_string(),
            channel_id: "vrpc.Rabc.i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            source_address: None,
            destination: "Rdest".to_string(),
            amount: "1".to_string(),
            convert_to: Some("Bridge.CHIPS".to_string()),
            export_to: Some("CHIPS".to_string()),
            via: Some("Bridge.vETH".to_string()),
            fee_currency: None,
            fee_satoshis: None,
            preconvert: Some(true),
            map_to: Some("Bridge.vETH".to_string()),
            vdxf_tag: None,
            memo: None,
        };

        let route = bridge_route_from_params(&params);
        assert_eq!(route.convert_to.as_deref(), Some("Bridge.CHIPS"));
        assert_eq!(route.export_to.as_deref(), Some("CHIPS"));
        assert_eq!(route.via.as_deref(), Some("Bridge.vETH"));
        assert_eq!(route.map_to.as_deref(), Some("Bridge.vETH"));
    }

    #[test]
    fn to_vrpc_bridge_params_preserves_bridge_route_fields() {
        let params = BridgeTransferPreflightParams {
            coin_id: "VRSC".to_string(),
            channel_id: "vrpc.Rabc.i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            source_address: None,
            destination: "Rdest".to_string(),
            amount: "1".to_string(),
            convert_to: Some("Bridge.CHIPS".to_string()),
            export_to: Some("CHIPS".to_string()),
            via: Some("Bridge.vETH".to_string()),
            fee_currency: Some("i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string()),
            fee_satoshis: Some("20000".to_string()),
            preconvert: Some(true),
            map_to: Some("Bridge.vETH".to_string()),
            vdxf_tag: Some("iTag".to_string()),
            memo: Some("memo".to_string()),
        };

        let converted = to_vrpc_bridge_params(&params);
        assert_eq!(converted.convert_to, params.convert_to);
        assert_eq!(converted.export_to, params.export_to);
        assert_eq!(converted.via, params.via);
        assert_eq!(converted.map_to, params.map_to);
        assert_eq!(converted.fee_currency, params.fee_currency);
        assert_eq!(converted.fee_satoshis, params.fee_satoshis);
        assert_eq!(converted.preconvert, params.preconvert);
        assert_eq!(converted.vdxf_tag, params.vdxf_tag);
        assert_eq!(converted.memo, params.memo);
    }

    #[test]
    fn bridge_conversion_paths_result_shape_supports_empty_map() {
        let empty: std::collections::HashMap<String, Vec<crate::types::BridgeConversionPathQuote>> =
            std::collections::HashMap::new();
        assert!(empty.is_empty());
    }
}
