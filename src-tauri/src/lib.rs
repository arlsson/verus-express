// 
// Main Tauri application setup and command registration
// Security: Manages application state and exposes secure wallet command interface
// Last Updated: Updated for wallet creation flow implementation

mod commands;
mod core;
mod types;

use commands::wallet;
use core::WalletManager;
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_stronghold::Builder::new(|password| {
            // Password hash function for Stronghold
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(password);
            hasher.finalize().to_vec()
        }).build())
        .setup(|app| {
            // Initialize wallet manager
            let wallet_manager = WalletManager::new();
            app.manage(wallet_manager);
            
            println!("[APP] Wallet manager initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            
            // Wallet creation commands
            wallet::generate_mnemonic,
            wallet::validate_mnemonic,
            wallet::create_wallet,
            wallet::list_wallets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
