//
// Stronghold-backed secure seed storage using IOTA Stronghold vault
// Security: Seeds encrypted at rest via Stronghold snapshot; one vault per account (per password)
// Last Updated: Replaced XOR with iota_stronghold; unified app data dir with WalletManager

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use zeroize::Zeroizing;

use crate::types::errors::WalletError;
use crate::types::wallet::WalletNetwork;
use iota_stronghold::{KeyProvider, SnapshotPath, Stronghold};

const SEED_RECORD_KEY: &[u8] = b"seed";
const ADDRESS_BOOK_RECORD_KEY: &[u8] = b"address_book_v1";
const WATCHED_VRPC_ADDRESSES_RECORD_KEY: &[u8] = b"watched_vrpc_addresses_v1";
const WATCHED_VRPC_ADDRESSES_SCHEMA_VERSION: u8 = 1;
const ACTIVE_ASSETS_RECORD_KEY: &[u8] = b"active_assets_v1";
const ACTIVE_ASSETS_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WatchedVrpcAddressesSnapshot {
    schema_version: u8,
    mainnet: Vec<String>,
    testnet: Vec<String>,
}

impl Default for WatchedVrpcAddressesSnapshot {
    fn default() -> Self {
        Self {
            schema_version: WATCHED_VRPC_ADDRESSES_SCHEMA_VERSION,
            mainnet: vec![],
            testnet: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActiveAssetsNetworkSnapshot {
    #[serde(default)]
    initialized: bool,
    #[serde(default)]
    coin_ids: Vec<String>,
}

impl Default for ActiveAssetsNetworkSnapshot {
    fn default() -> Self {
        Self {
            initialized: false,
            coin_ids: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActiveAssetsSnapshot {
    schema_version: u8,
    #[serde(default)]
    mainnet: ActiveAssetsNetworkSnapshot,
    #[serde(default)]
    testnet: ActiveAssetsNetworkSnapshot,
}

impl Default for ActiveAssetsSnapshot {
    fn default() -> Self {
        Self {
            schema_version: ACTIVE_ASSETS_SCHEMA_VERSION,
            mainnet: ActiveAssetsNetworkSnapshot::default(),
            testnet: ActiveAssetsNetworkSnapshot::default(),
        }
    }
}

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

    fn watched_vrpc_addresses_snapshot_path(&self, account_id: &str) -> PathBuf {
        let account_dir = self.base_path.join("accounts").join(account_id);
        let _ = std::fs::create_dir_all(&account_dir);
        account_dir.join("watched_vrpc_addresses.snapshot.stronghold")
    }

    fn active_assets_snapshot_path(&self, account_id: &str) -> PathBuf {
        let account_dir = self.base_path.join("accounts").join(account_id);
        let _ = std::fs::create_dir_all(&account_dir);
        account_dir.join("active_assets.snapshot.stronghold")
    }

    async fn load_watched_vrpc_addresses_snapshot(
        &self,
        account_id: &str,
        password_hash: &[u8],
    ) -> Result<WatchedVrpcAddressesSnapshot, WalletError> {
        let path = self.watched_vrpc_addresses_snapshot_path(account_id);
        if !path.exists() {
            return Ok(WatchedVrpcAddressesSnapshot::default());
        }

        let snapshot_path = SnapshotPath::from_path(&path);
        let keyprovider = Self::keyprovider_from_hash(password_hash)?;
        let stronghold = Stronghold::default();
        let client = stronghold
            .load_client_from_snapshot(account_id.as_bytes(), &keyprovider, &snapshot_path)
            .map_err(|e| {
                println!("[AUTH] Load watched VRPC addresses snapshot failed: {}", e);
                WalletError::OperationFailed
            })?;

        let bytes = client
            .store()
            .get(WATCHED_VRPC_ADDRESSES_RECORD_KEY)
            .map_err(|e| {
                println!("[AUTH] Read watched VRPC addresses record failed: {}", e);
                WalletError::OperationFailed
            })?;

        let Some(payload) = bytes else {
            return Ok(WatchedVrpcAddressesSnapshot::default());
        };

        serde_json::from_slice::<WatchedVrpcAddressesSnapshot>(&payload).map_err(|e| {
            println!("[AUTH] Parse watched VRPC addresses snapshot failed: {}", e);
            WalletError::OperationFailed
        })
    }

    async fn load_active_assets_snapshot(
        &self,
        account_id: &str,
        password_hash: &[u8],
    ) -> Result<ActiveAssetsSnapshot, WalletError> {
        let path = self.active_assets_snapshot_path(account_id);
        if !path.exists() {
            return Ok(ActiveAssetsSnapshot::default());
        }

        let snapshot_path = SnapshotPath::from_path(&path);
        let keyprovider = Self::keyprovider_from_hash(password_hash)?;
        let stronghold = Stronghold::default();
        let client = stronghold
            .load_client_from_snapshot(account_id.as_bytes(), &keyprovider, &snapshot_path)
            .map_err(|e| {
                println!("[AUTH] Load active assets snapshot failed: {}", e);
                WalletError::OperationFailed
            })?;

        let bytes = client.store().get(ACTIVE_ASSETS_RECORD_KEY).map_err(|e| {
            println!("[AUTH] Read active assets record failed: {}", e);
            WalletError::OperationFailed
        })?;

        let Some(payload) = bytes else {
            return Ok(ActiveAssetsSnapshot::default());
        };

        serde_json::from_slice::<ActiveAssetsSnapshot>(&payload).map_err(|e| {
            println!("[AUTH] Parse active assets snapshot failed: {}", e);
            WalletError::OperationFailed
        })
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

    pub async fn load_watched_vrpc_addresses(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
    ) -> Result<Vec<String>, WalletError> {
        let snapshot = self
            .load_watched_vrpc_addresses_snapshot(account_id, password_hash)
            .await?;

        Ok(match network {
            WalletNetwork::Mainnet => snapshot.mainnet,
            WalletNetwork::Testnet => snapshot.testnet,
        })
    }

    pub async fn store_watched_vrpc_addresses(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
        addresses: &[String],
    ) -> Result<(), WalletError> {
        let path = self.watched_vrpc_addresses_snapshot_path(account_id);
        let snapshot_path = SnapshotPath::from_path(&path);
        let keyprovider = Self::keyprovider_from_hash(password_hash)?;

        let mut snapshot = self
            .load_watched_vrpc_addresses_snapshot(account_id, password_hash)
            .await?;
        match network {
            WalletNetwork::Mainnet => {
                snapshot.mainnet = addresses.to_vec();
            }
            WalletNetwork::Testnet => {
                snapshot.testnet = addresses.to_vec();
            }
        }

        let stronghold = Stronghold::default();
        let client = Self::get_or_create_client(
            &stronghold,
            &snapshot_path,
            account_id,
            &keyprovider,
            path.exists(),
        )?;
        let payload = serde_json::to_vec(&snapshot).map_err(|_| WalletError::OperationFailed)?;
        client
            .store()
            .insert(
                WATCHED_VRPC_ADDRESSES_RECORD_KEY.to_vec(),
                payload.to_vec(),
                None,
            )
            .map_err(|e| {
                println!("[AUTH] Store watched VRPC addresses failed: {}", e);
                WalletError::OperationFailed
            })?;
        stronghold
            .commit_with_keyprovider(&snapshot_path, &keyprovider)
            .map_err(|e| {
                println!(
                    "[AUTH] Commit watched VRPC addresses snapshot failed: {}",
                    e
                );
                WalletError::OperationFailed
            })?;

        Ok(())
    }

    pub async fn load_active_assets(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
    ) -> Result<(bool, Vec<String>), WalletError> {
        let snapshot = self
            .load_active_assets_snapshot(account_id, password_hash)
            .await?;

        match network {
            WalletNetwork::Mainnet => Ok((snapshot.mainnet.initialized, snapshot.mainnet.coin_ids)),
            WalletNetwork::Testnet => Ok((snapshot.testnet.initialized, snapshot.testnet.coin_ids)),
        }
    }

    pub async fn store_active_assets(
        &self,
        account_id: &str,
        password_hash: &[u8],
        network: WalletNetwork,
        initialized: bool,
        coin_ids: &[String],
    ) -> Result<(), WalletError> {
        let path = self.active_assets_snapshot_path(account_id);
        let snapshot_path = SnapshotPath::from_path(&path);
        let keyprovider = Self::keyprovider_from_hash(password_hash)?;

        let mut snapshot = self
            .load_active_assets_snapshot(account_id, password_hash)
            .await?;
        snapshot.schema_version = ACTIVE_ASSETS_SCHEMA_VERSION;

        let network_snapshot = match network {
            WalletNetwork::Mainnet => &mut snapshot.mainnet,
            WalletNetwork::Testnet => &mut snapshot.testnet,
        };
        network_snapshot.initialized = initialized;
        network_snapshot.coin_ids = coin_ids.to_vec();

        let stronghold = Stronghold::default();
        let client = Self::get_or_create_client(
            &stronghold,
            &snapshot_path,
            account_id,
            &keyprovider,
            path.exists(),
        )?;
        let payload = serde_json::to_vec(&snapshot).map_err(|_| WalletError::OperationFailed)?;
        client
            .store()
            .insert(ACTIVE_ASSETS_RECORD_KEY.to_vec(), payload, None)
            .map_err(|e| {
                println!("[AUTH] Store active assets failed: {}", e);
                WalletError::OperationFailed
            })?;
        stronghold
            .commit_with_keyprovider(&snapshot_path, &keyprovider)
            .map_err(|e| {
                println!("[AUTH] Commit active assets snapshot failed: {}", e);
                WalletError::OperationFailed
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::StrongholdStore;
    use crate::types::wallet::WalletNetwork;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_store() -> StrongholdStore {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let base_path =
            std::env::temp_dir().join(format!("lite_wallet_stronghold_store_{}", unique));
        StrongholdStore { base_path }
    }

    #[tokio::test]
    async fn active_assets_round_trip_preserves_network_and_initialized_flag() {
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);

        let store = temp_store();
        let account_id = "account_roundtrip";
        let password_hash = StrongholdStore::hash_password("test-password");

        store
            .store_active_assets(
                account_id,
                password_hash.as_slice(),
                WalletNetwork::Mainnet,
                true,
                &["VRSC".to_string(), "vUSDC".to_string()],
            )
            .await
            .expect("store mainnet active assets");

        store
            .store_active_assets(
                account_id,
                password_hash.as_slice(),
                WalletNetwork::Testnet,
                false,
                &["VRSCTEST".to_string()],
            )
            .await
            .expect("store testnet active assets");

        let mainnet = store
            .load_active_assets(account_id, password_hash.as_slice(), WalletNetwork::Mainnet)
            .await
            .expect("load mainnet");
        assert_eq!(mainnet.0, true);
        assert_eq!(mainnet.1, vec!["VRSC".to_string(), "vUSDC".to_string()]);

        let testnet = store
            .load_active_assets(account_id, password_hash.as_slice(), WalletNetwork::Testnet)
            .await
            .expect("load testnet");
        assert_eq!(testnet.0, false);
        assert_eq!(testnet.1, vec!["VRSCTEST".to_string()]);

        let _ = std::fs::remove_dir_all(store.base_path);
    }
}
