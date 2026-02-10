//
// Wallet management core business logic with Stronghold integration
// Security: Handles sensitive key operations - all functions require proper validation
// Last Updated: data_directory now set from app data dir (unified with Stronghold storage)

use bip39::{Language, Mnemonic};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::types::{
    AccountRecord, CreateWalletRequest, WalletError, WalletListItem, WalletMetadata,
};

pub struct WalletManager {
    data_directory: PathBuf,
}

impl WalletManager {
    /// Create wallet manager with an explicit data directory (e.g. app data dir).
    /// Call from setup with `app_handle.path().app_data_dir().join("wallet_data")` so metadata
    /// and Stronghold vaults share the same root.
    pub fn new(data_directory: PathBuf) -> Self {
        Self { data_directory }
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
        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy).map_err(|e| {
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
        _password: &str,
    ) -> Result<String, WalletError> {
        // Validate seed phrase
        let _mnemonic =
            Mnemonic::parse_in(Language::English, &request.seed_phrase).map_err(|e| {
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

    /// List available wallets with account_id for unlock flow
    pub async fn list_wallets(&self) -> Result<Vec<WalletListItem>, WalletError> {
        let mut wallets = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.data_directory) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with("_metadata.json") {
                            let wallet_name = name.trim_end_matches("_metadata.json").to_string();
                            let path = self.data_directory.join(name);
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(account) = serde_json::from_str::<AccountRecord>(&content)
                                {
                                    wallets.push(WalletListItem {
                                        account_id: account.id,
                                        wallet_name,
                                        network: account.network,
                                        emoji: account.emoji,
                                        color: account.color,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(wallets)
    }

    /// Resolve wallet display name for an account_id (for active wallet display)
    pub async fn get_wallet_name_by_account_id(
        &self,
        account_id: &str,
    ) -> Result<Option<String>, WalletError> {
        let item = self.get_wallet_by_account_id(account_id).await?;
        Ok(item.map(|w| w.wallet_name))
    }

    /// Resolve wallet list item for account_id (name + network).
    pub async fn get_wallet_by_account_id(
        &self,
        account_id: &str,
    ) -> Result<Option<WalletListItem>, WalletError> {
        let list = self.list_wallets().await?;
        Ok(list.into_iter().find(|w| w.account_id == account_id))
    }

    /// Returns true if a wallet with the given name already exists (used to prevent duplicate names).
    pub async fn wallet_exists(&self, wallet_name: &str) -> Result<bool, WalletError> {
        let metadata_path = self.get_metadata_path(wallet_name)?;
        Ok(metadata_path.exists())
    }

    pub fn get_metadata_path(&self, wallet_name: &str) -> Result<PathBuf, WalletError> {
        std::fs::create_dir_all(&self.data_directory).map_err(|e| {
            println!("[WALLET] Failed to create data directory: {}", e);
            WalletError::OperationFailed
        })?;
        Ok(self
            .data_directory
            .join(format!("{}_metadata.json", wallet_name)))
    }

    #[allow(dead_code)]
    fn hash_password(password: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.finalize().to_vec()
    }
}
