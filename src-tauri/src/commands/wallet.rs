//
// Tauri command handlers for wallet operations
// Security: Thin wrappers that validate inputs and delegate to core logic
// Last Updated: Module 10 — unlock/session and update-engine start are decoupled

use secp256k1::SecretKey;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::core::address_book::manager as address_book_manager;
use crate::core::auth::SessionManager;
use crate::core::channels::btc::BtcProviderPool;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::coins::Channel;
use crate::core::coins::{CoinDefinition, CoinRegistry};
use crate::core::crypto::wif_encoding::decode_wif_unchecked_network;
use crate::core::updates::UpdateEngineStartConfig;
use crate::core::wallet::WalletManager;
use crate::core::{GuardSessionManager, PreflightStore, UpdateEngine};
use crate::types::wallet::WalletNetwork;
use crate::types::{
    AccountRecord, ActiveAssetsState, ActiveWalletResponse, AddressEndpointKind, AddressResponse,
    CoinScope, CoinScopesResult, CreateWalletRequest, CreateWalletResult, GenerateMnemonicRequest,
    ImportWalletTextRequest, MnemonicResult, WalletError, WalletListItem, WalletSecretKind,
};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct StartUpdateEngineRequest {
    pub include_transactions: Option<bool>,
    pub priority_coin_ids: Option<Vec<String>>,
    pub priority_channel_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
struct VrpcSystemDescriptor {
    system_id: String,
    system_ticker: String,
    system_display_name: String,
    is_root: bool,
}

const VRSC_MAINNET_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
const VRSC_TESTNET_SYSTEM_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";
const VETH_SYSTEM_ID: &str = "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X";

fn coin_supports_channel(coin: &CoinDefinition, channel: Channel) -> bool {
    coin.compatible_channels.iter().any(|item| *item == channel)
}

fn network_root_system_id(network: WalletNetwork) -> &'static str {
    if matches!(network, WalletNetwork::Testnet) {
        VRSC_TESTNET_SYSTEM_ID
    } else {
        VRSC_MAINNET_SYSTEM_ID
    }
}

fn coin_expands_vrpc_system_scope(coin: &CoinDefinition) -> bool {
    coin_supports_channel(coin, Channel::Vrpc)
        && coin.currency_id.eq_ignore_ascii_case(&coin.system_id)
}

fn normalize_watched_vrpc_address(address: &str, network: WalletNetwork) -> Option<String> {
    address_book_manager::normalize_destination_address(AddressEndpointKind::Vrpc, address, network)
        .ok()
}

fn dedupe_preserve_order(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<String>::new();

    for value in values {
        let key = value.trim().to_ascii_lowercase();
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        out.push(value);
    }

    out
}

fn canonical_coin_id_lookup(
    coin_registry: &CoinRegistry,
    network: WalletNetwork,
) -> HashMap<String, String> {
    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let mut lookup = HashMap::<String, String>::new();

    for coin in coin_registry
        .get_all()
        .into_iter()
        .filter(|coin| coin.is_testnet == is_testnet)
    {
        let key = coin.id.trim().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        lookup.entry(key).or_insert(coin.id);
    }

    lookup
}

fn sanitize_active_coin_ids(
    coin_registry: &CoinRegistry,
    network: WalletNetwork,
    coin_ids: &[String],
) -> Vec<String> {
    let lookup = canonical_coin_id_lookup(coin_registry, network);
    let mut seen = HashSet::<String>::new();
    let mut sanitized = Vec::<String>::new();

    for coin_id in coin_ids {
        let normalized = coin_id.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            continue;
        }

        let Some(canonical_id) = lookup.get(&normalized) else {
            continue;
        };
        let canonical_key = canonical_id.to_ascii_lowercase();
        if !seen.insert(canonical_key) {
            continue;
        }

        sanitized.push(canonical_id.clone());
    }

    sanitized
}

fn canonical_network_label_for_system(system_id: &str) -> Option<(String, String)> {
    if system_id.eq_ignore_ascii_case(VETH_SYSTEM_ID) {
        return Some(("ETH".to_string(), "Ethereum".to_string()));
    }
    if system_id.eq_ignore_ascii_case(VRSC_MAINNET_SYSTEM_ID) {
        return Some(("VRSC".to_string(), "Verus".to_string()));
    }
    if system_id.eq_ignore_ascii_case(VRSC_TESTNET_SYSTEM_ID) {
        return Some(("VRSCTEST".to_string(), "Verus Testnet".to_string()));
    }

    None
}

