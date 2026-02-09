// 
// Wallet management core business logic with Stronghold integration
// Security: Handles sensitive key operations - all functions require proper validation
// Last Updated: Created for wallet creation flow implementation

use bip39::{Language, Mnemonic};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
// Removed unused imports

use crate::types::{WalletError, WalletMetadata, CreateWalletRequest};

// TODO: Stronghold record identifiers (for future integration)
// const SEED_RECORD_PATH: &[u8] = b"seed";
// const WALLET_METADATA_PATH: &[u8] = b"metadata";

pub struct WalletManager {
    data_directory: PathBuf,
}

impl WalletManager {
    pub fn new() -> Self {
        Self {
            data_directory: Self::get_data_directory().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
    
    /// Generate a new BIP39 mnemonic phrase
    pub async fn generate_mnemonic(&self, word_count: usize) -> Result<String, WalletError> {
        let entropy_bits = match word_count {
            12 => 16,
            15 => 20, 
            18 => 24,
            21 => 28,
            24 => 32,
            _ => return Err(WalletError::InvalidSeedPhrase),
        };
        
        println!("[WALLET] Generating {} word mnemonic", word_count);
        
        // Generate entropy
        let mut entropy = vec![0u8; entropy_bits];
        rand::RngCore::fill_bytes(&mut OsRng, &mut entropy);
        
        // Create mnemonic from entropy
        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
            .map_err(|e| {
                println!("[WALLET] Mnemonic generation failed: {}", e);
                WalletError::OperationFailed
            })?;
        
        println!("[WALLET] Mnemonic generation successful");
        Ok(mnemonic.to_string())
    }
    
    /// Validate a BIP39 mnemonic phrase
    pub async fn validate_mnemonic(&self, seed_phrase: &str) -> Result<bool, WalletError> {
        let is_valid = Mnemonic::parse_in(Language::English, seed_phrase).is_ok();
        Ok(is_valid)
    }
    
    /// Create a new wallet (simplified implementation for now)
    /// TODO: Implement full Stronghold integration using frontend API
    pub async fn create_wallet(
        &self, 
        request: &CreateWalletRequest, 
        _password: &str
    ) -> Result<String, WalletError> {
        // Validate seed phrase
        let _mnemonic = Mnemonic::parse_in(Language::English, &request.seed_phrase)
            .map_err(|e| {
                println!("[WALLET] Invalid seed phrase format: {}", e);
                WalletError::InvalidSeedPhrase
            })?;
        
        // Check if wallet already exists
        if self.wallet_exists(&request.wallet_name).await? {
            return Err(WalletError::WalletExists);
        }
        
        println!("[WALLET] Creating wallet: {}", request.wallet_name);
        
        // Create wallet metadata file (simplified for now)
        let metadata = WalletMetadata::new(request.wallet_name.clone());
        let metadata_path = self.get_metadata_path(&request.wallet_name)?;
        
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(metadata_path, metadata_json).map_err(|e| {
            println!("[WALLET] Failed to write metadata: {}", e);
            WalletError::OperationFailed
        })?;
        
        println!("[WALLET] Wallet created successfully (simplified)");
        println!("[WALLET] TODO: Integrate Stronghold for secure seed storage");
        
        Ok(request.wallet_name.clone())
    }
    
    /// List available wallet metadata files
    pub async fn list_wallets(&self) -> Result<Vec<String>, WalletError> {
        let mut wallets = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(&self.data_directory) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with("_metadata.json") {
                            wallets.push(name.trim_end_matches("_metadata.json").to_string());
                        }
                    }
                }
            }
        }
        
        Ok(wallets)
    }
    
    // Private helper methods    
    async fn wallet_exists(&self, wallet_name: &str) -> Result<bool, WalletError> {
        let metadata_path = self.get_metadata_path(wallet_name)?;
        Ok(metadata_path.exists())
    }
    
    pub fn get_metadata_path(&self, wallet_name: &str) -> Result<PathBuf, WalletError> {
        Ok(self.data_directory.join(format!("{}_metadata.json", wallet_name)))
    }
    
    #[allow(dead_code)]
    fn hash_password(password: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.finalize().to_vec()
    }
    
    fn get_data_directory() -> Result<PathBuf, WalletError> {
        // Use simple directory structure for now
        let app_dir = std::env::current_dir()
            .map_err(|_| WalletError::OperationFailed)?
            .join("wallet_data");
        
        std::fs::create_dir_all(&app_dir).map_err(|e| {
            println!("[WALLET] Failed to create data directory: {}", e);
            WalletError::OperationFailed
        })?;
        
        Ok(app_dir)
    }
}
