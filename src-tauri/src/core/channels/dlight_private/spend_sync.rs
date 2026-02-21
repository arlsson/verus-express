use std::collections::{HashMap, HashSet};
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint, Uri};
use zcash_client_backend::data_api::BlockMetadata;
use zcash_client_backend::proto::compact_formats::CompactBlock;
use zcash_client_backend::proto::service::{
    self, compact_tx_streamer_client::CompactTxStreamerClient,
};
use zcash_client_backend::scanning::{scan_block, Nullifiers};
use zcash_protocol::consensus::BranchId;
use zcash_protocol::consensus::{BlockHeight, NetworkType, NetworkUpgrade, Parameters};
use zip32::Scope;

use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

use super::spend_db::{
    ensure_layout, load_meta, load_wallet_db, resolve_spend_paths, save_meta, save_wallet_db,
    StoredRseed, StoredRseedKind, StoredSpendMeta, StoredSpendNote, StoredSpendScope,
    StoredSpendTree, StoredSpendWalletDb, StoredSpendWitness,
};
use super::spend_keys::DlightSpendKeyMaterial;
use super::store::{load_state as load_runtime_state, resolve_paths as resolve_runtime_paths};
use super::{normalize_grpc_endpoint, DlightRuntimeRequest};

const VERUS_MAINNET_CHAIN_NAME: &str = "vrsc";
const VERUS_MAINNET_CHAIN_NAME_LEGACY: &str = "main";
const VERUS_TESTNET_CHAIN_NAME: &str = "vrsctest";
const VERUS_TESTNET_CHAIN_NAME_LEGACY: &str = "test";
const VERUS_MAINNET_SAPLING_ACTIVATION_HEIGHT: u64 = 227_520;
const VERUS_TESTNET_SAPLING_ACTIVATION_HEIGHT: u64 = 1;
const SYNC_BATCH_SIZE: u64 = 700;
const DIAL_CONNECT_TIMEOUT_SECS: u64 = 8;
const DIAL_RPC_TIMEOUT_SECS: u64 = 12;
const BLOCK_STREAM_MESSAGE_TIMEOUT_SECS: u64 = 20;
const SPEND_SYNC_LOOP_SLEEP_SYNCING_SECS: u64 = 4;
const SPEND_SYNC_LOOP_SLEEP_SYNCED_SECS: u64 = 20;
const SPEND_SYNC_LOOP_SLEEP_ERROR_SECS: u64 = 8;
const TREE_SIZE_RECOVERY_LIMIT: u8 = 8;

#[derive(Debug, Clone)]
pub struct SpendableNote {
    pub nullifier_hex: String,
    pub value_sats: u64,
    pub received_height: u64,
    pub txid: String,
    pub scope: Scope,
    pub note: sapling::Note,
    pub merkle_path: sapling::MerklePath,
}

#[derive(Debug, Clone)]
pub struct SpendSyncSnapshot {
    pub scanned_height: u64,
    pub chain_tip_height: u64,
    pub confirmed_balance_sats: u64,
    pub spendable_notes: Vec<SpendableNote>,
}

#[derive(Debug, Clone)]
pub struct SpendCacheStatus {
    pub ready: bool,
    pub scanned_height: u64,
    pub chain_tip_height: u64,
    pub effective_tip_height: u64,
    pub lag_blocks: u64,
    pub status_kind: String,
    pub percent: Option<f64>,
    pub last_updated: u64,
    pub note_count: u64,
    pub last_error: Option<String>,
}

fn summarize_spend_cache_status(
    scanned_height: u64,
    cache_tip_height: u64,
    runtime_tip_hint: Option<u64>,
    last_error: Option<String>,
) -> (bool, u64, u64, String, Option<f64>, Option<String>) {
    let effective_tip_height = runtime_tip_hint
        .filter(|tip| *tip > 0)
        .unwrap_or(cache_tip_height);
    let lag_blocks = effective_tip_height.saturating_sub(scanned_height);
    let ready = effective_tip_height > 0 && lag_blocks == 0;
    let status_kind = if ready {
        "synced".to_string()
    } else if last_error.is_some() {
        "error".to_string()
    } else if effective_tip_height > 0 {
        "syncing".to_string()
    } else {
        "initializing".to_string()
    };
    let percent = if effective_tip_height == 0 {
        Some(0.0)
    } else {
        Some(((scanned_height as f64 / effective_tip_height as f64) * 100.0).clamp(0.0, 100.0))
    };

    (
        ready,
        effective_tip_height,
        lag_blocks,
        status_kind,
        percent,
        last_error,
    )
}

#[derive(Debug, Clone)]
struct RuntimeTrackedNote {
    nullifier_hex: String,
    value_sats: u64,
    received_height: u64,
    txid: String,
    scope: Scope,
    note: sapling::Note,
    witness: sapling::IncrementalWitness,
}

#[derive(Debug, Clone)]
struct RuntimeSpendState {
    scanned_height: u64,
    tree: sapling::CommitmentTree,
    notes: HashMap<String, RuntimeTrackedNote>,
}

#[derive(Debug, Clone, Copy)]
struct VerusConsensusParams {
    network_type: NetworkType,
    sapling_activation_height: BlockHeight,
}

impl Parameters for VerusConsensusParams {
    fn network_type(&self) -> NetworkType {
        self.network_type
    }

    fn activation_height(&self, nu: NetworkUpgrade) -> Option<BlockHeight> {
        match nu {
            NetworkUpgrade::Overwinter => Some(self.sapling_activation_height),
            NetworkUpgrade::Sapling => Some(self.sapling_activation_height),
            NetworkUpgrade::Blossom => None,
            NetworkUpgrade::Heartwood => None,
            NetworkUpgrade::Canopy => None,
            NetworkUpgrade::Nu5 => None,
            NetworkUpgrade::Nu6 => None,
            NetworkUpgrade::Nu6_1 => None,
        }
    }
}