fn collect_vrpc_system_descriptors(
    coin_registry: &CoinRegistry,
    network: WalletNetwork,
    root_coin: &CoinDefinition,
    active_coin_ids: &[String],
) -> Vec<VrpcSystemDescriptor> {
    let root_system_id = network_root_system_id(network);
    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let mut allowed_systems = HashMap::<String, String>::new();
    let mut insert_system_id = |system_id: &str| {
        let trimmed = system_id.trim();
        if trimmed.is_empty() {
            return;
        }
        allowed_systems
            .entry(trimmed.to_ascii_lowercase())
            .or_insert_with(|| trimmed.to_string());
    };

    for coin_id in sanitize_active_coin_ids(coin_registry, network, active_coin_ids) {
        if let Some(coin) = coin_registry.find_by_id(&coin_id, is_testnet) {
            if coin_expands_vrpc_system_scope(&coin) {
                insert_system_id(&coin.system_id);
            }
        }
    }
    if coin_expands_vrpc_system_scope(root_coin) {
        insert_system_id(&root_coin.system_id);
    }
    insert_system_id(root_system_id);

    let vrpc_network_coins = coin_registry
        .get_all()
        .into_iter()
        .filter(|coin| coin.is_testnet == is_testnet && coin_supports_channel(coin, Channel::Vrpc))
        .collect::<Vec<_>>();

    let mut systems = allowed_systems
        .into_values()
        .map(|requested_system_id| {
            let native_system_definition = vrpc_network_coins.iter().find(|coin| {
                coin.system_id.eq_ignore_ascii_case(&requested_system_id)
                    && coin.currency_id.eq_ignore_ascii_case(&requested_system_id)
            });

            let system_id = native_system_definition
                .map(|coin| coin.system_id.clone())
                .unwrap_or_else(|| requested_system_id.clone());

            let (system_ticker, system_display_name) =
                if let Some((ticker, display_name)) = canonical_network_label_for_system(&system_id)
                {
                    (ticker, display_name)
                } else if let Some(system_coin) = native_system_definition {
                    let ticker = system_coin.display_ticker.trim();
                    let display_name = system_coin.display_name.trim();

                    (
                        if ticker.is_empty() {
                            system_id.clone()
                        } else {
                            ticker.to_string()
                        },
                        if display_name.is_empty() {
                            system_id.clone()
                        } else {
                            display_name.to_string()
                        },
                    )
                } else {
                    // Never fall back to arbitrary token labels for network names.
                    (system_id.clone(), system_id.clone())
                };

            VrpcSystemDescriptor {
                system_id: system_id.clone(),
                system_ticker,
                system_display_name,
                is_root: system_id.eq_ignore_ascii_case(root_system_id),
            }
        })
        .collect::<Vec<_>>();

    systems.sort_by(|left, right| {
        right
            .is_root
            .cmp(&left.is_root)
            .then(
                left.system_ticker
                    .to_ascii_lowercase()
                    .cmp(&right.system_ticker.to_ascii_lowercase()),
            )
            .then(
                left.system_id
                    .to_ascii_lowercase()
                    .cmp(&right.system_id.to_ascii_lowercase()),
            )
    });
    systems
}

fn channel_id_for_non_vrpc_coin(coin: &CoinDefinition) -> Option<String> {
    if coin_supports_channel(coin, Channel::Btc) {
        return Some(format!("btc.{}", coin.id));
    }
    if coin_supports_channel(coin, Channel::Eth) {
        return Some(format!("eth.{}", coin.id));
    }
    if coin_supports_channel(coin, Channel::Erc20) {
        return Some(format!("erc20.{}", coin.id));
    }
    None
}

fn address_for_non_vrpc_coin(
    coin: &CoinDefinition,
    addresses: &(String, String, String),
) -> String {
    if coin_supports_channel(coin, Channel::Btc) {
        return addresses.2.clone();
    }
    if coin_supports_channel(coin, Channel::Eth) || coin_supports_channel(coin, Channel::Erc20) {
        return addresses.1.clone();
    }
    addresses.0.clone()
}

