//
// ETH/ERC20 bridge send path.
// Phase-1 scaffold: deterministic not-implemented to preserve explicit behavior.

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::core::auth::SessionManager;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::PreflightStore;
use crate::types::transaction::SendResult;
use crate::types::WalletError;

pub async fn send(
    _preflight_id: &str,
    _preflight_store: &PreflightStore,
    _session_manager: &Arc<Mutex<SessionManager>>,
    _provider_pool: &EthProviderPool,
) -> Result<SendResult, WalletError> {
    Err(WalletError::BridgeNotImplemented)
}
