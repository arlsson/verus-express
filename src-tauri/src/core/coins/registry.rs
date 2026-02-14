//
// Module 3: Coin Registry — static coin definitions and dynamic PBaaS additions.
// VRPC/Electrum endpoints are allowlist-only; custom endpoints deferred to advanced settings.

use std::sync::Mutex;

use crate::core::coins::types::{Channel, CoinDefinition, Protocol};
use crate::types::WalletError;

/// Trusted VRPC allowlist (per backend architecture plan).
const VRPC_MAINNET: &str = "https://api.verus.services/";
const VRPC_TESTNET: &str = "https://api.verustest.net/";

/// Default Electrum allowlist for BTC (well-known public servers).
const ELECTRUM_MAINNET: &[&str] = &["https://electrum.blockstream.info"];
const ELECTRUM_TESTNET: &[&str] = &["https://electrum.blockstream.info/testnet"];

/// Verus mainnet system ID (i-address format).
const VRSC_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
const ETH_ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

/// Registry of coins: static defaults plus dynamically added PBaaS currencies.
pub struct CoinRegistry {
    dynamic_coins: Mutex<Vec<CoinDefinition>>,
}

impl CoinRegistry {
    pub fn new() -> Self {
        Self {
            dynamic_coins: Mutex::new(Vec::new()),
        }
    }

    /// Returns all coins: static list first, then dynamic PBaaS entries.
    pub fn get_all(&self) -> Vec<CoinDefinition> {
        let static_coins = Self::default_coins();
        let dynamic = self.dynamic_coins.lock().expect("coin registry lock");
        static_coins
            .into_iter()
            .chain(dynamic.iter().cloned())
            .collect()
    }

    /// Find a coin by system ID and network.
    pub fn find_by_system_id(&self, system_id: &str, is_testnet: bool) -> Option<CoinDefinition> {
        self.get_all()
            .into_iter()
            .find(|c| c.system_id == system_id && c.is_testnet == is_testnet)
    }

    /// Find a coin by ID and network.
    pub fn find_by_id(&self, id: &str, is_testnet: bool) -> Option<CoinDefinition> {
        self.get_all()
            .into_iter()
            .find(|c| c.id.eq_ignore_ascii_case(id) && c.is_testnet == is_testnet)
    }

    /// Adds a PBaaS currency. Validates proto (Vrsc) and required fields.
    /// Returns error if id or currency_id already present (no duplicates).
    pub fn add_pbaas(&self, def: CoinDefinition) -> Result<(), WalletError> {
        if def.proto != Protocol::Vrsc {
            return Err(WalletError::InvalidCoinDefinition);
        }
        if def.id.is_empty() || def.currency_id.is_empty() || def.system_id.is_empty() {
            return Err(WalletError::InvalidCoinDefinition);
        }
        let mut dynamic = self
            .dynamic_coins
            .lock()
            .map_err(|_| WalletError::OperationFailed)?;
        let exists = dynamic
            .iter()
            .any(|c| c.id == def.id || c.currency_id == def.currency_id);
        if exists {
            return Err(WalletError::DuplicatePbaasCurrency);
        }
        dynamic.push(def);
        Ok(())
    }

