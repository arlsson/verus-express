//
// Stronghold-backed secure seed storage using IOTA Stronghold vault
// Security: Seeds encrypted at rest via Stronghold snapshot; one vault per account (per password)
// Last Updated: Replaced XOR with iota_stronghold; unified app data dir with WalletManager

use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use zeroize::Zeroizing;

use crate::types::errors::WalletError;
use iota_stronghold::{KeyProvider, SnapshotPath, Stronghold};

const SEED_RECORD_KEY: &[u8] = b"seed";
const ADDRESS_BOOK_RECORD_KEY: &[u8] = b"address_book_v1";

#[derive(Clone)]
pub struct StrongholdStore {
    /// Base directory for Stronghold (app data dir / stronghold); accounts go under accounts/<id>/
    base_path: PathBuf,
}

impl StrongholdStore {
    /// Initialize with app data directory (same root as wallet metadata for unified storage).
    pub fn new(app_handle: &AppHandle) -> Result<Self, WalletError> {
        let app_dir = app_handle.path().app_data_dir().map_err(|e| {
            println!("[AUTH] Failed to get app data directory: {}", e);
            WalletError::OperationFailed
        })?;
        let base_path = app_dir.join("stronghold");
        std::fs::create_dir_all(&base_path).map_err(|e| {
            println!("[AUTH] Failed to create Stronghold directory: {}", e);
            WalletError::OperationFailed
        })?;
        Ok(Self { base_path })
    }

    fn account_snapshot_path(&self, account_id: &str) -> PathBuf {
        let account_dir = self.base_path.join("accounts").join(account_id);
        let _ = std::fs::create_dir_all(&account_dir);
        account_dir.join("snapshot.stronghold")
    }

    fn address_book_snapshot_path(&self, account_id: &str) -> PathBuf {
        let account_dir = self.base_path.join("accounts").join(account_id);
        let _ = std::fs::create_dir_all(&account_dir);
        account_dir.join("address_book.snapshot.stronghold")
    }

