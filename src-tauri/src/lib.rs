// 
// Main Tauri application setup and command registration
// Security: Manages application state and exposes secure wallet command interface
// Last Updated: Wired up SessionManager and auth/crypto modules for Module 1 & 2 integration

mod commands;
mod core;
mod types;

use commands::wallet;
use core::{WalletManager, SessionManager, StrongholdStore};
use tauri::Manager;
use std::sync::Arc;
use tokio::sync::Mutex;

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
            
            // Initialize Stronghold store and session manager
            let app_handle = app.handle();
            let stronghold_store = StrongholdStore::new(app_handle)
                .map_err(|e| {
                    eprintln!("[APP] Failed to initialize Stronghold store: {:?}", e);
                    e
                })?;
            let session_manager = Arc::new(Mutex::new(SessionManager::new(stronghold_store)));
            app.manage(session_manager);
            println!("[APP] Session manager initialized");
            
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
            
            // Session management commands
            wallet::unlock_wallet,
            wallet::lock_wallet,
            wallet::get_addresses,
            wallet::is_unlocked,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
