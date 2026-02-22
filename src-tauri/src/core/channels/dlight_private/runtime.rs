use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint, Uri};
use zcash_client_backend::data_api::BlockMetadata;
use zcash_client_backend::encoding::decode_extended_spending_key;
use zcash_client_backend::keys::{UnifiedFullViewingKey, UnifiedSpendingKey};
use zcash_client_backend::proto::compact_formats::CompactBlock;
use zcash_client_backend::proto::service::{
    self, compact_tx_streamer_client::CompactTxStreamerClient,
};
use zcash_client_backend::scanning::{scan_block, Nullifiers, ScanningKeys};
use zcash_protocol::consensus::{
    BlockHeight, BranchId, MainNetwork, NetworkType, NetworkUpgrade, Parameters, TestNetwork,
};
use zcash_protocol::constants::{mainnet, testnet};
use zip32::{AccountId, Scope};

use super::store::{
    ensure_layout, load_block_meta, load_state, resolve_paths, save_block_meta, save_state,
    unix_timestamp_secs, StoredBlockMeta, StoredNote, StoredRuntimeState, StoredTransaction,
    STORAGE_SCHEMA_VERSION,
};
use super::{normalize_grpc_endpoint, DlightInfo, DlightRuntimeRequest};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

const VERUS_MAINNET_CHAIN_NAME: &str = "vrsc";
const VERUS_MAINNET_CHAIN_NAME_LEGACY: &str = "main";
const VERUS_TESTNET_CHAIN_NAME: &str = "vrsctest";
const VERUS_TESTNET_CHAIN_NAME_LEGACY: &str = "test";
const VERUS_MAINNET_SAPLING_ACTIVATION_HEIGHT: u64 = 227_520;
const VERUS_TESTNET_SAPLING_ACTIVATION_HEIGHT: u64 = 1;
const SYNC_BATCH_SIZE: u64 = 600;
const MAX_RETRY_BACKOFF_SECS: u64 = 45;
const DIAL_CONNECT_TIMEOUT_SECS: u64 = 8;
const DIAL_RPC_TIMEOUT_SECS: u64 = 12;
const DNS_LOOKUP_PASSES: usize = 3;
const STALLED_SYNC_THRESHOLD_SECS: u64 = 90;
const FATAL_RETRY_BUDGET: u32 = 10;
const SYNC_LOOP_SLEEP_SYNCING_SECS: u64 = 4;
const SYNC_LOOP_SLEEP_SYNCED_SECS: u64 = 30;
const CONTINUITY_REWIND_BLOCKS: u64 = 20;

#[derive(Debug)]
enum SyncRangeError {
    Continuity { at_height: u64 },
    TreeSizeUnknown { at_height: u64 },
    Wallet(WalletError),
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

#[derive(Debug, Clone, Copy)]
struct VerusNetworkExpectations {
    accepted_chain_names: &'static [&'static str],
    sapling_activation_height: u64,
}

fn verus_network_expectations(network: WalletNetwork) -> VerusNetworkExpectations {
    match network {
        WalletNetwork::Mainnet => VerusNetworkExpectations {
            accepted_chain_names: &[VERUS_MAINNET_CHAIN_NAME, VERUS_MAINNET_CHAIN_NAME_LEGACY],
            sapling_activation_height: VERUS_MAINNET_SAPLING_ACTIVATION_HEIGHT,
        },
        WalletNetwork::Testnet => VerusNetworkExpectations {
            accepted_chain_names: &[VERUS_TESTNET_CHAIN_NAME, VERUS_TESTNET_CHAIN_NAME_LEGACY],
            sapling_activation_height: VERUS_TESTNET_SAPLING_ACTIVATION_HEIGHT,
        },
    }
}

fn verus_scan_params(network: WalletNetwork) -> VerusConsensusParams {
    let expectations = verus_network_expectations(network);
    let network_type = match network {
        WalletNetwork::Mainnet => NetworkType::Main,
        WalletNetwork::Testnet => NetworkType::Test,
    };

    VerusConsensusParams {
        network_type,
        sapling_activation_height: BlockHeight::from_u32(
            expectations
                .sapling_activation_height
                .try_into()
                .expect("known verus sapling activation heights must fit in u32"),
        ),
    }
}

fn verus_birthday_floor(network: WalletNetwork) -> u64 {
    verus_network_expectations(network)
        .sapling_activation_height
        .saturating_sub(1)
}