/// Generate a new BIP39 mnemonic phrase
#[tauri::command(rename_all = "snake_case")]
pub async fn generate_mnemonic(
    request: GenerateMnemonicRequest,
    wallet_manager: State<'_, WalletManager>,
) -> Result<MnemonicResult, WalletError> {
    // Input validation
    if request.word_count != 24 {
        return Err(WalletError::InvalidSeedPhrase);
    }

    println!(
        "[WALLET] Generate mnemonic requested: {} words",
        request.word_count
    );

    // Delegate to core logic
    let seed_phrase = wallet_manager.generate_mnemonic(request.word_count).await?;

    println!("[WALLET] Mnemonic generation completed");

    Ok(MnemonicResult { seed_phrase })
}

/// Validate a BIP39 mnemonic phrase
#[tauri::command(rename_all = "snake_case")]
pub async fn validate_mnemonic(
    seed_phrase: String,
    wallet_manager: State<'_, WalletManager>,
) -> Result<bool, WalletError> {
    // Basic input validation
    if seed_phrase.trim().is_empty() {
        return Ok(false);
    }

    println!("[WALLET] Mnemonic validation requested");

    // Delegate to core logic
    let is_valid = wallet_manager.validate_mnemonic(&seed_phrase).await?;

    println!("[WALLET] Mnemonic validation completed: {}", is_valid);

    Ok(is_valid)
}

/// Get the BIP39 English word list used for mnemonic entry suggestions.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_mnemonic_wordlist(
    wallet_manager: State<'_, WalletManager>,
) -> Result<Vec<String>, WalletError> {
    wallet_manager.get_mnemonic_wordlist().await
}

fn normalize_hex_private_key_candidate(input: &str) -> Option<String> {
    let stripped = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
        .unwrap_or(input);
    if stripped.len() != 64 || !stripped.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let decoded = hex::decode(stripped).ok()?;
    if decoded.len() != 32 {
        return None;
    }
    SecretKey::from_slice(&decoded).ok()?;
    Some(stripped.to_lowercase())
}

fn classify_import_text(import_text: &str) -> Result<(WalletSecretKind, String), WalletError> {
    let trimmed = import_text.trim();
    if trimmed.is_empty() {
        return Err(WalletError::InvalidImportText);
    }

    if decode_wif_unchecked_network(trimmed).is_ok() {
        return Ok((WalletSecretKind::Wif, trimmed.to_string()));
    }

    if let Some(private_key_hex) = normalize_hex_private_key_candidate(trimmed) {
        return Ok((WalletSecretKind::PrivateKeyHex, private_key_hex));
    }

    // Parity behavior with valu-mobile: any remaining non-empty input is treated as seed text.
    Ok((WalletSecretKind::SeedText, trimmed.to_string()))
}

/// Create a new wallet with Stronghold encryption
#[tauri::command(rename_all = "snake_case")]
pub async fn create_wallet(
    request: CreateWalletRequest,
    password: String,
    wallet_manager: State<'_, WalletManager>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    app_handle: AppHandle,
) -> Result<CreateWalletResult, WalletError> {
    // Validate inputs
    request.validate()?;

    if password.trim().is_empty() {
        return Err(WalletError::InvalidPassword);
    }

    if password.len() < 7 {
        return Err(WalletError::PasswordTooShort);
    }

    println!("[WALLET] Create wallet requested: {}", request.wallet_name);

    if wallet_manager.wallet_exists(&request.wallet_name).await? {
        return Err(WalletError::WalletExists);
    }

    // Generate account ID
    let account_id = Uuid::new_v4().to_string();

    // Store seed in Stronghold
    let session = session_manager.lock().await;
    session
        .stronghold_store()
        .store_seed(&account_id, &request.seed_phrase, &password, &app_handle)
        .await?;
    drop(session);

    // Create account hash
    let account_hash = hash_account_id(&account_id);

    // Create metadata record
    let account = AccountRecord {
        id: account_id.clone(),
        account_hash,
        key_derivation_version: 1,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        network: request.network,
        emoji: request.emoji,
        color: request.color,
        secret_kind: WalletSecretKind::SeedText,
    };

    // Save metadata to file (using WalletManager's existing method)
    let metadata_path = wallet_manager.get_metadata_path(&request.wallet_name)?;
    let metadata_json = serde_json::to_string_pretty(&account)?;
    std::fs::write(metadata_path, metadata_json).map_err(|_| WalletError::OperationFailed)?;

    println!("[WALLET] Wallet created successfully: {}", account_id);

    Ok(CreateWalletResult {
        wallet_id: account_id,
        success: true,
    })
}

