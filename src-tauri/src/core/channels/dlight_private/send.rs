use std::sync::Arc;

use tokio::sync::Mutex;

use crate::core::auth::SessionManager;
use crate::core::channels::store::PreflightStore;
use crate::types::transaction::SendResult;
use crate::types::WalletError;

use super::{DlightPreflightPayload, DlightRuntimeRequest};

pub async fn send(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    request: DlightRuntimeRequest,
) -> Result<SendResult, WalletError> {
    let record = preflight_store
        .take(preflight_id)
        .ok_or(WalletError::InvalidPreflight)?;

    let session = session_manager.lock().await;
    let active_id = session
        .active_account_id()
        .ok_or(WalletError::WalletLocked)?;
    if active_id.as_str() != record.account_id {
        return Err(WalletError::InvalidPreflight);
    }
    drop(session);

    let payload: DlightPreflightPayload =
        serde_json::from_value(record.payload).map_err(|_| WalletError::InvalidPreflight)?;

    let info = super::get_info(request).await?;
    if info.syncing {
        let percent = info.percent.unwrap_or(0.0);
        if (percent - 100.0).abs() > f64::EPSILON && (percent + 1.0).abs() > f64::EPSILON {
            return Err(WalletError::DlightSynchronizerNotReady);
        }
    }

    // The dlight scanner runtime currently does not persist full spend witnesses.
    // Keep routing/ownership semantics in place and return a safe error until the
    // dedicated spend engine lands.
    let _ = payload;
    Err(WalletError::OperationFailed)
}
