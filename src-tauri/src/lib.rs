//
// Main Tauri application setup and command registration
// Security: Manages application state and exposes secure wallet command interface
// Last Updated: Network-aware provider pools for mainnet/testnet routing

mod commands;
mod core;
mod types;

use commands::{
    address_book, bridge_transfer, clipboard, coins, guard, identity, transaction, vrpc_transfer,
    wallet,
};
use core::channels::btc::BtcProviderPool;
use core::channels::eth::EthProviderPool;
use core::channels::vrpc::VrpcProviderPool;
use core::{
    CoinRegistry, GuardSessionManager, PreflightStore, SessionManager, StrongholdStore,
    UpdateEngine, WalletManager,
};
use std::path::Path;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn load_runtime_env_files() {
    // Load local env files for desktop dev/runtime convenience without committing secrets.
    // Existing process environment variables keep precedence.
    for candidate in [".env.local", ".env", "../.env.local", "../.env"] {
        if !Path::new(candidate).exists() {
            continue;
        }

        match dotenvy::from_filename(candidate) {
            Ok(_) => println!("[APP] Loaded runtime env file: {}", candidate),
            Err(err) => eprintln!(
                "[APP] Failed to load runtime env file {}: {}",
                candidate, err
            ),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    load_runtime_env_files();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_stronghold::Builder::new(|password| {
                // Password hash function for Stronghold
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(password);
                hasher.finalize().to_vec()
            })
            .build(),
        )
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                // Speed up snapshot encrypt/decrypt during local development.
                // Do not use reduced work factors in production builds.
                match iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0) {
                    Ok(()) => println!("[APP] Stronghold debug work factor set to 0"),
                    Err(e) => {
                        eprintln!("[APP] Failed to set Stronghold debug work factor: {:?}", e)
                    }
                }
            }

            // Initialize wallet manager with app data dir (unified with Stronghold storage)
            let app_dir = app.path().app_data_dir().map_err(|e| {
                eprintln!("[APP] Failed to get app data directory: {:?}", e);
                e
            })?;
            let wallet_data_dir = app_dir.join("wallet_data");
            std::fs::create_dir_all(&wallet_data_dir).map_err(|e| {
                eprintln!("[APP] Failed to create wallet data directory: {:?}", e);
                e
            })?;
            let wallet_manager = WalletManager::new(wallet_data_dir.clone());
            app.manage(wallet_manager);

            // Initialize Stronghold store and session manager
            let app_handle = app.handle();
            let stronghold_store = StrongholdStore::new(app_handle).map_err(|e| {
                eprintln!("[APP] Failed to initialize Stronghold store: {:?}", e);
                e
            })?;
            let session_manager = Arc::new(Mutex::new(SessionManager::new(stronghold_store)));
            app.manage(session_manager);
            println!("[APP] Session manager initialized");

            let coin_registry_store = wallet_data_dir.join("dynamic_coins.json");
            let coin_registry = Arc::new(CoinRegistry::with_dynamic_store(coin_registry_store));
            app.manage(coin_registry);
            println!("[APP] Coin registry initialized");

            let preflight_store = PreflightStore::new();
            app.manage(preflight_store);
            println!("[APP] Preflight store initialized");

            let guard_session_manager = Arc::new(Mutex::new(GuardSessionManager::new()));
            app.manage(guard_session_manager);
            println!("[APP] Guard session manager initialized");

            let vrpc_provider_pool = Arc::new(VrpcProviderPool::new());
            app.manage(vrpc_provider_pool);
            println!("[APP] VRPC providers initialized");

            let btc_provider_pool = Arc::new(BtcProviderPool::new());
            app.manage(btc_provider_pool);
            println!("[APP] BTC providers initialized");

            let eth_provider_pool = Arc::new(EthProviderPool::new());
            if let Some(reason) = eth_provider_pool.disabled_reason() {
                println!("[APP] ETH providers disabled: {}", reason);
            } else {
                println!("[APP] ETH providers initialized");
            }
            app.manage(eth_provider_pool);

            let update_engine = Arc::new(UpdateEngine::new());
            app.manage(update_engine);
            println!("[APP] Update engine initialized");

            println!("[APP] Wallet manager initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            // Wallet creation commands
            wallet::generate_mnemonic,
            wallet::validate_mnemonic,
            wallet::get_mnemonic_wordlist,
            wallet::create_wallet,
            wallet::import_wallet_text,
            wallet::list_wallets,
            wallet::get_active_wallet,
            // Session management commands
            wallet::unlock_wallet,
            wallet::start_update_engine,
            wallet::lock_wallet,
            wallet::get_addresses,
            wallet::is_unlocked,
            clipboard::read_clipboard_text,
            // Coin registry commands (Module 3)
            coins::get_coin_registry,
            coins::add_coin_definition,
            coins::add_pbaas_currency,
            coins::resolve_pbaas_currency,
            coins::resolve_erc20_contract,
            // Transaction commands (Module 4 + 9)
            transaction::preflight_send,
            transaction::send_transaction,
            transaction::get_balances,
            transaction::get_transaction_history,
            vrpc_transfer::preflight_vrpc_transfer,
            bridge_transfer::get_bridge_capabilities,
            bridge_transfer::get_bridge_conversion_paths,
            bridge_transfer::estimate_bridge_conversion,
            bridge_transfer::preflight_bridge_transfer,
            // Identity commands
            identity::preflight_identity_update,
            identity::send_identity_update,
            guard::begin_guard_session,
            guard::end_guard_session,
            guard::lookup_guard_target_identity,
            guard::preflight_guard_identity_update,
            guard::send_guard_identity_update,
            // Address book commands
            address_book::list_address_book_contacts,
            address_book::save_address_book_contact,
            address_book::delete_address_book_contact,
            address_book::mark_address_book_endpoint_used,
            address_book::validate_destination_address,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