/// Import wallet from pasted private key or seed text.
#[tauri::command(rename_all = "snake_case")]
pub async fn import_wallet_text(
    request: ImportWalletTextRequest,
    password: String,
    wallet_manager: State<'_, WalletManager>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    app_handle: AppHandle,
) -> Result<CreateWalletResult, WalletError> {
    request.validate()?;

    if password.trim().is_empty() {
        return Err(WalletError::InvalidPassword);
    }

    if password.len() < 7 {
        return Err(WalletError::PasswordTooShort);
    }

    println!(
        "[WALLET] Import wallet text requested: {}",
        request.wallet_name
    );

    if wallet_manager.wallet_exists(&request.wallet_name).await? {
        return Err(WalletError::WalletExists);
    }

    let (secret_kind, secret_material) = classify_import_text(&request.import_text)?;

    let account_id = Uuid::new_v4().to_string();

    let session = session_manager.lock().await;
    session
        .stronghold_store()
        .store_seed(&account_id, &secret_material, &password, &app_handle)
        .await?;
    drop(session);

    let account_hash = hash_account_id(&account_id);
    let account = AccountRecord {
        id: account_id.clone(),
        account_hash,
        key_derivation_version: 1,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        network: request.network,
        emoji: request.emoji,
        color: request.color,
        secret_kind,
    };

    let metadata_path = wallet_manager.get_metadata_path(&request.wallet_name)?;
    let metadata_json = serde_json::to_string_pretty(&account)?;
    std::fs::write(metadata_path, metadata_json).map_err(|_| WalletError::OperationFailed)?;

    println!(
        "[WALLET] Wallet text import created account: {}",
        account_id
    );

    Ok(CreateWalletResult {
        wallet_id: account_id,
        success: true,
    })
}

/// Unlock wallet with password.
#[tauri::command(rename_all = "snake_case")]
pub async fn unlock_wallet(
    account_id: String,
    password: String,
    wallet_manager: State<'_, WalletManager>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    app_handle: AppHandle,
) -> Result<(), WalletError> {
    println!("[WALLET] Unlock wallet requested");

    let wallet = wallet_manager
        .get_account_record_by_account_id(&account_id)
        .await?
        .ok_or(WalletError::OperationFailed)?;

    let mut session = session_manager.lock().await;
    if let Err(err) = session
        .unlock(
            account_id,
            password,
            wallet.network,
            wallet.secret_kind,
            &app_handle,
        )
        .await
    {
        println!("[WALLET] Unlock failed: {:?}", err);
        return Err(err);
    }
    drop(session);

    println!("[WALLET] Wallet unlocked successfully");
    Ok(())
}

/// Start update engine polling after frontend event listeners are registered.
#[tauri::command(rename_all = "snake_case")]
pub async fn start_update_engine(
    request: Option<StartUpdateEngineRequest>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    btc_provider_pool: State<'_, Arc<BtcProviderPool>>,
    eth_provider_pool: State<'_, Arc<EthProviderPool>>,
    update_engine: State<'_, Arc<UpdateEngine>>,
    app_handle: AppHandle,
) -> Result<(), WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }
    drop(session);

    let request = request.unwrap_or_default();
    let start_config = UpdateEngineStartConfig {
        poll_transactions: request.include_transactions.unwrap_or(false),
        priority_coin_ids: request.priority_coin_ids.unwrap_or_default(),
        priority_channel_ids: request.priority_channel_ids.unwrap_or_default(),
    };

    update_engine
        .start(
            app_handle,
            session_manager.inner().clone(),
            coin_registry.inner().clone(),
            vrpc_provider_pool.inner().clone(),
            btc_provider_pool.inner().clone(),
            eth_provider_pool.inner().clone(),
            start_config,
        )
        .await;

    Ok(())
}

