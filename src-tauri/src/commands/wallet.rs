// 
// Tauri command handlers for wallet operations
// Security: Thin wrappers that validate inputs and delegate to core logic
// Last Updated: Added password minimum length validation (7 characters) matching Verus-Mobile

use tauri::{State, AppHandle};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use sha2::{Sha256, Digest};
use crate::core::wallet::WalletManager;
use crate::core::auth::SessionManager;
use crate::types::{WalletError, CreateWalletRequest, CreateWalletResult, GenerateMnemonicRequest, MnemonicResult, AccountRecord, AddressResponse};

/// Generate a new BIP39 mnemonic phrase
#[tauri::command]
pub async fn generate_mnemonic(
    request: GenerateMnemonicRequest,
    wallet_manager: State<'_, WalletManager>,
) -> Result<MnemonicResult, WalletError> {
    // Input validation
    if request.word_count != 24 {
        return Err(WalletError::InvalidSeedPhrase);
    }
    
    println!("[WALLET] Generate mnemonic requested: {} words", request.word_count);
    
    // Delegate to core logic
    let seed_phrase = wallet_manager.generate_mnemonic(request.word_count).await?;
    
    println!("[WALLET] Mnemonic generation completed");
    
    Ok(MnemonicResult { seed_phrase })
}

/// Validate a BIP39 mnemonic phrase
#[tauri::command]
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

/// Create a new wallet with Stronghold encryption
#[tauri::command]
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
    
    // Generate account ID
    let account_id = Uuid::new_v4().to_string();
    
    // Store seed in Stronghold
    let session = session_manager.lock().await;
    session.stronghold_store().store_seed(&account_id, &request.seed_phrase, &password, &app_handle).await?;
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

/// Unlock wallet with password
#[tauri::command]
pub async fn unlock_wallet(
    account_id: String,
    password: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    app_handle: AppHandle,
) -> Result<(), WalletError> {
    println!("[WALLET] Unlock wallet requested");
    
    let mut session = session_manager.lock().await;
    session.unlock(account_id, password, &app_handle).await?;
    
    println!("[WALLET] Wallet unlocked successfully");
    Ok(())
}

/// Lock wallet and zeroize keys
#[tauri::command]
pub async fn lock_wallet(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<(), WalletError> {
    println!("[WALLET] Lock wallet requested");
    
    let mut session = session_manager.lock().await;
    session.lock();
    
    println!("[WALLET] Wallet locked successfully");
    Ok(())
}

/// Get derived addresses for active account
#[tauri::command]
pub async fn get_addresses(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<AddressResponse, WalletError> {
    println!("[WALLET] Get addresses requested");
    
    let session = session_manager.lock().await;
    let (vrsc_address, eth_address) = session.get_addresses()?;
    
    println!("[WALLET] Addresses retrieved");
    Ok(AddressResponse {
        vrsc_address,
        eth_address,
    })
}

/// Check if wallet is unlocked
#[tauri::command]
pub async fn is_unlocked(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<bool, WalletError> {
    let session = session_manager.lock().await;
    Ok(session.is_unlocked())
}

/// Hash account ID for metadata record
fn hash_account_id(account_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(account_id.as_bytes());
    hex::encode(hasher.finalize())
}

/// List available wallets
#[tauri::command]
pub async fn list_wallets(
    wallet_manager: State<'_, WalletManager>,
) -> Result<Vec<String>, WalletError> {
    println!("[WALLET] List wallets requested");
    
    let wallets = wallet_manager.list_wallets().await?;
    
    println!("[WALLET] Found {} wallets", wallets.len());
    
    Ok(wallets)
}