pub async fn sync_spend_state(
    request: &DlightRuntimeRequest,
    key_material: &DlightSpendKeyMaterial,
) -> Result<SpendSyncSnapshot, WalletError> {
    let paths = resolve_spend_paths(
        &request.app_data_dir,
        request.network,
        &request.account_hash,
        &request.coin_id,
    );
    ensure_layout(&paths)?;

    let mut wallet_db = load_wallet_db(&paths).unwrap_or_default();

    let runtime_snapshot =
        super::runtime::get_runtime_snapshot(&request.runtime_key).unwrap_or_default();
    let runtime_tip_hint = runtime_snapshot
        .chain_tip_height
        .or(runtime_snapshot.estimated_tip_height)
        .filter(|tip| *tip > 0);

    let mut chain_tip_height = runtime_tip_hint.unwrap_or(0);
    let mut client: Option<CompactTxStreamerClient<Channel>> = None;

    if chain_tip_height == 0 {
        let mut tip_client = connect_lightwalletd(&request.endpoint).await?;
        let lightd_info = tokio::time::timeout(
            Duration::from_secs(DIAL_RPC_TIMEOUT_SECS),
            tip_client.get_lightd_info(service::Empty {}),
        )
        .await
        .map_err(|_| WalletError::NetworkError)?
        .map_err(|_| WalletError::NetworkError)?
        .into_inner();

        validate_server_chain_info(request.network, &lightd_info)?;
        chain_tip_height = lightd_info.block_height;
        client = Some(tip_client);
    }

    let birthday_floor = sapling_birthday_floor(request.network);
    if chain_tip_height <= birthday_floor {
        wallet_db.schema_version = super::spend_db::SPEND_SCHEMA_VERSION;
        wallet_db.scanned_height = chain_tip_height;
        wallet_db.chain_tip_height = chain_tip_height;
        wallet_db.confirmed_balance_sats = 0;
        wallet_db.notes.clear();
        wallet_db.tree = StoredSpendTree::default();
        wallet_db.last_updated = super::store::unix_timestamp_secs();
        save_wallet_db(&paths, &wallet_db)?;

        let meta = StoredSpendMeta {
            schema_version: super::spend_db::SPEND_SCHEMA_VERSION,
            last_successful_sync_height: chain_tip_height,
            last_successful_tip_height: chain_tip_height,
            last_successful_sync_at: super::store::unix_timestamp_secs(),
            last_error: None,
        };
        save_meta(&paths, &meta)?;

        return Ok(SpendSyncSnapshot {
            scanned_height: chain_tip_height,
            chain_tip_height,
            confirmed_balance_sats: 0,
            spendable_notes: vec![],
        });
    }

    let scan_params = verus_scan_params(request.network);

    let mut runtime_state = if let Some(cached) = load_cached_runtime_state(&wallet_db) {
        if cached.scanned_height >= birthday_floor && cached.scanned_height <= chain_tip_height {
            cached
        } else {
            RuntimeSpendState {
                scanned_height: birthday_floor,
                tree: sapling::CommitmentTree::empty(),
                notes: HashMap::new(),
            }
        }
    } else {
        RuntimeSpendState {
            scanned_height: birthday_floor,
            tree: sapling::CommitmentTree::empty(),
            notes: HashMap::new(),
        }
    };

    let mut prior_block_metadata: Option<BlockMetadata> =
        prior_block_metadata_from_runtime_state(request, runtime_state.scanned_height);
    let mut current = runtime_state.scanned_height.saturating_add(1);
    let mut tree_size_recovery_attempts: u8 = 0;

    if current <= chain_tip_height {
        if client.is_none() {
            client = Some(connect_lightwalletd(&request.endpoint).await?);
        }
        let client = client
            .as_mut()
            .ok_or(WalletError::DlightSpendCacheNotReady)?;

        while current <= chain_tip_height {
            let end = current
                .saturating_add(SYNC_BATCH_SIZE.saturating_sub(1))
                .min(chain_tip_height);

            let batch_blocks = fetch_compact_blocks(client, current, end).await?;
            for compact_block in batch_blocks {
                maybe_seed_prior_metadata_for_sapling_activation(
                    request.network,
                    &compact_block,
                    &mut prior_block_metadata,
                );
                maybe_seed_prior_metadata_from_local_state(
                    &compact_block,
                    &runtime_state,
                    &mut prior_block_metadata,
                );

                let scanned = match {
                    let scanning_keys = key_material.to_scanning_keys()?;
                    scan_block(
                        &scan_params,
                        compact_block.clone(),
                        &scanning_keys,
                        &Nullifiers::empty(),
                        prior_block_metadata.as_ref(),
                    )
                } {
                    Ok(value) => value,
                    Err(scan_error) => {
                        eprintln!(
                            "[dlight_private][spend_sync] scan failed at height={} continuity={} error={:?}",
                            compact_block.height,
                            scan_error.is_continuity_error(),
                            scan_error
                        );

                        if scan_error.is_continuity_error() {
                            persist_spend_sync_error(
                                &paths,
                                format!(
                                    "continuity mismatch at height {}; waiting for background recovery",
                                    compact_block.height
                                ),
                            );
                            return Err(WalletError::DlightSpendCacheNotReady);
                        }

                        let scan_error_debug = format!("{scan_error:?}");
                        let is_tree_size_unknown = scan_error_debug.contains("TreeSizeUnknown");
                        let can_attempt_tree_recovery =
                            is_tree_size_unknown && tree_size_recovery_attempts < TREE_SIZE_RECOVERY_LIMIT;

                        if can_attempt_tree_recovery {
                            tree_size_recovery_attempts =
                                tree_size_recovery_attempts.saturating_add(1);
                            let local_tree_size_u32 =
                                u32::try_from(runtime_state.tree.size()).ok();
                            let seeded = seed_prior_metadata_from_cached_state(
                                request,
                                client,
                                runtime_state.scanned_height,
                                local_tree_size_u32,
                            )
                            .await;
                            if let Some(metadata) = seeded {
                                prior_block_metadata = Some(metadata);
                                eprintln!(
                                    "[dlight_private][spend_sync] tree size recovery retry at height={} attempt={}",
                                    compact_block.height, tree_size_recovery_attempts
                                );
                                let retry_scanned = {
                                    let retry_scanning_keys = key_material.to_scanning_keys()?;
                                    scan_block(
                                        &scan_params,
                                        compact_block.clone(),
                                        &retry_scanning_keys,
                                        &Nullifiers::empty(),
                                        prior_block_metadata.as_ref(),
                                    )
                                };
                                if let Ok(value) = retry_scanned {
                                    value
                                } else {
                                    persist_spend_sync_error(
                                        &paths,
                                        format!(
                                            "tree size unknown at height {}; runtime metadata retry failed",
                                            compact_block.height
                                        ),
                                    );
                                    return Err(WalletError::DlightSpendCacheNotReady);
                                }
                            } else {
                                persist_spend_sync_error(
                                    &paths,
                                    format!(
                                        "tree size unknown at height {}; no runtime metadata available for retry",
                                        compact_block.height
                                    ),
                                );
                                return Err(WalletError::DlightSpendCacheNotReady);
                            }
                        } else if is_tree_size_unknown {
                            persist_spend_sync_error(
                                &paths,
                                format!(
                                    "tree size unknown at height {}; waiting for background recovery",
                                    compact_block.height
                                ),
                            );
                            return Err(WalletError::DlightSpendCacheNotReady);
                        } else {
                            persist_spend_sync_error(
                                &paths,
                                format!("spend sync scan failure at height {}", compact_block.height),
                            );
                            return Err(WalletError::OperationFailed);
                        }
                    }
                };

                apply_compact_block(
                    &compact_block,
                    &scanned,
                    &mut runtime_state.tree,
                    &mut runtime_state.notes,
                )?;
                runtime_state.scanned_height = compact_block.height;
                prior_block_metadata = Some(scanned.to_block_metadata());
            }

            if runtime_state.scanned_height < end {
                persist_spend_sync_error(
                    &paths,
                    format!(
                        "incomplete compact block range start={} end={} scanned_height={}",
                        current, end, runtime_state.scanned_height
                    ),
                );
                return Err(WalletError::DlightSpendCacheNotReady);
            }

            persist_runtime_spend_state(&paths, &runtime_state, chain_tip_height)?;
            current = runtime_state.scanned_height.saturating_add(1);
        }
    }

    persist_runtime_spend_state(&paths, &runtime_state, chain_tip_height)?;

    let mut spendable_notes = runtime_state
        .notes
        .values()
        .filter_map(|tracked| {
            let path = tracked.witness.path()?;
            Some(SpendableNote {
                nullifier_hex: tracked.nullifier_hex.clone(),
                value_sats: tracked.value_sats,
                received_height: tracked.received_height,
                txid: tracked.txid.clone(),
                scope: tracked.scope,
                note: tracked.note.clone(),
                merkle_path: path,
            })
        })
        .collect::<Vec<_>>();

    spendable_notes.sort_by(|left, right| {
        left.received_height
            .cmp(&right.received_height)
            .then(left.txid.cmp(&right.txid))
            .then(left.nullifier_hex.cmp(&right.nullifier_hex))
    });

    let confirmed_balance_sats: u64 = spendable_notes.iter().map(|note| note.value_sats).sum();

    Ok(SpendSyncSnapshot {
        scanned_height: runtime_state.scanned_height,
        chain_tip_height,
        confirmed_balance_sats,
        spendable_notes,
    })
}

