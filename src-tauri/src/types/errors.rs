//
// Error type definitions for wallet operations
// Security: Never expose internal implementation details to frontend
// Last Updated: Added InvalidPreflight and UnsupportedChannel for Module 8

use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "type", content = "message")]
pub enum WalletError {
    #[error("Invalid wallet address")]
    InvalidAddress,

    #[error("Invalid seed phrase")]
    InvalidSeedPhrase,

    #[error("Invalid import text")]
    InvalidImportText,

    #[error("Invalid wallet name")]
    InvalidWalletName,

    #[error("Wallet already exists")]
    WalletExists,

    #[error("Wallet is locked")]
    WalletLocked,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Password must be at least 7 characters")]
    PasswordTooShort,

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Network error")]
    NetworkError,

    #[error("Operation failed")]
    OperationFailed,

    #[error("Invalid coin definition")]
    InvalidCoinDefinition,

    #[error("PBaaS currency already exists")]
    DuplicatePbaasCurrency,

    #[error("Invalid or expired preflight")]
    InvalidPreflight,

    #[error("Unsupported channel")]
    UnsupportedChannel,

    #[error("Identity not found")]
    IdentityNotFound,

    #[error("Identity has an invalid state for this operation")]
    IdentityInvalidState,

    #[error("Identity authority is unsupported")]
    IdentityUnsupportedAuthority,

    #[error("Identity request expired")]
    IdentityRequestExpired,

    #[error("Identity RPC method unsupported by endpoint")]
    IdentityRpcUnsupported,

    #[error("Failed to build identity transaction")]
    IdentityBuildFailed,

    #[error("Failed to sign identity transaction")]
    IdentitySignFailed,

    #[error("Guard session not found or expired")]
    GuardSessionNotFound,

    // Internal errors are mapped to generic ones above
    #[serde(skip)]
    #[error("Internal error: {0}")]
    Internal(String),
}

// Convert internal errors to user-safe errors
impl From<std::io::Error> for WalletError {
    fn from(_: std::io::Error) -> Self {
        WalletError::OperationFailed
    }
}

impl From<serde_json::Error> for WalletError {
    fn from(_: serde_json::Error) -> Self {
        WalletError::OperationFailed
    }
}
