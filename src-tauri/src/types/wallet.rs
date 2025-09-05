// 
// Wallet type definitions
// Security: Contains only non-sensitive metadata, never private keys
// Last Updated: Created for wallet creation flow implementation

use serde::{Deserialize, Serialize};

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

impl WalletMetadata {
    pub fn new(name: String) -> Self {
        Self {
            id: name.clone(),
            name,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            coin_types: vec!["verus".to_string()],
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
        if name.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')) {
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