pub fn mark_notes_spent(
    request: &DlightRuntimeRequest,
    nullifiers: &[String],
    spent_height: u64,
) -> Result<(), WalletError> {
    if nullifiers.is_empty() {
        return Ok(());
    }

    let paths = resolve_spend_paths(
        &request.app_data_dir,
        request.network,
        &request.account_hash,
        &request.coin_id,
    );
    ensure_layout(&paths)?;

    let mut wallet_db = load_wallet_db(&paths)?;
    let spent_set = nullifiers
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect::<HashSet<_>>();

    for note in &mut wallet_db.notes {
        if spent_set.contains(&note.nullifier_hex.to_ascii_lowercase()) {
            note.spent_height = Some(spent_height);
        }
    }

    wallet_db.confirmed_balance_sats = wallet_db
        .notes
        .iter()
        .filter(|note| note.spent_height.is_none())
        .map(|note| note.value_sats)
        .sum();
    wallet_db.last_updated = super::store::unix_timestamp_secs();
    save_wallet_db(&paths, &wallet_db)
}

pub fn load_spend_snapshot(
    request: &DlightRuntimeRequest,
    runtime_tip_hint: Option<u64>,
) -> Result<SpendSyncSnapshot, WalletError> {
    let paths = resolve_spend_paths(
        &request.app_data_dir,
        request.network,
        &request.account_hash,
        &request.coin_id,
    );
    ensure_layout(&paths)?;

    let wallet_db = load_wallet_db(&paths)?;
    let effective_tip_height = runtime_tip_hint
        .filter(|tip| *tip > 0)
        .unwrap_or(wallet_db.chain_tip_height);
    if effective_tip_height == 0 || wallet_db.scanned_height < effective_tip_height {
        return Err(WalletError::DlightSpendCacheNotReady);
    }

    let runtime_state =
        load_cached_runtime_state(&wallet_db).ok_or(WalletError::DlightSpendCacheNotReady)?;
    let mut spendable_notes = runtime_state
        .notes
        .values()
        .filter_map(|tracked| {
            let path = tracked.witness.path()?;
            Some(SpendableNote {
                nullifier_hex: tracked.nullifier_hex.clone(),
                value_sats: tracked.value_sats,
                received_height: tracked.received_height,
                txid: tracked.txid.clone(),
                scope: tracked.scope,
                note: tracked.note.clone(),
                merkle_path: path,
            })
        })
        .collect::<Vec<_>>();
    spendable_notes.sort_by(|left, right| {
        left.received_height
            .cmp(&right.received_height)
            .then(left.txid.cmp(&right.txid))
            .then(left.nullifier_hex.cmp(&right.nullifier_hex))
    });
    let confirmed_balance_sats: u64 = spendable_notes.iter().map(|note| note.value_sats).sum();

    Ok(SpendSyncSnapshot {
        scanned_height: wallet_db.scanned_height,
        chain_tip_height: effective_tip_height,
        confirmed_balance_sats,
        spendable_notes,
    })
}

