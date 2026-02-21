use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

use super::store::{resolve_paths, unix_timestamp_secs};

pub const SPEND_SCHEMA_VERSION: u8 = 3;
const LEGACY_SPEND_SCHEMA_V2: u8 = 2;

#[derive(Debug, Clone)]
pub struct SpendStoragePaths {
    pub root: PathBuf,
    pub wallet_db: PathBuf,
    pub blocks_dir: PathBuf,
    pub meta_json: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredSpendScope {
    External,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredRseedKind {
    BeforeZip212,
    AfterZip212,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredRseed {
    pub kind: StoredRseedKind,
    pub bytes_hex: String,
}

impl Default for StoredRseed {
    fn default() -> Self {
        Self {
            kind: StoredRseedKind::AfterZip212,
            bytes_hex: "00".repeat(32),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoredSpendTree {
    pub left: Option<String>,
    pub right: Option<String>,
    pub parents: Vec<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoredSpendWitness {
    pub tree: StoredSpendTree,
    pub filled: Vec<String>,
    pub cursor: Option<StoredSpendTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredSpendNote {
    pub nullifier_hex: String,
    pub value_sats: u64,
    pub received_height: u64,
    pub spent_height: Option<u64>,
    pub note_position: u64,
    pub txid: String,
    pub scope: StoredSpendScope,
    #[serde(default)]
    pub recipient_bytes_hex: String,
    #[serde(default)]
    pub rseed: StoredRseed,
    #[serde(default)]
    pub witness: StoredSpendWitness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredSpendWalletDb {
    pub schema_version: u8,
    pub scanned_height: u64,
    pub chain_tip_height: u64,
    pub confirmed_balance_sats: u64,
    #[serde(default)]
    pub tree: StoredSpendTree,
    pub notes: Vec<StoredSpendNote>,
    pub last_updated: u64,
}

impl Default for StoredSpendWalletDb {
    fn default() -> Self {
        Self {
            schema_version: SPEND_SCHEMA_VERSION,
            scanned_height: 0,
            chain_tip_height: 0,
            confirmed_balance_sats: 0,
            tree: StoredSpendTree::default(),
            notes: vec![],
            last_updated: unix_timestamp_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredSpendMeta {
    pub schema_version: u8,
    pub last_successful_sync_height: u64,
    pub last_successful_tip_height: u64,
    pub last_successful_sync_at: u64,
    #[serde(default)]
    pub last_error: Option<String>,
}

impl Default for StoredSpendMeta {
    fn default() -> Self {
        Self {
            schema_version: SPEND_SCHEMA_VERSION,
            last_successful_sync_height: 0,
            last_successful_tip_height: 0,
            last_successful_sync_at: 0,
            last_error: None,
        }
    }
}

pub fn resolve_spend_paths(
    app_data_dir: &Path,
    network: WalletNetwork,
    account_hash: &str,
    coin_id: &str,
) -> SpendStoragePaths {
    let shared = resolve_paths(app_data_dir, network, account_hash, coin_id);
    let root = shared.root;
    SpendStoragePaths {
        wallet_db: root.join("spend_wallet.db"),
        blocks_dir: root.join("spend_blocks"),
        meta_json: root.join("spend_meta.json"),
        root,
    }
}

pub fn ensure_layout(paths: &SpendStoragePaths) -> Result<(), WalletError> {
    fs::create_dir_all(&paths.root).map_err(|_| WalletError::OperationFailed)?;
    fs::create_dir_all(&paths.blocks_dir).map_err(|_| WalletError::OperationFailed)?;
    if !paths.wallet_db.exists() {
        save_wallet_db(paths, &StoredSpendWalletDb::default())?;
    }
    if !paths.meta_json.exists() {
        save_meta(paths, &StoredSpendMeta::default())?;
    }
    Ok(())
}

pub fn load_wallet_db(paths: &SpendStoragePaths) -> Result<StoredSpendWalletDb, WalletError> {
    if !paths.wallet_db.exists() {
        return Ok(StoredSpendWalletDb::default());
    }

    let bytes = fs::read(&paths.wallet_db).map_err(|_| WalletError::OperationFailed)?;
    let mut parsed: StoredSpendWalletDb =
        serde_json::from_slice(&bytes).map_err(|_| WalletError::OperationFailed)?;
    if !matches!(
        parsed.schema_version,
        SPEND_SCHEMA_VERSION | LEGACY_SPEND_SCHEMA_V2
    ) {
        parsed = StoredSpendWalletDb::default();
    } else {
        // Keep legacy spend state instead of resetting to cold sync.
        parsed.schema_version = SPEND_SCHEMA_VERSION;
    }
    Ok(parsed)
}

pub fn save_wallet_db(
    paths: &SpendStoragePaths,
    state: &StoredSpendWalletDb,
) -> Result<(), WalletError> {
    let bytes = serde_json::to_vec_pretty(state).map_err(|_| WalletError::OperationFailed)?;
    fs::write(&paths.wallet_db, bytes).map_err(|_| WalletError::OperationFailed)
}

pub fn load_meta(paths: &SpendStoragePaths) -> Result<StoredSpendMeta, WalletError> {
    if !paths.meta_json.exists() {
        return Ok(StoredSpendMeta::default());
    }

    let bytes = fs::read(&paths.meta_json).map_err(|_| WalletError::OperationFailed)?;
    let mut parsed: StoredSpendMeta =
        serde_json::from_slice(&bytes).map_err(|_| WalletError::OperationFailed)?;
    if !matches!(
        parsed.schema_version,
        SPEND_SCHEMA_VERSION | LEGACY_SPEND_SCHEMA_V2
    ) {
        parsed = StoredSpendMeta::default();
    } else {
        parsed.schema_version = SPEND_SCHEMA_VERSION;
    }
    Ok(parsed)
}

pub fn save_meta(paths: &SpendStoragePaths, state: &StoredSpendMeta) -> Result<(), WalletError> {
    let bytes = serde_json::to_vec_pretty(state).map_err(|_| WalletError::OperationFailed)?;
    fs::write(&paths.meta_json, bytes).map_err(|_| WalletError::OperationFailed)
}

#[cfg(test)]
mod tests {
    use super::{
        load_meta, load_wallet_db, StoredSpendMeta, StoredSpendWalletDb, SpendStoragePaths,
        SPEND_SCHEMA_VERSION,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_test_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "lite_wallet_spend_db_{name}_{}_{}",
            std::process::id(),
            nanos
        ))
    }

    fn test_paths(name: &str) -> SpendStoragePaths {
        let root = unique_test_root(name);
        let _ = fs::create_dir_all(&root);
        SpendStoragePaths {
            wallet_db: root.join("spend_wallet.db"),
            blocks_dir: root.join("spend_blocks"),
            meta_json: root.join("spend_meta.json"),
            root,
        }
    }

    #[test]
    fn load_wallet_db_keeps_legacy_v2_state() {
        let paths = test_paths("wallet_v2");
        let mut wallet = StoredSpendWalletDb::default();
        wallet.schema_version = 2;
        wallet.scanned_height = 123;
        wallet.chain_tip_height = 456;
        wallet.confirmed_balance_sats = 999;
        wallet.last_updated = 42;
        let payload = serde_json::to_vec_pretty(&wallet).expect("serialize v2 wallet");
        fs::write(&paths.wallet_db, payload).expect("write v2 wallet");

        let loaded = load_wallet_db(&paths).expect("load wallet");
        assert_eq!(loaded.schema_version, SPEND_SCHEMA_VERSION);
        assert_eq!(loaded.scanned_height, 123);
        assert_eq!(loaded.chain_tip_height, 456);
        assert_eq!(loaded.confirmed_balance_sats, 999);
        assert_eq!(loaded.last_updated, 42);

        let _ = fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn load_meta_keeps_legacy_v2_state() {
        let paths = test_paths("meta_v2");
        let meta = serde_json::json!({
            "schemaVersion": 2,
            "lastSuccessfulSyncHeight": 10,
            "lastSuccessfulTipHeight": 12,
            "lastSuccessfulSyncAt": 99
        });
        fs::write(
            &paths.meta_json,
            serde_json::to_vec_pretty(&meta).expect("serialize meta"),
        )
        .expect("write meta");

        let loaded = load_meta(&paths).expect("load meta");
        assert_eq!(loaded.schema_version, SPEND_SCHEMA_VERSION);
        assert_eq!(loaded.last_successful_sync_height, 10);
        assert_eq!(loaded.last_successful_tip_height, 12);
        assert_eq!(loaded.last_successful_sync_at, 99);
        assert_eq!(loaded.last_error, None);

        let _ = fs::remove_dir_all(&paths.root);
    }

    #[test]
    fn load_meta_resets_unknown_schema() {
        let paths = test_paths("meta_unknown");
        let mut meta = StoredSpendMeta::default();
        meta.schema_version = 99;
        fs::write(
            &paths.meta_json,
            serde_json::to_vec_pretty(&meta).expect("serialize unknown meta"),
        )
        .expect("write meta");

        let loaded = load_meta(&paths).expect("load meta");
        assert_eq!(loaded.schema_version, SPEND_SCHEMA_VERSION);
        assert_eq!(loaded.last_successful_sync_height, 0);

        let _ = fs::remove_dir_all(&paths.root);
    }
}