/// Lock wallet and zeroize keys. Stops update engine, clears preflight store.
#[tauri::command(rename_all = "snake_case")]
pub async fn lock_wallet(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    guard_session_manager: State<'_, Arc<Mutex<GuardSessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    update_engine: State<'_, Arc<UpdateEngine>>,
) -> Result<(), WalletError> {
    println!("[WALLET] Lock wallet requested");

    update_engine.stop().await;

    let mut session = session_manager.lock().await;
    session.lock();
    preflight_store.clear();
    drop(session);

    let mut guard = guard_session_manager.lock().await;
    guard.clear();

    println!("[WALLET] Wallet locked successfully");
    Ok(())
}

/// Get derived addresses for active account
#[tauri::command(rename_all = "snake_case")]
pub async fn get_addresses(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<AddressResponse, WalletError> {
    println!("[WALLET] Get addresses requested");

    let session = session_manager.lock().await;
    let (vrsc_address, eth_address, btc_address) = session.get_addresses()?;

    println!("[WALLET] Addresses retrieved");
    Ok(AddressResponse {
        vrsc_address,
        eth_address,
        btc_address,
    })
}

/// Check if wallet is unlocked
#[tauri::command(rename_all = "snake_case")]
pub async fn is_unlocked(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<bool, WalletError> {
    let session = session_manager.lock().await;
    Ok(session.is_unlocked())
}

/// Get active wallet display info for dashboard (when unlocked)
#[tauri::command(rename_all = "snake_case")]
pub async fn get_active_wallet(
    wallet_manager: State<'_, WalletManager>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<Option<ActiveWalletResponse>, WalletError> {
    let session = session_manager.lock().await;
    let account_id = match session.active_account_id() {
        Some(id) => id.clone(),
        None => return Ok(None),
    };
    drop(session);

    let wallet_name = wallet_manager.get_wallet_by_account_id(&account_id).await?;

    Ok(wallet_name.map(|w| ActiveWalletResponse {
        wallet_name: w.wallet_name,
        network: w.network,
        emoji: w.emoji,
        color: w.color,
    }))
}

/// Return watched read-only VRPC addresses for the active wallet/network.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_watched_vrpc_addresses(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<Vec<String>, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let account_id = session
        .active_account_id()
        .cloned()
        .ok_or(WalletError::WalletLocked)?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let password_hash = session.stronghold_password_hash_for_storage()?;
    let stronghold_store = session.stronghold_store().clone();
    drop(session);

    let addresses = stronghold_store
        .load_watched_vrpc_addresses(&account_id, password_hash.as_ref(), network)
        .await?;
    Ok(dedupe_preserve_order(
        addresses
            .into_iter()
            .filter_map(|address| normalize_watched_vrpc_address(&address, network))
            .collect(),
    ))
}

/// Persist watched read-only VRPC addresses for the active wallet/network.
#[tauri::command(rename_all = "snake_case")]
pub async fn set_watched_vrpc_addresses(
    addresses: Vec<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<Vec<String>, WalletError> {
    const MAX_WATCHED_ADDRESSES: usize = 100;

    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let account_id = session
        .active_account_id()
        .cloned()
        .ok_or(WalletError::WalletLocked)?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let (primary_vrpc_address, _, _) = session.get_addresses()?;
    let password_hash = session.stronghold_password_hash_for_storage()?;
    let stronghold_store = session.stronghold_store().clone();
    drop(session);

    let mut sanitized = dedupe_preserve_order(
        addresses
            .iter()
            .filter_map(|address| normalize_watched_vrpc_address(address, network))
            .filter(|address| !address.eq_ignore_ascii_case(&primary_vrpc_address))
            .collect(),
    );
    sanitized.truncate(MAX_WATCHED_ADDRESSES);

    stronghold_store
        .store_watched_vrpc_addresses(&account_id, password_hash.as_ref(), network, &sanitized)
        .await?;

    Ok(sanitized)
}

/// Return active asset IDs for the active wallet/network.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_active_assets(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<ActiveAssetsState, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let account_id = session
        .active_account_id()
        .cloned()
        .ok_or(WalletError::WalletLocked)?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let password_hash = session.stronghold_password_hash_for_storage()?;
    let stronghold_store = session.stronghold_store().clone();
    drop(session);

    let (initialized, coin_ids) = stronghold_store
        .load_active_assets(&account_id, password_hash.as_ref(), network)
        .await?;
    let sanitized = sanitize_active_coin_ids(coin_registry.as_ref(), network, &coin_ids);

    Ok(ActiveAssetsState {
        network,
        initialized,
        coin_ids: sanitized,
    })
}

/// Persist active asset IDs for the active wallet/network.
#[tauri::command(rename_all = "snake_case")]
pub async fn set_active_assets(
    coin_ids: Vec<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<ActiveAssetsState, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let account_id = session
        .active_account_id()
        .cloned()
        .ok_or(WalletError::WalletLocked)?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let password_hash = session.stronghold_password_hash_for_storage()?;
    let stronghold_store = session.stronghold_store().clone();
    drop(session);

    let sanitized = sanitize_active_coin_ids(coin_registry.as_ref(), network, &coin_ids);
    stronghold_store
        .store_active_assets(
            &account_id,
            password_hash.as_ref(),
            network,
            true,
            &sanitized,
        )
        .await?;

    Ok(ActiveAssetsState {
        network,
        initialized: true,
        coin_ids: sanitized,
    })
}

/// Get available subwallet scopes for a coin.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_coin_scopes(
    coin_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
) -> Result<CoinScopesResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let account_id = session
        .active_account_id()
        .cloned()
        .ok_or(WalletError::WalletLocked)?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let addresses = session.get_addresses()?;
    let primary_vrpc_address = addresses.0.clone();
    let password_hash = session.stronghold_password_hash_for_storage()?;
    let stronghold_store = session.stronghold_store().clone();
    drop(session);

    let is_testnet = matches!(network, WalletNetwork::Testnet);
    let coin = coin_registry
        .find_by_id(&coin_id, is_testnet)
        .ok_or(WalletError::UnsupportedChannel)?;

    if coin_supports_channel(&coin, Channel::Vrpc) {
        let watched = stronghold_store
            .load_watched_vrpc_addresses(&account_id, password_hash.as_ref(), network)
            .await?;

        let mut read_only_addresses = dedupe_preserve_order(
            watched
                .into_iter()
                .filter_map(|address| normalize_watched_vrpc_address(&address, network))
                .filter(|address| !address.eq_ignore_ascii_case(&primary_vrpc_address))
                .collect(),
        );
        read_only_addresses
            .sort_by(|left, right| left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase()));

        let mut all_addresses = vec![primary_vrpc_address.clone()];
        all_addresses.extend(read_only_addresses);

        let active_coin_ids = stronghold_store
            .load_active_assets(&account_id, password_hash.as_ref(), network)
            .await
            .map(|(_initialized, coin_ids)| coin_ids);
        let systems = match active_coin_ids {
            Ok(active_ids) => {
                collect_vrpc_system_descriptors(coin_registry.as_ref(), network, &coin, &active_ids)
            }
            Err(error) => {
                println!(
                    "[WALLET] Failed to load active assets for coin scopes; using root-only fallback: {:?}",
                    error
                );
                collect_vrpc_system_descriptors(coin_registry.as_ref(), network, &coin, &[])
            }
        };

        let mut scopes = Vec::<CoinScope>::new();
        for address in all_addresses {
            let is_primary_address = address.eq_ignore_ascii_case(&primary_vrpc_address);
            for system in &systems {
                scopes.push(CoinScope {
                    channel_id: crate::core::channels::vrpc::canonical_vrpc_channel_id(
                        &address,
                        &system.system_id,
                    ),
                    coin_id: coin.id.clone(),
                    address: address.clone(),
                    address_label: address.clone(),
                    system_id: system.system_id.clone(),
                    system_ticker: system.system_ticker.clone(),
                    system_display_name: system.system_display_name.clone(),
                    is_primary_address,
                    is_read_only: !is_primary_address,
                });
            }
        }

        return Ok(CoinScopesResult {
            coin_id: coin.id,
            scopes,
        });
    }

    let channel_id = channel_id_for_non_vrpc_coin(&coin).ok_or(WalletError::UnsupportedChannel)?;
    let address = address_for_non_vrpc_coin(&coin, &addresses);
    Ok(CoinScopesResult {
        coin_id: coin.id.clone(),
        scopes: vec![CoinScope {
            channel_id,
            coin_id: coin.id.clone(),
            address: address.clone(),
            address_label: address,
            system_id: coin.system_id.clone(),
            system_ticker: coin.display_ticker.clone(),
            system_display_name: coin.display_name.clone(),
            is_primary_address: true,
            is_read_only: false,
        }],
    })
}