pub fn get_spend_cache_status(
    request: &DlightRuntimeRequest,
    runtime_tip_hint: Option<u64>,
) -> Option<SpendCacheStatus> {
    let paths = resolve_spend_paths(
        &request.app_data_dir,
        request.network,
        &request.account_hash,
        &request.coin_id,
    );
    if ensure_layout(&paths).is_err() {
        return None;
    }

    let wallet_db = load_wallet_db(&paths).ok()?;
    let meta = load_meta(&paths).unwrap_or_default();
    let note_count = wallet_db
        .notes
        .iter()
        .filter(|note| note.spent_height.is_none())
        .count() as u64;
    let (ready, effective_tip_height, lag_blocks, status_kind, percent, last_error) =
        summarize_spend_cache_status(
            wallet_db.scanned_height,
            wallet_db.chain_tip_height,
            runtime_tip_hint,
            meta.last_error.clone(),
        );

    Some(SpendCacheStatus {
        ready,
        scanned_height: wallet_db.scanned_height,
        chain_tip_height: wallet_db.chain_tip_height,
        effective_tip_height,
        lag_blocks,
        status_kind,
        percent,
        last_updated: wallet_db.last_updated,
        note_count,
        last_error,
    })
}

pub async fn run_spend_cache_loop(request: DlightRuntimeRequest, cancel: CancellationToken) {
    let key_material = match DlightSpendKeyMaterial::from_seed_material(
        &request.seed_material,
        request.network,
        &request.scope_address,
    ) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "[dlight_private][spend_sync] worker failed to derive key material: {:?}",
                error
            );
            return;
        }
    };

    loop {
        if cancel.is_cancelled() {
            return;
        }

        let sync_result = sync_spend_state(&request, &key_material).await;
        let sleep_secs = match sync_result {
            Ok(snapshot) => {
                if snapshot.scanned_height >= snapshot.chain_tip_height {
                    SPEND_SYNC_LOOP_SLEEP_SYNCED_SECS
                } else {
                    SPEND_SYNC_LOOP_SLEEP_SYNCING_SECS
                }
            }
            Err(error) => {
                eprintln!(
                    "[dlight_private][spend_sync] background sync cycle failed: {:?}",
                    error
                );
                SPEND_SYNC_LOOP_SLEEP_ERROR_SECS
            }
        };

        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = tokio::time::sleep(Duration::from_secs(sleep_secs)) => {}
        }
    }
}

fn persist_spend_sync_error(paths: &super::spend_db::SpendStoragePaths, error: String) {
    let mut meta = load_meta(paths).unwrap_or_default();
    meta.schema_version = super::spend_db::SPEND_SCHEMA_VERSION;
    meta.last_error = Some(error);
    let _ = save_meta(paths, &meta);
}

fn persist_runtime_spend_state(
    paths: &super::spend_db::SpendStoragePaths,
    runtime_state: &RuntimeSpendState,
    chain_tip_height: u64,
) -> Result<(), WalletError> {
    let mut wallet_db = load_wallet_db(paths).unwrap_or_default();
    wallet_db.schema_version = super::spend_db::SPEND_SCHEMA_VERSION;
    wallet_db.scanned_height = runtime_state.scanned_height.min(chain_tip_height);
    wallet_db.chain_tip_height = chain_tip_height;
    wallet_db.tree = encode_tree(&runtime_state.tree);
    let mut stored_notes = runtime_state
        .notes
        .values()
        .filter_map(encode_runtime_tracked_note)
        .collect::<Vec<_>>();
    stored_notes.sort_by(|left, right| {
        left.received_height
            .cmp(&right.received_height)
            .then(left.txid.cmp(&right.txid))
            .then(left.nullifier_hex.cmp(&right.nullifier_hex))
    });
    wallet_db.confirmed_balance_sats = stored_notes
        .iter()
        .filter(|note| note.spent_height.is_none())
        .map(|note| note.value_sats)
        .sum();
    wallet_db.notes = stored_notes;
    wallet_db.last_updated = super::store::unix_timestamp_secs();
    save_wallet_db(paths, &wallet_db)?;

    let meta = StoredSpendMeta {
        schema_version: super::spend_db::SPEND_SCHEMA_VERSION,
        last_successful_sync_height: wallet_db.scanned_height,
        last_successful_tip_height: chain_tip_height,
        last_successful_sync_at: wallet_db.last_updated,
        last_error: None,
    };
    save_meta(paths, &meta)?;

    Ok(())
}

