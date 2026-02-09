// 
// Type definition re-exports
// Last Updated: Added AccountRecord and AddressResponse exports for Module 1 & 2 integration

pub mod errors;
pub mod wallet;

pub use errors::WalletError;
pub use wallet::{
    WalletMetadata, 
    CreateWalletRequest, 
    CreateWalletResult, 
    GenerateMnemonicRequest, 
    MnemonicResult,
    AccountRecord,
    AddressResponse,
};