/// Hash account ID for metadata record
fn hash_account_id(account_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(account_id.as_bytes());
    hex::encode(hasher.finalize())
}

/// List available wallets (account_id + wallet_name for unlock flow)
#[tauri::command(rename_all = "snake_case")]
pub async fn list_wallets(
    wallet_manager: State<'_, WalletManager>,
) -> Result<Vec<WalletListItem>, WalletError> {
    println!("[WALLET] List wallets requested");

    let wallets = wallet_manager.list_wallets().await?;

    println!("[WALLET] Found {} wallets", wallets.len());

    Ok(wallets)
}

#[cfg(test)]
mod tests {
    use super::{
        channel_id_for_non_vrpc_coin, collect_vrpc_system_descriptors, dedupe_preserve_order,
        sanitize_active_coin_ids,
    };
    use crate::core::coins::{Channel, CoinDefinition, CoinRegistry, Protocol};
    use crate::types::wallet::WalletNetwork;

    const VRSC_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
    const VETH_SYSTEM_ID: &str = "i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X";
    const CHIPS_SYSTEM_ID: &str = "iJ3WZocnjG9ufv7GKUA4LijQno5gTMb7tP";

    fn sample_vrpc_coin(
        id: &str,
        currency_id: &str,
        system_id: &str,
        ticker: &str,
        name: &str,
    ) -> CoinDefinition {
        CoinDefinition {
            id: id.to_string(),
            currency_id: currency_id.to_string(),
            system_id: system_id.to_string(),
            display_ticker: ticker.to_string(),
            display_name: name.to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec!["https://api.verus.services/".to_string()],
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        }
    }

