//
// Wallet type definitions
// Security: Contains only non-sensitive metadata, never private keys
// Last Updated: Added WalletListItem and ActiveWalletResponse for list/unlock and dashboard

use serde::{Deserialize, Serialize};
use zeroize::ZeroizeOnDrop;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WalletNetwork {
    Mainnet,
    Testnet,
}

impl Default for WalletNetwork {
    fn default() -> Self {
        WalletNetwork::Mainnet
    }
}

fn default_wallet_network() -> WalletNetwork {
    WalletNetwork::Mainnet
}

fn default_wallet_emoji() -> String {
    "💰".to_string()
}

fn default_wallet_color() -> String {
    "blue".to_string()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WalletMetadata {
    pub id: String,
    pub name: String,
    pub created_at: u64,
    pub coin_types: Vec<String>,
    pub version: u8,
}

#[derive(Serialize, Deserialize)]
pub struct KeyPair {
    pub public_key: String,
    pub address: String,
    // Note: Private key never leaves Stronghold vault!
}

#[derive(Serialize, Deserialize)]
pub struct CreateWalletRequest {
    pub wallet_name: String,
    pub seed_phrase: String,
    #[serde(default = "default_wallet_network")]
    pub network: WalletNetwork,
    #[serde(default = "default_wallet_emoji")]
    pub emoji: String,
    #[serde(default = "default_wallet_color")]
    pub color: String,
    // Note: No password field - will be handled separately for security
}

#[derive(Serialize, Deserialize)]
pub struct CreateWalletResult {
    pub wallet_id: String,
    pub success: bool,
}

#[derive(Serialize, Deserialize)]
pub struct GenerateMnemonicRequest {
    pub word_count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct MnemonicResult {
    pub seed_phrase: String,
}

/// Derived cryptographic keys for a wallet account
/// Security: Implements ZeroizeOnDrop to ensure keys are cleared from memory
#[derive(Clone, ZeroizeOnDrop)]
pub struct DerivedKeys {
    pub wif: String,
    pub address: String,
    pub pub_hex: String,
    pub eth_private_key: String,
    pub eth_address: String,
    pub btc_address: String,
}

/// Account metadata record stored in filesystem
#[derive(Serialize, Deserialize, Clone)]
pub struct AccountRecord {
    pub id: String,
    pub account_hash: String,
    pub key_derivation_version: u8,
    pub created_at: u64,
    #[serde(default = "default_wallet_network")]
    pub network: WalletNetwork,
    #[serde(default = "default_wallet_emoji")]
    pub emoji: String,
    #[serde(default = "default_wallet_color")]
    pub color: String,
}

/// Response containing derived addresses
#[derive(Serialize, Deserialize)]
pub struct AddressResponse {
    pub vrsc_address: String,
    pub eth_address: String,
    pub btc_address: String,
}

/// List item for wallet selection (unlock screen)
#[derive(Serialize, Deserialize, Clone)]
pub struct WalletListItem {
    pub account_id: String,
    pub wallet_name: String,
    pub network: WalletNetwork,
    pub emoji: String,
    pub color: String,
}

/// Active wallet display info for dashboard
#[derive(Serialize, Deserialize)]
pub struct ActiveWalletResponse {
    pub wallet_name: String,
    pub network: WalletNetwork,
    pub emoji: String,
    pub color: String,
}

impl WalletMetadata {
    pub fn new(name: String) -> Self {
        Self {
            id: name.clone(),
            name,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            coin_types: vec![
                "verus".to_string(),
                "ethereum".to_string(),
                "bitcoin".to_string(),
            ],
            version: 1,
        }
    }
}

impl CreateWalletRequest {
    pub fn validate(&self) -> Result<(), crate::types::errors::WalletError> {
        // Validate wallet name
        let name = self.wallet_name.trim();
        if name.is_empty() || name.len() > 50 {
            return Err(crate::types::errors::WalletError::InvalidWalletName);
        }

        // Check for filesystem-unsafe characters
        if name
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        {
            return Err(crate::types::errors::WalletError::InvalidWalletName);
        }

        // Validate seed phrase format (basic check)
        let words: Vec<&str> = self.seed_phrase.split_whitespace().collect();
        if words.len() != 24 {
            return Err(crate::types::errors::WalletError::InvalidSeedPhrase);
        }

        Ok(())
    }
}