fn normalize_chain_name(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn validate_server_chain_info(
    network: WalletNetwork,
    lightd_info: &service::LightdInfo,
) -> Result<(), String> {
    let expectations = verus_network_expectations(network);
    let actual_chain_name = normalize_chain_name(&lightd_info.chain_name);
    let chain_name_ok = expectations
        .accepted_chain_names
        .iter()
        .any(|expected| actual_chain_name == *expected);
    if !chain_name_ok {
        return Err(format!(
            "dlight endpoint chain mismatch: expected one of {:?}, got '{}' (raw='{}')",
            expectations.accepted_chain_names, actual_chain_name, lightd_info.chain_name
        ));
    }

    if lightd_info.sapling_activation_height != expectations.sapling_activation_height {
        return Err(format!(
            "dlight endpoint sapling activation mismatch for network {:?}: expected {}, got {}",
            network, expectations.sapling_activation_height, lightd_info.sapling_activation_height
        ));
    }

    let server_branch = lightd_info.consensus_branch_id().ok_or_else(|| {
        format!(
            "dlight endpoint returned an unknown consensus branch id '{}'",
            lightd_info.consensus_branch_id
        )
    })?;
    if server_branch != BranchId::Sapling {
        return Err(format!(
            "dlight endpoint consensus branch mismatch: expected Sapling (76b809bb), got {:?} (raw='{}')",
            server_branch, lightd_info.consensus_branch_id
        ));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeStatusKind {
    Initializing,
    Syncing,
    Synced,
    Error,
}

#[derive(Debug, Clone)]
pub struct RuntimeTransaction {
    pub txid: String,
    pub net_sats: i128,
    pub block_height: u64,
    pub block_time: u64,
}

#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    pub status_kind: RuntimeStatusKind,
    pub info: DlightInfo,
    pub scanned_height: u64,
    pub chain_tip_height: Option<u64>,
    pub estimated_tip_height: Option<u64>,
    pub last_error: Option<String>,
    pub last_updated: u64,
    pub last_progress_at: Option<u64>,
    pub last_tip_probe_at: Option<u64>,
    pub consecutive_failures: u32,
    pub scan_rate_blocks_per_sec: Option<f64>,
    pub stalled: bool,
    pub confirmed_sats: i128,
    pub pending_sats: i128,
    pub total_sats: i128,
    pub transactions: Vec<RuntimeTransaction>,
}

impl Default for RuntimeSnapshot {
    fn default() -> Self {
        Self {
            status_kind: RuntimeStatusKind::Initializing,
            info: DlightInfo {
                blocks: Some(0),
                longest_chain: Some(0),
                syncing: true,
                percent: Some(0.0),
                status_kind: Some("initializing".to_string()),
                last_updated: Some(unix_timestamp_secs()),
                last_progress_at: None,
                stalled: Some(false),
                scan_rate_blocks_per_sec: None,
            },
            scanned_height: 0,
            chain_tip_height: Some(0),
            estimated_tip_height: Some(0),
            last_error: None,
            last_updated: unix_timestamp_secs(),
            last_progress_at: None,
            last_tip_probe_at: None,
            consecutive_failures: 0,
            scan_rate_blocks_per_sec: None,
            stalled: false,
            confirmed_sats: 0,
            pending_sats: 0,
            total_sats: 0,
            transactions: vec![],
        }
    }
}

#[derive(Debug, Clone)]
struct RuntimeContext {
    endpoint: String,
    coin_id: String,
    network: WalletNetwork,
    seed_material: String,
    app_data_dir: PathBuf,
    account_hash: String,
}

#[derive(Debug)]
pub struct RuntimeHandle {
    snapshot: Arc<Mutex<RuntimeSnapshot>>,
    cancel_token: CancellationToken,
    runtime_task: JoinHandle<()>,
    spend_cache_task: JoinHandle<()>,
}

fn runtime_registry() -> &'static Mutex<HashMap<String, Arc<RuntimeHandle>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Arc<RuntimeHandle>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn dial_rotation_counter() -> &'static AtomicUsize {
    static COUNTER: OnceLock<AtomicUsize> = OnceLock::new();
    COUNTER.get_or_init(|| AtomicUsize::new(0))
}

fn scale_percent(scanned_height: u64, estimated_tip: u64) -> f64 {
    if estimated_tip == 0 {
        return 0.0;
    }

    ((scanned_height as f64 / estimated_tip as f64) * 100.0).clamp(0.0, 100.0)
}

fn prior_block_metadata_from_state(state: &StoredRuntimeState) -> Option<BlockMetadata> {
    let sapling_tree_size = state.sapling_tree_size?;
    let block_hash_hex = state.block_hash_hex.as_ref()?;
    let block_hash_bytes = hex::decode(block_hash_hex).ok()?;
    if block_hash_bytes.len() != 32 {
        return None;
    }

    // Rehydrate the prior block metadata from persisted runtime state so a resumed
    // sync can continue across ranges even when the first new block lacks chain metadata.
    let synthetic_block = CompactBlock {
        proto_version: 0,
        height: state.scanned_height,
        hash: block_hash_bytes,
        prev_hash: vec![],
        time: 0,
        header: vec![],
        vtx: vec![],
        chain_metadata: None,
    };

    Some(BlockMetadata::from_parts(
        synthetic_block.height(),
        synthetic_block.hash(),
        Some(sapling_tree_size),
    ))
}

fn snapshot_from_state(
    state: &StoredRuntimeState,
    chain_tip_height: u64,
    estimated_tip_height: u64,
    status_kind: RuntimeStatusKind,
    last_error: Option<String>,
    last_progress_at: Option<u64>,
    last_tip_probe_at: Option<u64>,
    consecutive_failures: u32,
    scan_rate_blocks_per_sec: Option<f64>,
) -> RuntimeSnapshot {
    let confirmed_sats: i128 = state
        .notes
        .iter()
        .filter(|note| note.spent_height.is_none())
        .map(|note| note.value_sats as i128)
        .sum();

    let mut transactions = state
        .transactions
        .iter()
        .map(|transaction| RuntimeTransaction {
            txid: transaction.txid.clone(),
            net_sats: transaction.net_sats,
            block_height: transaction.block_height,
            block_time: transaction.block_time,
        })
        .collect::<Vec<_>>();

    transactions.sort_by(|left, right| {
        right
            .block_height
            .cmp(&left.block_height)
            .then(right.block_time.cmp(&left.block_time))
            .then(left.txid.cmp(&right.txid))
    });

    let now = unix_timestamp_secs();
    let blocks = Some(state.scanned_height.min(chain_tip_height));
    let longest_chain = Some(estimated_tip_height.max(chain_tip_height));
    let percent = Some(scale_percent(
        state.scanned_height.min(chain_tip_height),
        estimated_tip_height.max(chain_tip_height),
    ));
    let syncing = matches!(
        status_kind,
        RuntimeStatusKind::Initializing | RuntimeStatusKind::Syncing
    ) || state.scanned_height < chain_tip_height;
    let stalled = syncing
        && chain_tip_height > 0
        && state.scanned_height < chain_tip_height
        && last_progress_at
            .map(|value| now.saturating_sub(value) >= STALLED_SYNC_THRESHOLD_SECS)
            .unwrap_or(false);

    let status_kind_value = match status_kind {
        RuntimeStatusKind::Initializing => "initializing",
        RuntimeStatusKind::Syncing => "syncing",
        RuntimeStatusKind::Synced => "synced",
        RuntimeStatusKind::Error => "error",
    };

    RuntimeSnapshot {
        status_kind,
        info: DlightInfo {
            blocks,
            longest_chain,
            syncing,
            percent,
            status_kind: Some(status_kind_value.to_string()),
            last_updated: Some(now),
            last_progress_at,
            stalled: Some(stalled),
            scan_rate_blocks_per_sec,
        },
        scanned_height: state.scanned_height,
        chain_tip_height: Some(chain_tip_height),
        estimated_tip_height: Some(estimated_tip_height),
        last_error,
        last_updated: now,
        last_progress_at,
        last_tip_probe_at,
        consecutive_failures,
        scan_rate_blocks_per_sec,
        stalled,
        confirmed_sats,
        pending_sats: 0,
        total_sats: confirmed_sats,
        transactions,
    }
}

fn network_hrp_spending_key(network: WalletNetwork) -> &'static str {
    match network {
        WalletNetwork::Mainnet => mainnet::HRP_SAPLING_EXTENDED_SPENDING_KEY,
        WalletNetwork::Testnet => testnet::HRP_SAPLING_EXTENDED_SPENDING_KEY,
    }
}

fn build_scanning_keys_from_seed(
    seed_material: &str,
    network: WalletNetwork,
) -> Result<ScanningKeys<u32, (u32, Scope)>, WalletError> {
    let normalized = seed_material.trim();

    if normalized.starts_with("secret-extended-key-") {
        let extsk = decode_extended_spending_key(network_hrp_spending_key(network), normalized)
            .map_err(|_| WalletError::InvalidImportText)?;
        let sapling_fvk = extsk.to_extended_full_viewing_key();
        let ufvk = UnifiedFullViewingKey::from_sapling_extended_full_viewing_key(sapling_fvk)
            .map_err(|_| WalletError::InvalidImportText)?;
        return Ok(ScanningKeys::from_account_ufvks(vec![(0u32, ufvk)]));
    }

    let mnemonic =
        bip39::Mnemonic::parse(normalized).map_err(|_| WalletError::InvalidSeedPhrase)?;
    let seed = mnemonic.to_seed_normalized("").to_vec();

    let usk = match network {
        WalletNetwork::Mainnet => {
            UnifiedSpendingKey::from_seed(&MainNetwork, &seed, AccountId::ZERO)
                .map_err(|_| WalletError::InvalidSeedPhrase)?
        }
        WalletNetwork::Testnet => {
            UnifiedSpendingKey::from_seed(&TestNetwork, &seed, AccountId::ZERO)
                .map_err(|_| WalletError::InvalidSeedPhrase)?
        }
    };

    Ok(ScanningKeys::from_account_ufvks(vec![(
        0u32,
        usk.to_unified_full_viewing_key(),
    )]))
}

fn track_tx_spends(
    compact_block: &CompactBlock,
    notes_by_nullifier: &mut HashMap<String, StoredNote>,
) -> HashMap<String, u64> {
    let mut spent_by_txid = HashMap::<String, u64>::new();

    for transaction in &compact_block.vtx {
        let txid = transaction.txid().to_string();
        let mut spent_value_sats = 0u64;

        for spend in &transaction.spends {
            if spend.nf.len() != 32 {
                continue;
            }
            let nullifier_hex = hex::encode(&spend.nf);
            if let Some(note) = notes_by_nullifier.get_mut(&nullifier_hex) {
                if note.spent_height.is_none() {
                    note.spent_height = Some(compact_block.height);
                    spent_value_sats = spent_value_sats.saturating_add(note.value_sats);
                }
            }
        }

        if spent_value_sats > 0 {
            spent_by_txid.insert(txid, spent_value_sats);
        }
    }

    spent_by_txid
}

fn apply_scanned_block(
    compact_block: &CompactBlock,
    scanned_block: zcash_client_backend::data_api::ScannedBlock<u32>,
    state: &mut StoredRuntimeState,
    notes_by_nullifier: &mut HashMap<String, StoredNote>,
    txs_by_txid: &mut HashMap<String, StoredTransaction>,
) {
    let spent_by_txid = track_tx_spends(compact_block, notes_by_nullifier);

    for wallet_tx in scanned_block.transactions() {
        let txid = wallet_tx.txid().to_string();
        let block_height = scanned_block.height().into();
        let block_time = scanned_block.block_time() as u64;

        let mut received_sats: u64 = 0;
        for output in wallet_tx.sapling_outputs() {
            let value_sats = output.note().value().inner();
            received_sats = received_sats.saturating_add(value_sats);

            let Some(nullifier) = output.nf() else {
                continue;
            };
            let nullifier_hex = hex::encode(nullifier.as_ref());
            notes_by_nullifier
                .entry(nullifier_hex.clone())
                .or_insert_with(|| StoredNote {
                    nullifier_hex,
                    value_sats,
                    received_height: block_height,
                    spent_height: None,
                    received_txid: txid.clone(),
                });
        }

        let spent_sats = spent_by_txid.get(&txid).copied().unwrap_or(0);
        if received_sats == 0 && spent_sats == 0 {
            continue;
        }

        let net_sats = i128::from(received_sats) - i128::from(spent_sats);
        txs_by_txid.insert(
            txid.clone(),
            StoredTransaction {
                txid,
                net_sats,
                block_height,
                block_time,
            },
        );
    }

    state.scanned_height = scanned_block.height().into();
    let metadata = scanned_block.to_block_metadata();
    state.sapling_tree_size = metadata.sapling_tree_size();
    state.block_hash_hex = Some(hex::encode(metadata.block_hash().0));
}

fn maybe_seed_prior_metadata_for_sapling_activation(
    network: WalletNetwork,
    compact_block: &CompactBlock,
    prior_block_metadata: &mut Option<BlockMetadata>,
) {
    if prior_block_metadata.is_some() || compact_block.chain_metadata.is_some() {
        return;
    }

    let activation_height = verus_network_expectations(network).sapling_activation_height;
    if compact_block.height != activation_height {
        return;
    }

    let synthetic_prev_height = activation_height.saturating_sub(1);
    let Ok(synthetic_prev_height_u32) = u32::try_from(synthetic_prev_height) else {
        return;
    };

    // Some Verus lightwalletd servers omit chain metadata exactly at Sapling activation.
    // Seed prior metadata from this block's prev hash with tree size 0 to bootstrap scanning.
    eprintln!(
        "[dlight_private] seeding synthetic prior metadata at sapling activation height={} prev_height={}",
        activation_height, synthetic_prev_height
    );
    *prior_block_metadata = Some(BlockMetadata::from_parts(
        BlockHeight::from_u32(synthetic_prev_height_u32),
        compact_block.prev_hash(),
        Some(0),
    ));
}

async fn refresh_tip_info(
    endpoint: &str,
    network: WalletNetwork,
) -> Result<(CompactTxStreamerClient<Channel>, u64, u64), String> {
    let grpc_endpoint = normalize_grpc_endpoint(endpoint).map_err(|error| error.to_string())?;
    let parsed_uri: Uri = grpc_endpoint
        .parse()
        .map_err(|_| "invalid dlight endpoint URL".to_string())?;
    let host = parsed_uri
        .host()
        .ok_or_else(|| "dlight endpoint has no host".to_string())?
        .to_string();
    let port = parsed_uri.port_u16().unwrap_or(443);
    let is_https = parsed_uri.scheme_str() == Some("https");

    let mut resolved_addrs = Vec::<SocketAddr>::new();
    for pass in 0..DNS_LOOKUP_PASSES {
        match tokio::net::lookup_host((host.as_str(), port)).await {
            Ok(pass_addrs) => {
                for socket_addr in pass_addrs {
                    if !resolved_addrs
                        .iter()
                        .any(|existing| existing == &socket_addr)
                    {
                        resolved_addrs.push(socket_addr);
                    }
                }
            }
            Err(error) => {
                eprintln!(
                    "[dlight_private] endpoint lookup failed on pass {} for {}:{}: {}",
                    pass + 1,
                    host,
                    port,
                    error
                );
            }
        }

        if pass + 1 < DNS_LOOKUP_PASSES {
            tokio::task::yield_now().await;
        }
    }

    let mut ip_candidates = resolved_addrs
        .iter()
        .map(|socket_addr| format!("https://{socket_addr}"))
        .collect::<Vec<_>>();

    if !ip_candidates.is_empty() {
        let rotate_from = dial_rotation_counter().fetch_add(1, Ordering::Relaxed);
        let rotate_offset = rotate_from % ip_candidates.len();
        ip_candidates.rotate_left(rotate_offset);
    }

    let mut candidates = ip_candidates;
    if !candidates
        .iter()
        .any(|candidate| candidate == &grpc_endpoint)
    {
        candidates.push(grpc_endpoint.clone());
    }

    eprintln!(
        "[dlight_private] tip probe for {}:{} candidates={}",
        host,
        port,
        candidates.len()
    );

    let mut last_metadata_error: Option<String> = None;
    for (index, candidate) in candidates.iter().enumerate() {
        eprintln!(
            "[dlight_private] tip probe attempt {}/{} via {}",
            index + 1,
            candidates.len(),
            candidate
        );

        let endpoint_builder = match Endpoint::from_shared(candidate.clone()) {
            Ok(builder) => builder
                .origin(parsed_uri.clone())
                .connect_timeout(Duration::from_secs(DIAL_CONNECT_TIMEOUT_SECS))
                .timeout(Duration::from_secs(DIAL_RPC_TIMEOUT_SECS))
                .tcp_nodelay(true),
            Err(error) => {
                eprintln!(
                    "[dlight_private] tip probe endpoint parse failed for {}: {}",
                    candidate, error
                );
                continue;
            }
        };

        let endpoint_builder = if is_https {
            match endpoint_builder.tls_config(
                ClientTlsConfig::new()
                    .with_webpki_roots()
                    .domain_name(host.clone()),
            ) {
                Ok(builder) => builder,
                Err(error) => {
                    eprintln!(
                        "[dlight_private] tip probe tls config failed for {}: {}",
                        candidate, error
                    );
                    continue;
                }
            }
        } else {
            endpoint_builder
        };

        let connect_result = tokio::time::timeout(
            Duration::from_secs(DIAL_CONNECT_TIMEOUT_SECS + 1),
            endpoint_builder.connect(),
        )
        .await;

        let channel = match connect_result {
            Ok(Ok(channel)) => channel,
            Ok(Err(error)) => {
                eprintln!(
                    "[dlight_private] tip probe connect failed for {}: {}",
                    candidate, error
                );
                continue;
            }
            Err(_) => {
                eprintln!(
                    "[dlight_private] tip probe connect timed out for {}",
                    candidate
                );
                continue;
            }
        };

        let mut client = CompactTxStreamerClient::new(channel);
        let info_result = tokio::time::timeout(
            Duration::from_secs(DIAL_RPC_TIMEOUT_SECS),
            client.get_lightd_info(service::Empty {}),
        )
        .await;

        let lightd_info = match info_result {
            Ok(Ok(info)) => info.into_inner(),
            Ok(Err(error)) => {
                eprintln!(
                    "[dlight_private] tip probe rpc failed for {}: {}",
                    candidate, error
                );
                continue;
            }
            Err(_) => {
                eprintln!("[dlight_private] tip probe rpc timed out for {}", candidate);
                continue;
            }
        };

        if let Err(error) = validate_server_chain_info(network, &lightd_info) {
            eprintln!(
                "[dlight_private] tip probe metadata validation failed for {}: {}",
                candidate, error
            );
            last_metadata_error = Some(error);
            continue;
        }

        let chain_tip_height = lightd_info.block_height;
        let estimated_tip_height = if lightd_info.estimated_height > 0 {
            lightd_info.estimated_height
        } else {
            chain_tip_height
        };

        eprintln!(
            "[dlight_private] tip probe success via {} chain_tip={} estimated_tip={}",
            candidate, chain_tip_height, estimated_tip_height
        );

        return Ok((client, chain_tip_height, estimated_tip_height));
    }

    eprintln!(
        "[dlight_private] tip probe exhausted all candidates for {}:{}",
        host, port
    );
    Err(last_metadata_error.unwrap_or_else(|| {
        format!(
            "failed to connect to dlight endpoint {}:{} with valid chain metadata",
            host, port
        )
    }))
}

async fn sync_range(
    client: &mut CompactTxStreamerClient<Channel>,
    start_height: u64,
    end_height: u64,
    seed_material: &str,
    state: &mut StoredRuntimeState,
    notes_by_nullifier: &mut HashMap<String, StoredNote>,
    txs_by_txid: &mut HashMap<String, StoredTransaction>,
    prior_block_metadata: &mut Option<BlockMetadata>,
    cancel_token: &CancellationToken,
    network: WalletNetwork,
) -> Result<(), SyncRangeError> {
    let verus_params = verus_scan_params(network);
    let mut compact_blocks = Vec::<CompactBlock>::new();

    let block_range = service::BlockRange {
        start: Some(service::BlockId {
            height: start_height,
            hash: vec![],
        }),
        end: Some(service::BlockId {
            height: end_height,
            hash: vec![],
        }),
    };

    let mut stream = client
        .get_block_range(block_range)
        .await
        .map_err(|_| SyncRangeError::Wallet(WalletError::NetworkError))?
        .into_inner();

    while let Some(compact_block) =
        tokio::time::timeout(Duration::from_secs(DIAL_RPC_TIMEOUT_SECS), stream.message())
            .await
            .map_err(|_| SyncRangeError::Wallet(WalletError::NetworkError))?
            .map_err(|_| SyncRangeError::Wallet(WalletError::NetworkError))?
    {
        if cancel_token.is_cancelled() {
            break;
        }
        compact_blocks.push(compact_block);
    }

    let scanning_keys =
        build_scanning_keys_from_seed(seed_material, network).map_err(SyncRangeError::Wallet)?;

    for compact_block in compact_blocks {
        maybe_seed_prior_metadata_for_sapling_activation(
            network,
            &compact_block,
            prior_block_metadata,
        );

        let nullifiers = Nullifiers::empty();
        let scanned = scan_block(
            &verus_params,
            compact_block.clone(),
            &scanning_keys,
            &nullifiers,
            (*prior_block_metadata).as_ref(),
        );

        let scanned_block = match scanned {
            Ok(block) => block,
            Err(scan_error) => {
                eprintln!(
                    "[dlight_private] scan failed at height={} continuity={} error={:?}",
                    compact_block.height,
                    scan_error.is_continuity_error(),
                    scan_error
                );
                let scan_error_debug = format!("{scan_error:?}");
                if scan_error.is_continuity_error() {
                    return Err(SyncRangeError::Continuity {
                        at_height: compact_block.height,
                    });
                }
                if scan_error_debug.contains("TreeSizeUnknown") {
                    return Err(SyncRangeError::TreeSizeUnknown {
                        at_height: compact_block.height,
                    });
                }
                return Err(SyncRangeError::Wallet(WalletError::OperationFailed));
            }
        };

        *prior_block_metadata = Some(scanned_block.to_block_metadata());
        apply_scanned_block(
            &compact_block,
            scanned_block,
            state,
            notes_by_nullifier,
            txs_by_txid,
        );
    }

    Ok(())
}

fn sync_state_maps_to_vectors(
    state: &mut StoredRuntimeState,
    notes_by_nullifier: &HashMap<String, StoredNote>,
    txs_by_txid: &HashMap<String, StoredTransaction>,
) {
    state.notes = notes_by_nullifier.values().cloned().collect();
    state.transactions = txs_by_txid.values().cloned().collect();
}

fn persist_runtime_state(paths: &super::store::DlightStoragePaths, state: &StoredRuntimeState) {
    let _ = save_state(paths, state);
    let _ = save_block_meta(
        paths,
        &StoredBlockMeta {
            schema_version: STORAGE_SCHEMA_VERSION,
            scanned_height: state.scanned_height,
            sapling_tree_size: state.sapling_tree_size,
            block_hash_hex: state.block_hash_hex.clone(),
            last_updated: state.last_updated,
        },
    );
}

fn rewind_runtime_state_after_continuity(
    state: &mut StoredRuntimeState,
    notes_by_nullifier: &mut HashMap<String, StoredNote>,
    txs_by_txid: &mut HashMap<String, StoredTransaction>,
    birthday_floor: u64,
) -> u64 {
    let rewind_to = state
        .scanned_height
        .saturating_sub(CONTINUITY_REWIND_BLOCKS)
        .max(birthday_floor);

    notes_by_nullifier.retain(|_, note| note.received_height <= rewind_to);
    for note in notes_by_nullifier.values_mut() {
        if note.spent_height.is_some_and(|height| height > rewind_to) {
            note.spent_height = None;
        }
    }
    txs_by_txid.retain(|_, tx| tx.block_height <= rewind_to);

    state.scanned_height = rewind_to;
    state.sapling_tree_size = None;
    state.block_hash_hex = None;
    state.last_updated = unix_timestamp_secs();
    sync_state_maps_to_vectors(state, notes_by_nullifier, txs_by_txid);

    rewind_to
}

fn reset_runtime_state_to_birthday(
    state: &mut StoredRuntimeState,
    notes_by_nullifier: &mut HashMap<String, StoredNote>,
    txs_by_txid: &mut HashMap<String, StoredTransaction>,
    birthday_floor: u64,
) -> u64 {
    notes_by_nullifier.clear();
    txs_by_txid.clear();
    state.scanned_height = birthday_floor;
    state.sapling_tree_size = None;
    state.block_hash_hex = None;
    state.last_updated = unix_timestamp_secs();
    sync_state_maps_to_vectors(state, notes_by_nullifier, txs_by_txid);
    birthday_floor
}

async fn run_sync_loop(
    context: RuntimeContext,
    snapshot: Arc<Mutex<RuntimeSnapshot>>,
    cancel: CancellationToken,
) {
    let paths = resolve_paths(
        &context.app_data_dir,
        context.network,
        &context.account_hash,
        &context.coin_id,
    );

    if let Err(error) = ensure_layout(&paths) {
        let mut guard = snapshot.lock().expect("runtime snapshot lock");
        guard.status_kind = RuntimeStatusKind::Error;
        guard.last_error = Some(format!("{:?}", error));
        guard.last_updated = unix_timestamp_secs();
        guard.info.status_kind = Some("error".to_string());
        guard.info.syncing = false;
        guard.consecutive_failures = FATAL_RETRY_BUDGET;
        return;
    }

    if let Err(error) = build_scanning_keys_from_seed(&context.seed_material, context.network) {
        let mut guard = snapshot.lock().expect("runtime snapshot lock");
        guard.status_kind = RuntimeStatusKind::Error;
        guard.last_error = Some(error.to_string());
        guard.last_updated = unix_timestamp_secs();
        guard.info.status_kind = Some("error".to_string());
        guard.info.syncing = false;
        guard.consecutive_failures = FATAL_RETRY_BUDGET;
        return;
    }

    let mut stored_state = load_state(&paths).unwrap_or_default();
    if let Ok(block_meta) = load_block_meta(&paths) {
        if stored_state.scanned_height < block_meta.scanned_height {
            stored_state.scanned_height = block_meta.scanned_height;
        }
        if stored_state.sapling_tree_size.is_none() {
            stored_state.sapling_tree_size = block_meta.sapling_tree_size;
        }
        if stored_state.block_hash_hex.is_none() {
            stored_state.block_hash_hex = block_meta.block_hash_hex;
        }
    }
    let mut notes_by_nullifier = stored_state
        .notes
        .iter()
        .cloned()
        .map(|note| (note.nullifier_hex.clone(), note))
        .collect::<HashMap<_, _>>();
    let mut txs_by_txid = stored_state
        .transactions
        .iter()
        .cloned()
        .map(|transaction| (transaction.txid.clone(), transaction))
        .collect::<HashMap<_, _>>();
    let mut prior_block_metadata = prior_block_metadata_from_state(&stored_state);

    let mut retry_count: u32 = 0;
    let mut consecutive_failures: u32 = 0;
    let mut last_progress_at = if stored_state.scanned_height > 0 {
        Some(stored_state.last_updated)
    } else {
        None
    };
    let mut last_tip_probe_at: Option<u64>;
    let mut scan_rate_blocks_per_sec: Option<f64> = None;
    let mut progress_rate_started_at = Instant::now();

    loop {
        if cancel.is_cancelled() {
            return;
        }

        last_tip_probe_at = Some(unix_timestamp_secs());
        match refresh_tip_info(&context.endpoint, context.network).await {
            Ok((mut client, chain_tip_height, estimated_tip_height)) => {
                retry_count = 0;
                consecutive_failures = 0;
                last_tip_probe_at = Some(unix_timestamp_secs());

                let birthday_floor = verus_birthday_floor(context.network);
                if stored_state.scanned_height < birthday_floor {
                    stored_state.scanned_height = birthday_floor;
                    prior_block_metadata = None;
                }

                if stored_state.scanned_height > chain_tip_height {
                    stored_state.scanned_height = chain_tip_height;
                    prior_block_metadata = None;
                }

                if prior_block_metadata
                    .as_ref()
                    .map(|metadata| u64::from(metadata.block_height()))
                    != Some(stored_state.scanned_height)
                {
                    prior_block_metadata = None;
                }

                if stored_state.scanned_height < chain_tip_height {
                    {
                        let mut guard = snapshot.lock().expect("runtime snapshot lock");
                        *guard = snapshot_from_state(
                            &stored_state,
                            chain_tip_height,
                            estimated_tip_height,
                            RuntimeStatusKind::Syncing,
                            None,
                            last_progress_at,
                            last_tip_probe_at,
                            consecutive_failures,
                            scan_rate_blocks_per_sec,
                        );
                    }

                    let mut current = stored_state.scanned_height.saturating_add(1);
                    let mut cycle_error: Option<String> = None;
                    while current <= chain_tip_height {
                        if cancel.is_cancelled() {
                            return;
                        }

                        let end = current
                            .saturating_add(SYNC_BATCH_SIZE.saturating_sub(1))
                            .min(chain_tip_height);

                        let pre_sync_height = stored_state.scanned_height;
                        let sync_result = sync_range(
                            &mut client,
                            current,
                            end,
                            &context.seed_material,
                            &mut stored_state,
                            &mut notes_by_nullifier,
                            &mut txs_by_txid,
                            &mut prior_block_metadata,
                            &cancel,
                            context.network,
                        )
                        .await;

                        match sync_result {
                            Err(SyncRangeError::Continuity { at_height }) => {
                                let rewind_to = rewind_runtime_state_after_continuity(
                                    &mut stored_state,
                                    &mut notes_by_nullifier,
                                    &mut txs_by_txid,
                                    birthday_floor,
                                );
                                prior_block_metadata = None;
                                persist_runtime_state(&paths, &stored_state);
                                eprintln!(
                                    "[dlight_private] continuity mismatch at height={} rewinding to {}",
                                    at_height, rewind_to
                                );
                                cycle_error = Some(format!(
                                    "continuity mismatch at height {at_height}; rewound to {rewind_to}"
                                ));
                                consecutive_failures = 0;
                                retry_count = 0;
                                last_progress_at = Some(unix_timestamp_secs());
                                progress_rate_started_at = Instant::now();

                                let mut guard = snapshot.lock().expect("runtime snapshot lock");
                                *guard = snapshot_from_state(
                                    &stored_state,
                                    chain_tip_height,
                                    estimated_tip_height,
                                    RuntimeStatusKind::Syncing,
                                    cycle_error.clone(),
                                    last_progress_at,
                                    last_tip_probe_at,
                                    consecutive_failures,
                                    scan_rate_blocks_per_sec,
                                );
                                break;
                            }
                            Err(SyncRangeError::TreeSizeUnknown { at_height }) => {
                                let reset_to = reset_runtime_state_to_birthday(
                                    &mut stored_state,
                                    &mut notes_by_nullifier,
                                    &mut txs_by_txid,
                                    birthday_floor,
                                );
                                prior_block_metadata = None;
                                persist_runtime_state(&paths, &stored_state);
                                eprintln!(
                                    "[dlight_private] tree size unknown at height={} resetting to birthday floor {}",
                                    at_height, reset_to
                                );
                                cycle_error = Some(format!(
                                    "tree size unknown at height {at_height}; reset to {reset_to}"
                                ));
                                consecutive_failures = 0;
                                retry_count = 0;
                                last_progress_at = Some(unix_timestamp_secs());
                                progress_rate_started_at = Instant::now();

                                let mut guard = snapshot.lock().expect("runtime snapshot lock");
                                *guard = snapshot_from_state(
                                    &stored_state,
                                    chain_tip_height,
                                    estimated_tip_height,
                                    RuntimeStatusKind::Syncing,
                                    cycle_error.clone(),
                                    last_progress_at,
                                    last_tip_probe_at,
                                    consecutive_failures,
                                    scan_rate_blocks_per_sec,
                                );
                                break;
                            }
                            Err(SyncRangeError::Wallet(error)) => {
                                eprintln!(
                                    "[dlight_private] sync range failed start={} end={} scanned_height={} error={:?}",
                                    current,
                                    end,
                                    stored_state.scanned_height,
                                    error
                                );
                                consecutive_failures = consecutive_failures.saturating_add(1);
                                cycle_error = Some(error.to_string());

                                let degraded_or_fatal_status =
                                    if consecutive_failures >= FATAL_RETRY_BUDGET {
                                        RuntimeStatusKind::Error
                                    } else {
                                        RuntimeStatusKind::Syncing
                                    };
                                let mut guard = snapshot.lock().expect("runtime snapshot lock");
                                *guard = snapshot_from_state(
                                    &stored_state,
                                    chain_tip_height,
                                    estimated_tip_height,
                                    degraded_or_fatal_status,
                                    cycle_error.clone(),
                                    last_progress_at,
                                    last_tip_probe_at,
                                    consecutive_failures,
                                    scan_rate_blocks_per_sec,
                                );
                                break;
                            }
                            Ok(()) => {}
                        }

                        if stored_state.scanned_height > pre_sync_height {
                            let advanced_blocks = stored_state.scanned_height - pre_sync_height;
                            let elapsed_secs = progress_rate_started_at.elapsed().as_secs_f64();
                            if advanced_blocks > 0 && elapsed_secs > 0.0 {
                                scan_rate_blocks_per_sec =
                                    Some(advanced_blocks as f64 / elapsed_secs);
                            }
                            last_progress_at = Some(unix_timestamp_secs());
                            progress_rate_started_at = Instant::now();
                        }

                        sync_state_maps_to_vectors(
                            &mut stored_state,
                            &notes_by_nullifier,
                            &txs_by_txid,
                        );
                        stored_state.last_updated = unix_timestamp_secs();
                        persist_runtime_state(&paths, &stored_state);

                        {
                            let mut guard = snapshot.lock().expect("runtime snapshot lock");
                            *guard = snapshot_from_state(
                                &stored_state,
                                chain_tip_height,
                                estimated_tip_height,
                                RuntimeStatusKind::Syncing,
                                cycle_error.clone(),
                                last_progress_at,
                                last_tip_probe_at,
                                consecutive_failures,
                                scan_rate_blocks_per_sec,
                            );
                        }

                        current = end.saturating_add(1);
                    }
                }

                sync_state_maps_to_vectors(&mut stored_state, &notes_by_nullifier, &txs_by_txid);
                stored_state.last_updated = unix_timestamp_secs();
                persist_runtime_state(&paths, &stored_state);

                {
                    let mut guard = snapshot.lock().expect("runtime snapshot lock");
                    let is_fatal = consecutive_failures >= FATAL_RETRY_BUDGET;
                    let runtime_status = if is_fatal {
                        RuntimeStatusKind::Error
                    } else if stored_state.scanned_height >= chain_tip_height {
                        RuntimeStatusKind::Synced
                    } else {
                        RuntimeStatusKind::Syncing
                    };
                    *guard = snapshot_from_state(
                        &stored_state,
                        chain_tip_height,
                        estimated_tip_height,
                        runtime_status,
                        if is_fatal {
                            Some("dlight endpoint retry budget exhausted".to_string())
                        } else {
                            None
                        },
                        last_progress_at,
                        last_tip_probe_at,
                        consecutive_failures,
                        scan_rate_blocks_per_sec,
                    );
                }

                let is_syncing_cycle = stored_state.scanned_height < chain_tip_height;
                let loop_sleep_secs = if is_syncing_cycle {
                    SYNC_LOOP_SLEEP_SYNCING_SECS
                } else {
                    SYNC_LOOP_SLEEP_SYNCED_SECS
                };
                tokio::time::sleep(Duration::from_secs(loop_sleep_secs)).await;
            }
            Err(error) => {
                retry_count = retry_count.saturating_add(1);
                consecutive_failures = consecutive_failures.saturating_add(1);
                let backoff = (2u64.saturating_pow(retry_count.min(5)))
                    .min(MAX_RETRY_BACKOFF_SECS)
                    .max(2);
                let is_fatal = consecutive_failures >= FATAL_RETRY_BUDGET;

                {
                    let (prior_chain_tip, prior_estimated_tip) = {
                        let guard = snapshot.lock().expect("runtime snapshot lock");
                        (
                            guard.chain_tip_height.unwrap_or_default(),
                            guard.estimated_tip_height.unwrap_or_default(),
                        )
                    };
                    let mut guard = snapshot.lock().expect("runtime snapshot lock");
                    *guard = snapshot_from_state(
                        &stored_state,
                        prior_chain_tip,
                        prior_estimated_tip,
                        if is_fatal {
                            RuntimeStatusKind::Error
                        } else {
                            RuntimeStatusKind::Syncing
                        },
                        Some(error.to_string()),
                        last_progress_at,
                        last_tip_probe_at,
                        consecutive_failures,
                        scan_rate_blocks_per_sec,
                    );
                }

                tokio::time::sleep(Duration::from_secs(backoff)).await;
            }
        }
    }
}

fn to_context(request: &DlightRuntimeRequest) -> RuntimeContext {
    RuntimeContext {
        endpoint: request.endpoint.clone(),
        coin_id: request.coin_id.clone(),
        network: request.network,
        seed_material: request.seed_material.clone(),
        app_data_dir: request.app_data_dir.clone(),
        account_hash: request.account_hash.clone(),
    }
}

pub fn ensure_runtime(request: &DlightRuntimeRequest) -> Arc<RuntimeHandle> {
    let runtime_key = request.runtime_key.clone();
    let context = to_context(request);

    if let Some(handle) = runtime_registry()
        .lock()
        .ok()
        .and_then(|registry| registry.get(&runtime_key).cloned())
    {
        return handle;
    }

    let snapshot = Arc::new(Mutex::new(RuntimeSnapshot::default()));
    let cancel_token = CancellationToken::new();
    let child_token = cancel_token.child_token();
    let spend_child_token = cancel_token.child_token();
    let runtime_snapshot = Arc::clone(&snapshot);
    let spend_request = request.clone();

    let runtime_task = tokio::spawn(async move {
        run_sync_loop(context, runtime_snapshot, child_token).await;
    });
    let spend_cache_task = tokio::spawn(async move {
        super::spend_sync::run_spend_cache_loop(spend_request, spend_child_token).await;
    });

    let handle = Arc::new(RuntimeHandle {
        snapshot,
        cancel_token,
        runtime_task,
        spend_cache_task,
    });

    if let Ok(mut registry) = runtime_registry().lock() {
        registry.insert(runtime_key, Arc::clone(&handle));
    }

    handle
}

pub fn get_runtime_snapshot(runtime_key: &str) -> Option<RuntimeSnapshot> {
    let handle = runtime_registry()
        .lock()
        .ok()
        .and_then(|registry| registry.get(runtime_key).cloned())?;

    handle.snapshot.lock().ok().map(|snapshot| snapshot.clone())
}

pub async fn stop_runtime(runtime_key: &str) {
    let handle = runtime_registry()
        .lock()
        .ok()
        .and_then(|mut registry| registry.remove(runtime_key));

    if let Some(handle) = handle {
        handle.cancel_token.cancel();
        handle.runtime_task.abort();
        handle.spend_cache_task.abort();
    }
}

pub async fn stop_all_runtimes() {
    let handles = runtime_registry()
        .lock()
        .ok()
        .map(|mut registry| {
            registry
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for handle in handles {
        handle.cancel_token.cancel();
        handle.runtime_task.abort();
        handle.spend_cache_task.abort();
    }
}
