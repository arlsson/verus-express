//
// ETH/ERC20 bridge preflight.
// Phase-1 scaffold: typed entrypoint with deterministic not-implemented return.

use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::core::channels::PreflightStore;
use crate::types::bridge::{BridgeTransferPreflightParams, BridgeTransferPreflightResult};
use crate::types::WalletError;

pub async fn preflight(
    _params: BridgeTransferPreflightParams,
    _preflight_store: &PreflightStore,
    _account_id: &str,
    _from_address: &str,
    _channel_id: &str,
    _provider: &EthNetworkProvider,
) -> Result<BridgeTransferPreflightResult, WalletError> {
    Err(WalletError::BridgeNotImplemented)
}