    #[test]
    fn dedupe_preserve_order_is_case_insensitive() {
        let deduped = dedupe_preserve_order(vec![
            "RAlpha".to_string(),
            "rbeta".to_string(),
            "rBeta".to_string(),
            "RGamma".to_string(),
            "  ".to_string(),
        ]);

        assert_eq!(
            deduped,
            vec![
                "RAlpha".to_string(),
                "rbeta".to_string(),
                "RGamma".to_string()
            ]
        );
    }

    #[test]
    fn vrpc_system_descriptors_put_root_first() {
        let registry = CoinRegistry::new();
        let root_coin = registry.find_by_id("VRSC", false).expect("VRSC root coin");
        let systems =
            collect_vrpc_system_descriptors(&registry, WalletNetwork::Mainnet, &root_coin, &[]);
        assert!(!systems.is_empty());
        assert_eq!(systems[0].system_id, VRSC_SYSTEM_ID.to_string());
        assert!(systems[0].is_root);
    }

    #[test]
    fn vrpc_system_descriptors_include_only_activated_systems() {
        let registry = CoinRegistry::new();
        registry
            .add_coin(sample_vrpc_coin(
                "vUSDC",
                "i6nreNEZpMML7Qw8PWcXh4BB6nffF7tA8Y",
                VETH_SYSTEM_ID,
                "vUSDC.vETH",
                "USDC on Verus",
            ))
            .expect("add vUSDC");
        registry
            .add_coin(sample_vrpc_coin(
                "CHIPS",
                CHIPS_SYSTEM_ID,
                CHIPS_SYSTEM_ID,
                "CHIPS",
                "CHIPS",
            ))
            .expect("add CHIPS");

        let root_coin = registry.find_by_id("VRSC", false).expect("VRSC root coin");
        let root_only =
            collect_vrpc_system_descriptors(&registry, WalletNetwork::Mainnet, &root_coin, &[]);
        assert_eq!(root_only.len(), 1);
        assert_eq!(root_only[0].system_id, VRSC_SYSTEM_ID);

        let with_token_only = collect_vrpc_system_descriptors(
            &registry,
            WalletNetwork::Mainnet,
            &root_coin,
            &["vUSDC".to_string()],
        );
        assert_eq!(with_token_only.len(), 1);
        assert_eq!(with_token_only[0].system_id, VRSC_SYSTEM_ID);

        let with_chips = collect_vrpc_system_descriptors(
            &registry,
            WalletNetwork::Mainnet,
            &root_coin,
            &["vUSDC".to_string(), "CHIPS".to_string()],
        );
        assert_eq!(with_chips.len(), 2);
        assert_eq!(with_chips[0].system_id, VRSC_SYSTEM_ID);
        assert!(with_chips
            .iter()
            .any(|descriptor| descriptor.system_id == CHIPS_SYSTEM_ID));
    }

