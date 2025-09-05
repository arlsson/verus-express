// 
// Tauri command handlers for wallet operations
// Security: Thin wrappers that validate inputs and delegate to core logic
// Last Updated: Created for wallet creation flow implementation

use tauri::State;
use crate::core::wallet::WalletManager;
use crate::types::{WalletError, CreateWalletRequest, CreateWalletResult, GenerateMnemonicRequest, MnemonicResult};

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
) -> Result<CreateWalletResult, WalletError> {
    // Validate inputs
    request.validate()?;
    
    if password.trim().is_empty() {
        return Err(WalletError::InvalidPassword);
    }
    
    println!("[WALLET] Create wallet requested: {}", request.wallet_name);
    
    // Delegate to core logic
    let wallet_id = wallet_manager.create_wallet(&request, &password).await?;
    
    println!("[WALLET] Wallet created successfully: {}", wallet_id);
    
    Ok(CreateWalletResult {
        wallet_id,
        success: true,
    })
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