fn prior_block_metadata_from_runtime_state(
    request: &DlightRuntimeRequest,
    expected_height: u64,
) -> Option<BlockMetadata> {
    if expected_height == 0 {
        return None;
    }

    let runtime_paths = resolve_runtime_paths(
        &request.app_data_dir,
        request.network,
        &request.account_hash,
        &request.coin_id,
    );
    let runtime_state = load_runtime_state(&runtime_paths).ok()?;
    if runtime_state.scanned_height != expected_height {
        return None;
    }
    let tree_size = runtime_state.sapling_tree_size?;
    let hash_hex = runtime_state.block_hash_hex?;
    let hash_bytes = hex::decode(hash_hex).ok()?;
    if hash_bytes.len() != 32 {
        return None;
    }

    let synthetic_block = CompactBlock {
        proto_version: 0,
        height: expected_height,
        hash: hash_bytes,
        prev_hash: vec![],
        time: 0,
        header: vec![],
        vtx: vec![],
        chain_metadata: None,
    };

    Some(BlockMetadata::from_parts(
        synthetic_block.height(),
        synthetic_block.hash(),
        Some(tree_size),
    ))
}

async fn seed_prior_metadata_from_cached_runtime(
    request: &DlightRuntimeRequest,
    client: &mut CompactTxStreamerClient<Channel>,
    expected_height: u64,
) -> Option<BlockMetadata> {
    if expected_height == 0 {
        return None;
    }

    let runtime_paths = resolve_runtime_paths(
        &request.app_data_dir,
        request.network,
        &request.account_hash,
        &request.coin_id,
    );
    let runtime_state = load_runtime_state(&runtime_paths).ok()?;
    if runtime_state.scanned_height != expected_height {
        return None;
    }
    let tree_size = runtime_state.sapling_tree_size?;

    let block = fetch_compact_blocks(client, expected_height, expected_height)
        .await
        .ok()?
        .into_iter()
        .next()?;

    Some(BlockMetadata::from_parts(
        block.height(),
        block.hash(),
        Some(tree_size),
    ))
}

async fn seed_prior_metadata_from_cached_state(
    request: &DlightRuntimeRequest,
    client: &mut CompactTxStreamerClient<Channel>,
    expected_height: u64,
    local_tree_size: Option<u32>,
) -> Option<BlockMetadata> {
    let runtime_seed =
        seed_prior_metadata_from_cached_runtime(request, client, expected_height).await;
    if runtime_seed.is_some() {
        return runtime_seed;
    }

    let tree_size = local_tree_size?;
    let block = fetch_compact_blocks(client, expected_height, expected_height)
        .await
        .ok()?
        .into_iter()
        .next()?;

    Some(BlockMetadata::from_parts(
        block.height(),
        block.hash(),
        Some(tree_size),
    ))
}

fn load_cached_runtime_state(wallet_db: &StoredSpendWalletDb) -> Option<RuntimeSpendState> {
    if wallet_db.scanned_height == 0 {
        return None;
    }

    let tree = decode_tree(&wallet_db.tree)?;
    let mut notes = HashMap::<String, RuntimeTrackedNote>::new();

    for stored_note in wallet_db
        .notes
        .iter()
        .filter(|note| note.spent_height.is_none())
    {
        let note = decode_note(stored_note)?;
        let witness = decode_witness(&stored_note.witness)?;
        let scope = match stored_note.scope {
            StoredSpendScope::External => Scope::External,
            StoredSpendScope::Internal => Scope::Internal,
        };

        notes.insert(
            stored_note.nullifier_hex.clone(),
            RuntimeTrackedNote {
                nullifier_hex: stored_note.nullifier_hex.clone(),
                value_sats: stored_note.value_sats,
                received_height: stored_note.received_height,
                txid: stored_note.txid.clone(),
                scope,
                note,
                witness,
            },
        );
    }

    Some(RuntimeSpendState {
        scanned_height: wallet_db.scanned_height,
        tree,
        notes,
    })
}

fn encode_runtime_tracked_note(note: &RuntimeTrackedNote) -> Option<StoredSpendNote> {
    let note_position = note.witness.witnessed_position();

    Some(StoredSpendNote {
        nullifier_hex: note.nullifier_hex.clone(),
        value_sats: note.value_sats,
        received_height: note.received_height,
        spent_height: None,
        note_position: u64::from(note_position),
        txid: note.txid.clone(),
        scope: match note.scope {
            Scope::External => StoredSpendScope::External,
            Scope::Internal => StoredSpendScope::Internal,
        },
        recipient_bytes_hex: hex::encode(note.note.recipient().to_bytes()),
        rseed: encode_rseed(note.note.rseed()),
        witness: encode_witness(&note.witness),
    })
}

fn encode_rseed(rseed: &sapling::Rseed) -> StoredRseed {
    match rseed {
        sapling::Rseed::BeforeZip212(rcm) => StoredRseed {
            kind: StoredRseedKind::BeforeZip212,
            bytes_hex: hex::encode(rcm.to_bytes()),
        },
        sapling::Rseed::AfterZip212(bytes) => StoredRseed {
            kind: StoredRseedKind::AfterZip212,
            bytes_hex: hex::encode(bytes),
        },
    }
}

fn decode_rseed(stored: &StoredRseed) -> Option<sapling::Rseed> {
    match stored.kind {
        StoredRseedKind::AfterZip212 => {
            let bytes = decode_fixed_hex::<32>(&stored.bytes_hex)?;
            Some(sapling::Rseed::AfterZip212(bytes))
        }
        StoredRseedKind::BeforeZip212 => {
            let bytes = decode_fixed_hex::<32>(&stored.bytes_hex)?;
            let rcm = Option::<jubjub::Fr>::from(jubjub::Fr::from_bytes(&bytes))?;
            Some(sapling::Rseed::BeforeZip212(rcm))
        }
    }
}