    /// Password hash for Stronghold key (must match plugin / KDF used at unlock).
    pub(crate) fn hash_password(password: &str) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.finalize().to_vec()
    }

    fn keyprovider_from_hash(password_hash: &[u8]) -> Result<KeyProvider, WalletError> {
        KeyProvider::try_from(Zeroizing::new(password_hash.to_vec())).map_err(|e| {
            println!("[AUTH] KeyProvider failed: {}", e);
            WalletError::OperationFailed
        })
    }

    fn get_or_create_client(
        stronghold: &Stronghold,
        snapshot_path: &SnapshotPath,
        account_id: &str,
        keyprovider: &KeyProvider,
        snapshot_exists: bool,
    ) -> Result<iota_stronghold::Client, WalletError> {
        if snapshot_exists {
            stronghold
                .load_client_from_snapshot(account_id.as_bytes(), keyprovider, snapshot_path)
                .map_err(|e| {
                    println!("[AUTH] Load client from snapshot failed: {}", e);
                    WalletError::InvalidPassword
                })
        } else {
            stronghold
                .create_client(account_id.as_bytes())
                .map_err(|e| {
                    println!("[AUTH] Create client failed: {}", e);
                    WalletError::OperationFailed
                })
        }
    }

    /// Store seed in a Stronghold vault for this account (encrypted with password).
    pub async fn store_seed(
        &self,
        account_id: &str,
        seed: &str,
        password: &str,
        _app_handle: &AppHandle,
    ) -> Result<(), WalletError> {
        println!("[AUTH] Storing seed for account: {}", account_id);

        let path = self.account_snapshot_path(account_id);
        let password_hash = Self::hash_password(password);
        let keyprovider = Self::keyprovider_from_hash(&password_hash)?;
        let snapshot_path = SnapshotPath::from_path(&path);

        let stronghold = Stronghold::default();
        let client = Self::get_or_create_client(
            &stronghold,
            &snapshot_path,
            account_id,
            &keyprovider,
            path.exists(),
        )?;
        client
            .store()
            .insert(SEED_RECORD_KEY.to_vec(), seed.as_bytes().to_vec(), None)
            .map_err(|e| {
                println!("[AUTH] Store insert failed: {}", e);
                WalletError::OperationFailed
            })?;
        stronghold
            .commit_with_keyprovider(&snapshot_path, &keyprovider)
            .map_err(|e| {
                println!("[AUTH] Commit failed: {}", e);
                WalletError::OperationFailed
            })?;

        println!("[AUTH] Seed stored successfully");
        Ok(())
    }

    /// Load seed from Stronghold vault (wrong password returns InvalidPassword).
    pub async fn load_seed(
        &self,
        account_id: &str,
        password: &str,
        _app_handle: &AppHandle,
    ) -> Result<String, WalletError> {
        println!("[AUTH] Loading seed for account: {}", account_id);

        let path = self.account_snapshot_path(account_id);
        if !path.exists() {
            println!(
                "[AUTH] Stronghold snapshot missing for account: {}",
                account_id
            );
            return Err(WalletError::InvalidPassword);
        }

        let password_hash = Self::hash_password(password);
        let keyprovider = Self::keyprovider_from_hash(&password_hash)?;
        let snapshot_path = SnapshotPath::from_path(&path);

        let stronghold = Stronghold::default();
        let client = stronghold
            .load_client_from_snapshot(account_id.as_bytes(), &keyprovider, &snapshot_path)
            .map_err(|e| {
                println!("[AUTH] Load client from snapshot failed: {}", e);
                WalletError::InvalidPassword
            })?;
        let data = client.store().get(SEED_RECORD_KEY).map_err(|e| {
            println!("[AUTH] Store get failed: {}", e);
            WalletError::OperationFailed
        })?;
        let bytes = data.ok_or_else(|| {
            println!("[AUTH] Seed record missing for account: {}", account_id);
            WalletError::InvalidPassword
        })?;
        let seed = String::from_utf8(bytes).map_err(|_| WalletError::OperationFailed)?;

        #[cfg(debug_assertions)]
        {
            // Re-commit after load so existing debug snapshots migrate to the current
            // (lower) debug work factor for faster subsequent unlocks.
            if let Err(e) = stronghold.commit_with_keyprovider(&snapshot_path, &keyprovider) {
                println!("[AUTH] Debug snapshot re-commit failed: {}", e);
            }
        }

        println!("[AUTH] Seed loaded successfully");
        Ok(seed)
    }

    /// Store address book snapshot for an account in an isolated Stronghold snapshot.
    pub async fn store_address_book(
        &self,
        account_id: &str,
        password_hash: &[u8],
        data: &[u8],
    ) -> Result<(), WalletError> {
        let path = self.address_book_snapshot_path(account_id);
        let snapshot_path = SnapshotPath::from_path(&path);
        let keyprovider = Self::keyprovider_from_hash(password_hash)?;

        let stronghold = Stronghold::default();
        let client = Self::get_or_create_client(
            &stronghold,
            &snapshot_path,
            account_id,
            &keyprovider,
            path.exists(),
        )?;

        client
            .store()
            .insert(ADDRESS_BOOK_RECORD_KEY.to_vec(), data.to_vec(), None)
            .map_err(|e| {
                println!("[AUTH] Store address book failed: {}", e);
                WalletError::OperationFailed
            })?;

        stronghold
            .commit_with_keyprovider(&snapshot_path, &keyprovider)
            .map_err(|e| {
                println!("[AUTH] Commit address book snapshot failed: {}", e);
                WalletError::OperationFailed
            })?;

        Ok(())
    }

    /// Load address book snapshot bytes for an account from isolated Stronghold snapshot.
    pub async fn load_address_book(
        &self,
        account_id: &str,
        password_hash: &[u8],
    ) -> Result<Option<Vec<u8>>, WalletError> {
        let path = self.address_book_snapshot_path(account_id);
        if !path.exists() {
            return Ok(None);
        }

        let snapshot_path = SnapshotPath::from_path(&path);
        let keyprovider = Self::keyprovider_from_hash(password_hash)?;
        let stronghold = Stronghold::default();
        let client = stronghold
            .load_client_from_snapshot(account_id.as_bytes(), &keyprovider, &snapshot_path)
            .map_err(|e| {
                println!("[AUTH] Load address book snapshot failed: {}", e);
                WalletError::OperationFailed
            })?;

        client.store().get(ADDRESS_BOOK_RECORD_KEY).map_err(|e| {
            println!("[AUTH] Read address book record failed: {}", e);
            WalletError::OperationFailed
        })
    }
}