    /// Static coin definitions. VRPC endpoints are allowlist-only.
    fn default_coins() -> Vec<CoinDefinition> {
        vec![
            // VRSC mainnet
            CoinDefinition {
                id: "VRSC".to_string(),
                currency_id: VRSC_SYSTEM_ID.to_string(),
                system_id: VRSC_SYSTEM_ID.to_string(),
                display_ticker: "VRSC".to_string(),
                display_name: "Verus".to_string(),
                proto: Protocol::Vrsc,
                compatible_channels: vec![Channel::Vrpc],
                decimals: 8,
                vrpc_endpoints: vec![VRPC_MAINNET.to_string()],
                electrum_endpoints: None,
                seconds_per_block: 60,
                mapped_to: None,
                is_testnet: false,
            },
            // VRSCTEST
            CoinDefinition {
                id: "VRSCTEST".to_string(),
                currency_id: "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq".to_string(),
                system_id: "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq".to_string(),
                display_ticker: "VRSCTEST".to_string(),
                display_name: "Verus Testnet".to_string(),
                proto: Protocol::Vrsc,
                compatible_channels: vec![Channel::Vrpc],
                decimals: 8,
                vrpc_endpoints: vec![VRPC_TESTNET.to_string()],
                electrum_endpoints: None,
                seconds_per_block: 60,
                mapped_to: None,
                is_testnet: true,
            },
            // BTC mainnet
            CoinDefinition {
                id: "BTC".to_string(),
                currency_id: String::new(),
                system_id: "BTC".to_string(),
                display_ticker: "BTC".to_string(),
                display_name: "Bitcoin".to_string(),
                proto: Protocol::Btc,
                compatible_channels: vec![Channel::Btc],
                decimals: 8,
                vrpc_endpoints: vec![],
                electrum_endpoints: Some(
                    ELECTRUM_MAINNET.iter().map(|s| (*s).to_string()).collect(),
                ),
                seconds_per_block: 600,
                mapped_to: None,
                is_testnet: false,
            },
            // BTCTEST
            CoinDefinition {
                id: "BTCTEST".to_string(),
                currency_id: String::new(),
                system_id: "BTCTEST".to_string(),
                display_ticker: "BTCTEST".to_string(),
                display_name: "Bitcoin Testnet".to_string(),
                proto: Protocol::Btc,
                compatible_channels: vec![Channel::Btc],
                decimals: 8,
                vrpc_endpoints: vec![],
                electrum_endpoints: Some(
                    ELECTRUM_TESTNET.iter().map(|s| (*s).to_string()).collect(),
                ),
                seconds_per_block: 600,
                mapped_to: None,
                is_testnet: true,
            },
            // ETH mainnet
            CoinDefinition {
                id: "ETH".to_string(),
                currency_id: ETH_ZERO_ADDRESS.to_string(),
                system_id: "ETH".to_string(),
                display_ticker: "ETH".to_string(),
                display_name: "Ethereum".to_string(),
                proto: Protocol::Eth,
                compatible_channels: vec![Channel::Eth],
                decimals: 18,
                vrpc_endpoints: vec![],
                electrum_endpoints: None,
                seconds_per_block: 12,
                mapped_to: None,
                is_testnet: false,
            },
            // GETH testnet
            CoinDefinition {
                id: "GETH".to_string(),
                currency_id: ETH_ZERO_ADDRESS.to_string(),
                system_id: "GETH".to_string(),
                display_ticker: "GETH".to_string(),
                display_name: "Ethereum Testnet".to_string(),
                proto: Protocol::Eth,
                compatible_channels: vec![Channel::Eth],
                decimals: 18,
                vrpc_endpoints: vec![],
                electrum_endpoints: None,
                seconds_per_block: 12,
                mapped_to: None,
                is_testnet: true,
            },
            // ERC20 example: USDC on Ethereum
            CoinDefinition {
                id: "USDC".to_string(),
                currency_id: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
                system_id: "ETH".to_string(),
                display_ticker: "USDC".to_string(),
                display_name: "USD Coin".to_string(),
                proto: Protocol::Erc20,
                compatible_channels: vec![Channel::Erc20],
                decimals: 6,
                vrpc_endpoints: vec![],
                electrum_endpoints: None,
                seconds_per_block: 12,
                mapped_to: Some("ETH".to_string()),
                is_testnet: false,
            },
        ]
    }
}

impl Default for CoinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vrsc_and_vrsctest_system_ids_match_reference() {
        let registry = CoinRegistry::new();
        let all = registry.get_all();

        let vrsc = all.iter().find(|c| c.id == "VRSC").expect("VRSC coin");
        let vrsctest = all
            .iter()
            .find(|c| c.id == "VRSCTEST")
            .expect("VRSCTEST coin");

        assert_eq!(
            vrsc.system_id, "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV",
            "VRSC mainnet system id must match mobile reference"
        );
        assert_eq!(
            vrsctest.system_id, "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
            "VRSCTEST system id must stay stable"
        );
    }
}