fn decode_note(stored_note: &StoredSpendNote) -> Option<sapling::Note> {
    let recipient_bytes = decode_fixed_hex::<43>(&stored_note.recipient_bytes_hex)?;
    let recipient = sapling::PaymentAddress::from_bytes(&recipient_bytes)?;
    let rseed = decode_rseed(&stored_note.rseed)?;
    Some(sapling::Note::from_parts(
        recipient,
        sapling::value::NoteValue::from_raw(stored_note.value_sats),
        rseed,
    ))
}

fn encode_witness(witness: &sapling::IncrementalWitness) -> StoredSpendWitness {
    StoredSpendWitness {
        tree: encode_tree(witness.tree()),
        filled: witness
            .filled()
            .iter()
            .map(|node| hex::encode(node.to_bytes()))
            .collect::<Vec<_>>(),
        cursor: witness.cursor().as_ref().map(encode_tree),
    }
}

fn decode_witness(stored: &StoredSpendWitness) -> Option<sapling::IncrementalWitness> {
    let tree = decode_tree(&stored.tree)?;
    let filled = stored
        .filled
        .iter()
        .map(|encoded| decode_node_hex(encoded))
        .collect::<Option<Vec<_>>>()?;
    let cursor = match stored.cursor.as_ref() {
        Some(value) => Some(decode_tree(value)?),
        None => None,
    };

    sapling::IncrementalWitness::from_parts(tree, filled, cursor)
}

fn encode_tree(tree: &sapling::CommitmentTree) -> StoredSpendTree {
    StoredSpendTree {
        left: tree
            .left()
            .as_ref()
            .map(|node| hex::encode(node.to_bytes())),
        right: tree
            .right()
            .as_ref()
            .map(|node| hex::encode(node.to_bytes())),
        parents: tree
            .parents()
            .iter()
            .map(|parent| parent.as_ref().map(|node| hex::encode(node.to_bytes())))
            .collect::<Vec<_>>(),
    }
}

fn decode_tree(stored: &StoredSpendTree) -> Option<sapling::CommitmentTree> {
    let left = match stored.left.as_ref() {
        Some(value) => Some(decode_node_hex(value)?),
        None => None,
    };
    let right = match stored.right.as_ref() {
        Some(value) => Some(decode_node_hex(value)?),
        None => None,
    };
    let parents = stored
        .parents
        .iter()
        .map(|entry| match entry {
            Some(value) => Some(decode_node_hex(value)),
            None => Some(None),
        })
        .collect::<Option<Vec<_>>>()?;

    sapling::CommitmentTree::from_parts(left, right, parents).ok()
}

fn decode_node_hex(value: &str) -> Option<sapling::Node> {
    let bytes = decode_fixed_hex::<32>(value)?;
    Option::<sapling::Node>::from(sapling::Node::from_bytes(bytes))
}

fn decode_fixed_hex<const N: usize>(value: &str) -> Option<[u8; N]> {
    let bytes = hex::decode(value.trim()).ok()?;
    bytes.try_into().ok()
}

async fn fetch_compact_blocks(
    client: &mut CompactTxStreamerClient<Channel>,
    start: u64,
    end: u64,
) -> Result<Vec<CompactBlock>, WalletError> {
    let block_range = service::BlockRange {
        start: Some(service::BlockId {
            height: start,
            hash: vec![],
        }),
        end: Some(service::BlockId {
            height: end,
            hash: vec![],
        }),
    };

    let mut stream = client
        .get_block_range(block_range)
        .await
        .map_err(|_| WalletError::NetworkError)?
        .into_inner();

    let mut blocks = Vec::<CompactBlock>::new();
    while let Some(compact_block) = tokio::time::timeout(
        Duration::from_secs(BLOCK_STREAM_MESSAGE_TIMEOUT_SECS),
        stream.message(),
    )
    .await
    .map_err(|_| WalletError::NetworkError)?
    .map_err(|_| WalletError::NetworkError)?
    {
        blocks.push(compact_block);
    }

    Ok(blocks)
}

fn apply_compact_block(
    compact_block: &CompactBlock,
    scanned_block: &zcash_client_backend::data_api::ScannedBlock<u32>,
    tree: &mut sapling::CommitmentTree,
    notes: &mut HashMap<String, RuntimeTrackedNote>,
) -> Result<(), WalletError> {
    #[derive(Debug)]
    struct DetectedNote {
        nullifier_hex: String,
        value_sats: u64,
        received_height: u64,
        txid: String,
        scope: Scope,
        note: sapling::Note,
    }

    let mut detected = HashMap::<(String, usize), DetectedNote>::new();
    let scanned_height = u64::from(scanned_block.height());

    for wallet_tx in scanned_block.transactions() {
        let txid = wallet_tx.txid().to_string();
        for output in wallet_tx.sapling_outputs() {
            let Some(nullifier) = output.nf() else {
                continue;
            };
            let witness_scope = output.recipient_key_scope().unwrap_or(Scope::External);
            let nullifier_hex = hex::encode(nullifier.as_ref());
            detected.insert(
                (txid.clone(), output.index()),
                DetectedNote {
                    nullifier_hex,
                    value_sats: output.note().value().inner(),
                    received_height: scanned_height,
                    txid: txid.clone(),
                    scope: witness_scope,
                    note: output.note().clone(),
                },
            );
        }
    }

    for tx in &compact_block.vtx {
        for spend in &tx.spends {
            if spend.nf.len() != 32 {
                continue;
            }
            let nullifier_hex = hex::encode(&spend.nf);
            notes.remove(&nullifier_hex);
        }
    }

    for tx in &compact_block.vtx {
        let txid = tx.txid().to_string();
        for (index, output) in tx.outputs.iter().enumerate() {
            let cmu = output.cmu().map_err(|_| WalletError::OperationFailed)?;
            let node = sapling::Node::from_cmu(&cmu);

            tree.append(node)
                .map_err(|_| WalletError::OperationFailed)?;
            for tracked in notes.values_mut() {
                let _ = tracked.witness.append(node);
            }

            if let Some(detected_note) = detected.remove(&(txid.clone(), index)) {
                let witness = sapling::IncrementalWitness::from_tree(tree.clone())
                    .ok_or(WalletError::OperationFailed)?;
                notes
                    .entry(detected_note.nullifier_hex.clone())
                    .or_insert(RuntimeTrackedNote {
                        nullifier_hex: detected_note.nullifier_hex,
                        value_sats: detected_note.value_sats,
                        received_height: detected_note.received_height,
                        txid: detected_note.txid,
                        scope: detected_note.scope,
                        note: detected_note.note,
                        witness,
                    });
            }
        }
    }

    Ok(())
}

