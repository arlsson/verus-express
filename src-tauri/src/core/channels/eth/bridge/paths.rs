//
// Bridge conversion-path discovery.
// Phase-1 scaffold: deterministic not-implemented for ETH bridge paths.

use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::core::channels::vrpc::VrpcProvider;
use crate::types::bridge::{BridgeConversionPathRequest, BridgeConversionPathsResult};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

pub async fn get_conversion_paths(
    _request: &BridgeConversionPathRequest,
    _network: WalletNetwork,
    _vrpc_provider: &VrpcProvider,
    _eth_provider: &EthNetworkProvider,
) -> Result<BridgeConversionPathsResult, WalletError> {
    Err(WalletError::BridgeNotImplemented)
}
