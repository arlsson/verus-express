//
// Module 3: Tauri commands for coin registry (get_coin_registry, add_pbaas_currency).
// Thin wrappers; validation and allowlist policy live in core/coins.
// Last Updated: State<Arc<CoinRegistry>> for Module 7 update engine sharing.

use std::sync::Arc;
use tauri::State;

use crate::core::{CoinDefinition, CoinRegistry};
use crate::types::WalletError;

/// Returns all coins: static definitions plus dynamically added PBaaS currencies.
#[tauri::command(rename_all = "snake_case")]
pub fn get_coin_registry(
    registry: State<'_, Arc<CoinRegistry>>,
) -> Result<Vec<CoinDefinition>, WalletError> {
    println!("[COINS] Get coin registry requested");
    let coins = registry.get_all();
    println!("[COINS] Returning {} coins", coins.len());
    Ok(coins)
}

/// Adds a PBaaS currency to the registry. Validates definition; rejects duplicates.
#[tauri::command(rename_all = "snake_case")]
pub fn add_pbaas_currency(
    registry: State<'_, Arc<CoinRegistry>>,
    definition: CoinDefinition,
) -> Result<(), WalletError> {
    println!("[COINS] Add PBaaS currency requested: {}", definition.id);
    registry.add_pbaas(definition)?;
    println!("[COINS] PBaaS currency added");
    Ok(())
}