async fn connect_lightwalletd(
    endpoint: &str,
) -> Result<CompactTxStreamerClient<Channel>, WalletError> {
    let grpc_endpoint = normalize_grpc_endpoint(endpoint)?;
    let parsed_uri: Uri = grpc_endpoint
        .parse()
        .map_err(|_| WalletError::UnsupportedChannel)?;
    let host = parsed_uri.host().ok_or(WalletError::UnsupportedChannel)?;
    let is_https = parsed_uri.scheme_str() == Some("https");

    let endpoint_builder = Endpoint::from_shared(grpc_endpoint)
        .map_err(|_| WalletError::UnsupportedChannel)?
        .connect_timeout(Duration::from_secs(DIAL_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(DIAL_RPC_TIMEOUT_SECS))
        .tcp_nodelay(true);

    let endpoint_builder = if is_https {
        endpoint_builder
            .tls_config(
                ClientTlsConfig::new()
                    .with_webpki_roots()
                    .domain_name(host.to_string()),
            )
            .map_err(|_| WalletError::OperationFailed)?
    } else {
        endpoint_builder
    };

    let channel = endpoint_builder
        .connect()
        .await
        .map_err(|_| WalletError::NetworkError)?;
    Ok(CompactTxStreamerClient::new(channel))
}

fn sapling_birthday_floor(network: WalletNetwork) -> u64 {
    match network {
        WalletNetwork::Mainnet => VERUS_MAINNET_SAPLING_ACTIVATION_HEIGHT.saturating_sub(1),
        WalletNetwork::Testnet => VERUS_TESTNET_SAPLING_ACTIVATION_HEIGHT.saturating_sub(1),
    }
}

fn verus_scan_params(network: WalletNetwork) -> VerusConsensusParams {
    let activation_height = match network {
        WalletNetwork::Mainnet => VERUS_MAINNET_SAPLING_ACTIVATION_HEIGHT,
        WalletNetwork::Testnet => VERUS_TESTNET_SAPLING_ACTIVATION_HEIGHT,
    };

    VerusConsensusParams {
        network_type: match network {
            WalletNetwork::Mainnet => NetworkType::Main,
            WalletNetwork::Testnet => NetworkType::Test,
        },
        sapling_activation_height: BlockHeight::from_u32(
            activation_height
                .try_into()
                .expect("known verus sapling activation heights fit in u32"),
        ),
    }
}

fn validate_server_chain_info(
    network: WalletNetwork,
    lightd_info: &service::LightdInfo,
) -> Result<(), WalletError> {
    let accepted_names: &[&str] = match network {
        WalletNetwork::Mainnet => &[VERUS_MAINNET_CHAIN_NAME, VERUS_MAINNET_CHAIN_NAME_LEGACY],
        WalletNetwork::Testnet => &[VERUS_TESTNET_CHAIN_NAME, VERUS_TESTNET_CHAIN_NAME_LEGACY],
    };
    let actual_chain_name = lightd_info.chain_name.trim().to_ascii_lowercase();
    if !accepted_names
        .iter()
        .any(|expected| actual_chain_name == *expected)
    {
        return Err(WalletError::UnsupportedChannel);
    }

    let expected_activation = match network {
        WalletNetwork::Mainnet => VERUS_MAINNET_SAPLING_ACTIVATION_HEIGHT,
        WalletNetwork::Testnet => VERUS_TESTNET_SAPLING_ACTIVATION_HEIGHT,
    };
    if lightd_info.sapling_activation_height != expected_activation {
        return Err(WalletError::UnsupportedChannel);
    }

    let branch_id = lightd_info
        .consensus_branch_id()
        .ok_or(WalletError::UnsupportedChannel)?;
    if branch_id != BranchId::Sapling {
        return Err(WalletError::UnsupportedChannel);
    }

    Ok(())
}

fn maybe_seed_prior_metadata_for_sapling_activation(
    network: WalletNetwork,
    compact_block: &CompactBlock,
    prior_block_metadata: &mut Option<BlockMetadata>,
) {
    if prior_block_metadata.is_some() || compact_block.chain_metadata.is_some() {
        return;
    }

    let activation_height = match network {
        WalletNetwork::Mainnet => VERUS_MAINNET_SAPLING_ACTIVATION_HEIGHT,
        WalletNetwork::Testnet => VERUS_TESTNET_SAPLING_ACTIVATION_HEIGHT,
    };
    if compact_block.height != activation_height {
        return;
    }

    let synthetic_prev_height = activation_height.saturating_sub(1);
    let Ok(synthetic_prev_height_u32) = u32::try_from(synthetic_prev_height) else {
        return;
    };

    *prior_block_metadata = Some(BlockMetadata::from_parts(
        BlockHeight::from_u32(synthetic_prev_height_u32),
        compact_block.prev_hash(),
        Some(0),
    ));
}

fn maybe_seed_prior_metadata_from_local_state(
    compact_block: &CompactBlock,
    runtime_state: &RuntimeSpendState,
    prior_block_metadata: &mut Option<BlockMetadata>,
) {
    if compact_block.chain_metadata.is_some() {
        return;
    }
    if compact_block.height == 0 {
        return;
    }

    let expected_prev_height = compact_block.height.saturating_sub(1);
    if runtime_state.scanned_height != expected_prev_height {
        return;
    }

    let needs_seed = prior_block_metadata
        .as_ref()
        .and_then(|value| value.sapling_tree_size())
        .is_none();
    if !needs_seed {
        return;
    }

    let Ok(expected_prev_height_u32) = u32::try_from(expected_prev_height) else {
        return;
    };
    let Some(tree_size_u32) = u32::try_from(runtime_state.tree.size()).ok() else {
        return;
    };

    *prior_block_metadata = Some(BlockMetadata::from_parts(
        BlockHeight::from_u32(expected_prev_height_u32),
        compact_block.prev_hash(),
        Some(tree_size_u32),
    ));
}

#[cfg(test)]
mod tests {
    use super::{
        decode_note, decode_tree, decode_witness, encode_rseed, encode_tree, encode_witness,
        summarize_spend_cache_status,
    };
    use crate::core::channels::dlight_private::spend_db::{
        StoredRseed, StoredRseedKind, StoredSpendTree, StoredSpendWitness,
    };

    #[test]
    fn encode_decode_tree_roundtrip_empty() {
        let tree = sapling::CommitmentTree::empty();
        let stored = encode_tree(&tree);
        let decoded = decode_tree(&stored).expect("decode tree");
        assert_eq!(decoded.size(), tree.size());
    }

    #[test]
    fn encode_decode_witness_roundtrip() {
        let mut tree = sapling::CommitmentTree::empty();
        let extsk = sapling::zip32::ExtendedSpendingKey::master(&[9u8; 32]);
        let recipient = extsk
            .to_diversifiable_full_viewing_key()
            .default_address()
            .1;
        let note = sapling::Note::from_parts(
            recipient,
            sapling::value::NoteValue::from_raw(1),
            sapling::Rseed::AfterZip212([5u8; 32]),
        );
        let node = sapling::Node::from_cmu(&note.cmu());
        tree.append(node).expect("append leaf");
        let witness = sapling::IncrementalWitness::from_tree(tree).expect("create witness");

        let stored = encode_witness(&witness);
        let decoded = decode_witness(&stored).expect("decode witness");
        assert_eq!(decoded.tree().size(), witness.tree().size());
    }

    #[test]
    fn decode_note_accepts_after_zip212() {
        let extsk = sapling::zip32::ExtendedSpendingKey::master(&[7u8; 32]);
        let recipient = extsk
            .to_diversifiable_full_viewing_key()
            .default_address()
            .1;
        let note = sapling::Note::from_parts(
            recipient,
            sapling::value::NoteValue::from_raw(123),
            sapling::Rseed::AfterZip212([7u8; 32]),
        );
        let stored = crate::core::channels::dlight_private::spend_db::StoredSpendNote {
            nullifier_hex: "00".repeat(32),
            value_sats: 123,
            received_height: 1,
            spent_height: None,
            note_position: 0,
            txid: "00".repeat(32),
            scope: crate::core::channels::dlight_private::spend_db::StoredSpendScope::External,
            recipient_bytes_hex: hex::encode(note.recipient().to_bytes()),
            rseed: encode_rseed(note.rseed()),
            witness: StoredSpendWitness {
                tree: StoredSpendTree::default(),
                filled: vec![],
                cursor: None,
            },
        };

        let decoded = decode_note(&stored).expect("decode note");
        assert_eq!(decoded.value().inner(), 123);
    }

    #[test]
    fn decode_note_rejects_invalid_rseed_hex() {
        let stored = crate::core::channels::dlight_private::spend_db::StoredSpendNote {
            nullifier_hex: "00".repeat(32),
            value_sats: 123,
            received_height: 1,
            spent_height: None,
            note_position: 0,
            txid: "00".repeat(32),
            scope: crate::core::channels::dlight_private::spend_db::StoredSpendScope::External,
            recipient_bytes_hex: "00".repeat(43),
            rseed: StoredRseed {
                kind: StoredRseedKind::AfterZip212,
                bytes_hex: "abc".to_string(),
            },
            witness: StoredSpendWitness {
                tree: StoredSpendTree::default(),
                filled: vec![],
                cursor: None,
            },
        };

        assert!(decode_note(&stored).is_none());
    }

    #[test]
    fn spend_cache_summary_uses_runtime_tip_hint() {
        let (ready, effective_tip, lag, status, percent, _) =
            summarize_spend_cache_status(90, 90, Some(100), None);
        assert!(!ready);
        assert_eq!(effective_tip, 100);
        assert_eq!(lag, 10);
        assert_eq!(status, "syncing");
        assert_eq!(percent, Some(90.0));
    }

    #[test]
    fn spend_cache_summary_reports_error_status_when_present() {
        let (ready, effective_tip, lag, status, percent, last_error) =
            summarize_spend_cache_status(90, 100, Some(100), Some("scan failed".to_string()));
        assert!(!ready);
        assert_eq!(effective_tip, 100);
        assert_eq!(lag, 10);
        assert_eq!(status, "error");
        assert_eq!(percent, Some(90.0));
        assert_eq!(last_error.as_deref(), Some("scan failed"));
    }
}
