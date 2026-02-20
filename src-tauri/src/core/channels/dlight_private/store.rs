use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

pub const STORAGE_SCHEMA_VERSION: u8 = 2;

#[derive(Debug, Clone)]
pub struct DlightStoragePaths {
    pub root: PathBuf,
    pub data_db: PathBuf,
    pub blockmeta_db: PathBuf,
    pub fs_cache: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredNote {
    pub nullifier_hex: String,
    pub value_sats: u64,
    pub received_height: u64,
    pub spent_height: Option<u64>,
    pub received_txid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredTransaction {
    pub txid: String,
    pub net_sats: i128,
    pub block_height: u64,
    pub block_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredRuntimeState {
    pub schema_version: u8,
    pub scanned_height: u64,
    pub sapling_tree_size: Option<u32>,
    pub block_hash_hex: Option<String>,
    pub notes: Vec<StoredNote>,
    pub transactions: Vec<StoredTransaction>,
    pub last_updated: u64,
}

impl Default for StoredRuntimeState {
    fn default() -> Self {
        Self {
            schema_version: STORAGE_SCHEMA_VERSION,
            scanned_height: 0,
            sapling_tree_size: None,
            block_hash_hex: None,
            notes: vec![],
            transactions: vec![],
            last_updated: unix_timestamp_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredBlockMeta {
    pub schema_version: u8,
    pub scanned_height: u64,
    pub sapling_tree_size: Option<u32>,
    pub block_hash_hex: Option<String>,
    pub last_updated: u64,
}

impl Default for StoredBlockMeta {
    fn default() -> Self {
        Self {
            schema_version: STORAGE_SCHEMA_VERSION,
            scanned_height: 0,
            sapling_tree_size: None,
            block_hash_hex: None,
            last_updated: unix_timestamp_secs(),
        }
    }
}

pub fn resolve_paths(
    app_data_dir: &Path,
    network: WalletNetwork,
    account_hash: &str,
    coin_id: &str,
) -> DlightStoragePaths {
    let network_key = match network {
        WalletNetwork::Mainnet => "mainnet",
        WalletNetwork::Testnet => "testnet",
    };

    let root = app_data_dir
        .join("dlight")
        .join(network_key)
        .join(account_hash)
        .join(coin_id);

    DlightStoragePaths {
        data_db: root.join("data.db"),
        blockmeta_db: root.join("blockmeta.db"),
        fs_cache: root.join("fs_cache"),
        root,
    }
}

pub fn ensure_layout(paths: &DlightStoragePaths) -> Result<(), WalletError> {
    fs::create_dir_all(&paths.root).map_err(|_| WalletError::OperationFailed)?;
    fs::create_dir_all(&paths.fs_cache).map_err(|_| WalletError::OperationFailed)?;

    if !paths.data_db.exists() {
        save_state(paths, &StoredRuntimeState::default())?;
    }
    if !paths.blockmeta_db.exists() {
        save_block_meta(paths, &StoredBlockMeta::default())?;
    }

    Ok(())
}

pub fn load_state(paths: &DlightStoragePaths) -> Result<StoredRuntimeState, WalletError> {
    if !paths.data_db.exists() {
        return Ok(StoredRuntimeState::default());
    }

    let bytes = fs::read(&paths.data_db).map_err(|_| WalletError::OperationFailed)?;
    let mut state: StoredRuntimeState =
        serde_json::from_slice(&bytes).map_err(|_| WalletError::OperationFailed)?;
    if state.schema_version != STORAGE_SCHEMA_VERSION {
        state = StoredRuntimeState::default();
    }
    Ok(state)
}

pub fn save_state(
    paths: &DlightStoragePaths,
    state: &StoredRuntimeState,
) -> Result<(), WalletError> {
    let payload = serde_json::to_vec_pretty(state).map_err(|_| WalletError::OperationFailed)?;
    fs::write(&paths.data_db, payload).map_err(|_| WalletError::OperationFailed)
}

pub fn load_block_meta(paths: &DlightStoragePaths) -> Result<StoredBlockMeta, WalletError> {
    if !paths.blockmeta_db.exists() {
        return Ok(StoredBlockMeta::default());
    }

    let bytes = fs::read(&paths.blockmeta_db).map_err(|_| WalletError::OperationFailed)?;
    let mut meta: StoredBlockMeta =
        serde_json::from_slice(&bytes).map_err(|_| WalletError::OperationFailed)?;
    if meta.schema_version != STORAGE_SCHEMA_VERSION {
        meta = StoredBlockMeta::default();
    }
    Ok(meta)
}

pub fn save_block_meta(
    paths: &DlightStoragePaths,
    meta: &StoredBlockMeta,
) -> Result<(), WalletError> {
    let payload = serde_json::to_vec_pretty(meta).map_err(|_| WalletError::OperationFailed)?;
    fs::write(&paths.blockmeta_db, payload).map_err(|_| WalletError::OperationFailed)
}

pub fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}
