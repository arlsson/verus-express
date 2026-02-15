//
// Tauri command handlers for wallet operations
// Security: Thin wrappers that validate inputs and delegate to core logic
// Last Updated: Module 10 — unlock/session and update-engine start are decoupled

use secp256k1::SecretKey;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::core::auth::SessionManager;
use crate::core::channels::btc::BtcProviderPool;
use crate::core::channels::eth::EthProviderPool;
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::coins::CoinRegistry;
use crate::core::crypto::wif_encoding::decode_wif_unchecked_network;
use crate::core::updates::UpdateEngineStartConfig;
use crate::core::wallet::WalletManager;
use crate::core::{GuardSessionManager, PreflightStore, UpdateEngine};
use crate::types::{
    AccountRecord, ActiveWalletResponse, AddressResponse, CreateWalletRequest, CreateWalletResult,
    GenerateMnemonicRequest, ImportWalletTextRequest, MnemonicResult, WalletError, WalletListItem,
    WalletSecretKind,
};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct StartUpdateEngineRequest {
    pub include_transactions: Option<bool>,
    pub priority_coin_ids: Option<Vec<String>>,
    pub priority_channel_ids: Option<Vec<String>>,
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
