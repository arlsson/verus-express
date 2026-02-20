use async_trait::async_trait;

use crate::types::transaction::{BalanceResult, Transaction};
use crate::types::WalletError;

use super::reader;
use super::{DlightInfo, DlightRuntimeDiagnostics, DlightRuntimeRequest};

#[async_trait]
pub trait DlightSynchronizerAdapter: Send + Sync {
    async fn get_balances(
        &self,
        request: &DlightRuntimeRequest,
    ) -> Result<BalanceResult, WalletError>;

    async fn get_transactions(
        &self,
        request: &DlightRuntimeRequest,
    ) -> Result<Vec<Transaction>, WalletError>;

    async fn get_info(&self, request: &DlightRuntimeRequest) -> Result<DlightInfo, WalletError>;

    async fn get_runtime_diagnostics(
        &self,
        request: &DlightRuntimeRequest,
    ) -> Result<DlightRuntimeDiagnostics, WalletError>;
}

#[derive(Debug, Default)]
pub struct DlightSynchronizerRuntimeAdapter;

#[async_trait]
impl DlightSynchronizerAdapter for DlightSynchronizerRuntimeAdapter {
    async fn get_balances(
        &self,
        request: &DlightRuntimeRequest,
    ) -> Result<BalanceResult, WalletError> {
        reader::get_balances(request).await
    }

    async fn get_transactions(
        &self,
        request: &DlightRuntimeRequest,
    ) -> Result<Vec<Transaction>, WalletError> {
        reader::get_transactions(request).await
    }

    async fn get_info(&self, request: &DlightRuntimeRequest) -> Result<DlightInfo, WalletError> {
        reader::get_info(request).await
    }

    async fn get_runtime_diagnostics(
        &self,
        request: &DlightRuntimeRequest,
    ) -> Result<DlightRuntimeDiagnostics, WalletError> {
        reader::get_runtime_diagnostics(request).await
    }
}
