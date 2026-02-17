//
// Type definition re-exports
// Last Updated: Added transaction module and Module 8 preflight/send types

pub mod address_book;
pub mod bridge;
pub mod errors;
pub mod guard;
pub mod identity;
pub mod transaction;
pub mod vrpc_transfer;
pub mod wallet;

pub use address_book::{
    AddressBookContact, AddressBookEndpoint, AddressBookSnapshot, AddressEndpointKind,
    SaveAddressBookContactRequest, SaveAddressBookEndpointInput, ValidateDestinationAddressRequest,
    ValidateDestinationAddressResult,
};
pub use bridge::{
    BridgeCapabilitiesRequest, BridgeCapabilitiesResult, BridgeConversionEstimateRequest,
    BridgeConversionEstimateResult, BridgeConversionPathQuote, BridgeConversionPathRequest,
    BridgeConversionPathsResult, BridgeExecutionHint, BridgeTransferPreflightParams,
    BridgeTransferPreflightResult, BridgeTransferRoute,
};
pub use errors::WalletError;
pub use guard::{
    BeginGuardSessionRequest, BeginGuardSessionResult, EndGuardSessionRequest,
    EndGuardSessionResult, GuardIdentityLookupRequest, GuardIdentityLookupResult,
    GuardIdentityPreflightRequest, GuardIdentitySendRequest, GuardImportMode, GuardPreflightResult,
    GuardSendResult,
};
pub use identity::{
    HighRiskChange, IdentityOperation, IdentityPatch, IdentityPreflightParams,
    IdentityPreflightResult, IdentitySendRequest, IdentitySendResult, IdentityWarning,
};
pub use transaction::{
    BalanceResult, PreflightParams, PreflightResult, PreflightWarning, SendRequest, SendResult,
    Transaction,
};
pub use vrpc_transfer::{VrpcTransferPreflightParams, VrpcTransferPreflightResult};
pub use wallet::{
    AccountRecord, ActiveAssetsState, ActiveWalletResponse, AddressResponse, CoinScope,
    CoinScopesResult, CreateWalletRequest, CreateWalletResult, GenerateMnemonicRequest,
    ImportWalletTextRequest, MnemonicResult, WalletListItem, WalletMetadata, WalletSecretKind,
};
