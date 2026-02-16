//
// ETH/ERC20 bridge send path.
// Bridge preflights are executed through the unified ETH send dispatcher.

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::core::auth::SessionManager;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::PreflightStore;
use crate::types::transaction::SendResult;
use crate::types::WalletError;

pub async fn send(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    provider_pool: &EthProviderPool,
) -> Result<SendResult, WalletError> {
    crate::core::channels::eth::send(
        preflight_id,
        preflight_store,
        session_manager,
        provider_pool,
    )
    .await
}