    #[test]
    fn vrpc_system_descriptors_use_canonical_network_labels() {
        let registry = CoinRegistry::new();
        registry
            .add_coin(sample_vrpc_coin(
                "VETHCHAIN",
                VETH_SYSTEM_ID,
                VETH_SYSTEM_ID,
                "vETH",
                "Ethereum on Verus",
            ))
            .expect("add VETHCHAIN");

        let root_coin = registry.find_by_id("VRSC", false).expect("VRSC root coin");
        let descriptors = collect_vrpc_system_descriptors(
            &registry,
            WalletNetwork::Mainnet,
            &root_coin,
            &["VETHCHAIN".to_string()],
        );

        let veth = descriptors
            .iter()
            .find(|descriptor| descriptor.system_id == VETH_SYSTEM_ID)
            .expect("vETH descriptor");
        assert_eq!(veth.system_ticker, "ETH");
        assert_eq!(veth.system_display_name, "Ethereum");
    }

    #[test]
    fn vrpc_system_descriptors_do_not_use_token_ticker_as_network_label() {
        let registry = CoinRegistry::new();
        registry
            .add_coin(sample_vrpc_coin(
                "vDAI",
                "i7exampleTokenCurrencyForDai",
                VETH_SYSTEM_ID,
                "DAI",
                "Dai Stablecoin",
            ))
            .expect("add vDAI");

        let root_coin = registry.find_by_id("VRSC", false).expect("VRSC root coin");
        let descriptors = collect_vrpc_system_descriptors(
            &registry,
            WalletNetwork::Mainnet,
            &root_coin,
            &["vDAI".to_string()],
        );

        assert_eq!(descriptors.len(), 1);
        assert_eq!(descriptors[0].system_id, VRSC_SYSTEM_ID);
        assert!(!descriptors
            .iter()
            .any(|descriptor| descriptor.system_id == VETH_SYSTEM_ID));
    }

    #[test]
    fn vrpc_system_descriptors_always_include_root_system() {
        let registry = CoinRegistry::new();
        registry
            .add_coin(sample_vrpc_coin(
                "CHIPS",
                CHIPS_SYSTEM_ID,
                CHIPS_SYSTEM_ID,
                "CHIPS",
                "CHIPS",
            ))
            .expect("add CHIPS");

        let root_coin = registry.find_by_id("VRSC", false).expect("VRSC root coin");
        let descriptors = collect_vrpc_system_descriptors(
            &registry,
            WalletNetwork::Mainnet,
            &root_coin,
            &["CHIPS".to_string()],
        );
        assert!(descriptors
            .iter()
            .any(|descriptor| descriptor.system_id == VRSC_SYSTEM_ID));
        assert_eq!(descriptors[0].system_id, VRSC_SYSTEM_ID);
    }

    #[test]
    fn sanitize_active_coin_ids_dedupes_and_drops_unknown() {
        let registry = CoinRegistry::new();
        let sanitized = sanitize_active_coin_ids(
            &registry,
            WalletNetwork::Mainnet,
            &[
                "vrsc".to_string(),
                "VRSC".to_string(),
                "UNKNOWN".to_string(),
                " ".to_string(),
            ],
        );
        assert_eq!(sanitized, vec!["VRSC".to_string()]);
    }

    #[test]
    fn channel_id_for_non_vrpc_coin_respects_channel_priority() {
        let btc_coin = CoinDefinition {
            id: "BTC".to_string(),
            currency_id: "BTC".to_string(),
            system_id: "BTC".to_string(),
            display_ticker: "BTC".to_string(),
            display_name: "Bitcoin".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Btc,
            compatible_channels: vec![Channel::Btc, Channel::Eth],
            decimals: 8,
            vrpc_endpoints: vec![],
            electrum_endpoints: None,
            seconds_per_block: 600,
            mapped_to: None,
            is_testnet: false,
        };

        let channel_id = channel_id_for_non_vrpc_coin(&btc_coin).expect("channel");
        assert_eq!(channel_id, "btc.BTC");
    }
}
