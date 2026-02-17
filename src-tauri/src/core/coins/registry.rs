//
// Module 3: Coin Registry — static coin definitions and dynamic PBaaS additions.
// VRPC/Electrum endpoints are allowlist-only; custom endpoints deferred to advanced settings.

use std::fs;
use std::path::{Path, PathBuf};
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
    dynamic_store_path: Option<PathBuf>,
}

impl CoinRegistry {
    pub fn new() -> Self {
        Self {
            dynamic_coins: Mutex::new(Vec::new()),
            dynamic_store_path: None,
        }
    }

    pub fn with_dynamic_store(path: PathBuf) -> Self {
        let dynamic_coins = Self::load_dynamic_coins(&path);
        Self {
            dynamic_coins: Mutex::new(dynamic_coins),
            dynamic_store_path: Some(path),
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

    fn load_dynamic_coins(path: &Path) -> Vec<CoinDefinition> {
        let Ok(raw) = fs::read_to_string(path) else {
            return Vec::new();
        };
        serde_json::from_str::<Vec<CoinDefinition>>(&raw).unwrap_or_default()
    }

    fn persist_dynamic_coins(&self, coins: &[CoinDefinition]) -> Result<(), WalletError> {
        let Some(path) = self.dynamic_store_path.as_ref() else {
            return Ok(());
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| WalletError::OperationFailed)?;
        }

        let payload =
            serde_json::to_string_pretty(coins).map_err(|_| WalletError::OperationFailed)?;
        fs::write(path, payload).map_err(|_| WalletError::OperationFailed)?;
        Ok(())
    }

    fn validate_coin_definition(def: &CoinDefinition) -> Result<(), WalletError> {
        if def.id.trim().is_empty() || def.system_id.trim().is_empty() {
            return Err(WalletError::InvalidCoinDefinition);
        }
        if def.compatible_channels.is_empty() {
            return Err(WalletError::InvalidCoinDefinition);
        }
        Ok(())
    }

    fn canonical_asset_key(def: &CoinDefinition) -> String {
        let network = if def.is_testnet { "testnet" } else { "mainnet" };
        match def.proto {
            Protocol::Vrsc => format!(
                "vrpc:{}:{}:{}",
                def.system_id.to_ascii_lowercase(),
                def.currency_id.to_ascii_lowercase(),
                network
            ),
            Protocol::Btc => format!("btc:{}:{}", def.id.to_ascii_lowercase(), network),
            Protocol::Eth => format!("eth:{}:{}", def.id.to_ascii_lowercase(), network),
            Protocol::Erc20 => {
                format!("erc20:{}:{}", def.currency_id.to_ascii_lowercase(), network)
            }
        }
    }

    fn has_duplicate(existing: &CoinDefinition, candidate: &CoinDefinition) -> bool {
        if existing.is_testnet != candidate.is_testnet {
            return false;
        }

        if existing.id.eq_ignore_ascii_case(&candidate.id) {
            return true;
        }

        if !existing.currency_id.is_empty()
            && !candidate.currency_id.is_empty()
            && existing
                .currency_id
                .eq_ignore_ascii_case(&candidate.currency_id)
            && existing.proto == candidate.proto
        {
            return true;
        }

        Self::canonical_asset_key(existing) == Self::canonical_asset_key(candidate)
    }

    /// Adds a coin definition to the dynamic registry and persists it when storage is configured.
    /// Rejects duplicates by canonical asset key (network + protocol identity).
    pub fn add_coin(&self, def: CoinDefinition) -> Result<CoinDefinition, WalletError> {
        Self::validate_coin_definition(&def)?;

        let static_coins = Self::default_coins();
        let mut dynamic = self
            .dynamic_coins
            .lock()
            .map_err(|_| WalletError::OperationFailed)?;

        let duplicate_static = static_coins.iter().any(|c| Self::has_duplicate(c, &def));
        if duplicate_static {
            return Err(WalletError::AssetAlreadyExists);
        }

        if let Some(existing_index) = dynamic.iter().position(|c| Self::has_duplicate(c, &def)) {
            if dynamic[existing_index] == def {
                return Err(WalletError::AssetAlreadyExists);
            }

            let previous = dynamic[existing_index].clone();
            dynamic[existing_index] = def.clone();
            if let Err(err) = self.persist_dynamic_coins(&dynamic) {
                dynamic[existing_index] = previous;
                return Err(err);
            }
            return Ok(def);
        }

        dynamic.push(def.clone());
        if let Err(err) = self.persist_dynamic_coins(&dynamic) {
            dynamic.pop();
            return Err(err);
        }
        Ok(def)
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
        match self.add_coin(def) {
            Ok(_) => Ok(()),
            Err(WalletError::AssetAlreadyExists) => Err(WalletError::DuplicatePbaasCurrency),
            Err(err) => Err(err),
        }
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
                coin_paprika_id: Some("vrsc-verus-coin".to_string()),
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
                coin_paprika_id: None,
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
                coin_paprika_id: Some("btc-bitcoin".to_string()),
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
                coin_paprika_id: None,
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
                coin_paprika_id: Some("eth-ethereum".to_string()),
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
                coin_paprika_id: None,
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
                coin_paprika_id: Some("usdc-usd-coin".to_string()),
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample_dynamic_coin() -> CoinDefinition {
        CoinDefinition {
            id: "iSampleCurrency".to_string(),
            currency_id: "iSampleCurrency".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "SAMPLE".to_string(),
            display_name: "Sample currency".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![VRPC_MAINNET.to_string()],
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        }
    }

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

    #[test]
    fn add_coin_rejects_duplicate_static_asset() {
        let registry = CoinRegistry::new();
        let duplicate = CoinDefinition {
            id: "USDC_DUP".to_string(),
            currency_id: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
            system_id: "ETH".to_string(),
            display_ticker: "USDC".to_string(),
            display_name: "USD Coin duplicate".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Erc20,
            compatible_channels: vec![Channel::Erc20],
            decimals: 6,
            vrpc_endpoints: vec![],
            electrum_endpoints: None,
            seconds_per_block: 12,
            mapped_to: Some("ETH".to_string()),
            is_testnet: false,
        };

        let result = registry.add_coin(duplicate);
        assert!(matches!(result, Err(WalletError::AssetAlreadyExists)));
    }

    #[test]
    fn add_coin_updates_existing_dynamic_duplicate_with_new_metadata() {
        let registry = CoinRegistry::new();
        let stale = CoinDefinition {
            id: "i3d4vSCbXYEC3u6TzwohMvdghHkhBrXWpE".to_string(),
            currency_id: "i3d4vSCbXYEC3u6TzwohMvdghHkhBrXWpE".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "VRSC".to_string(),
            display_name: "Verus".to_string(),
            coin_paprika_id: Some("vrsc-verus-coin".to_string()),
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![VRPC_MAINNET.to_string()],
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        };
        registry.add_coin(stale).expect("add stale dynamic coin");

        let corrected = CoinDefinition {
            id: "i3d4vSCbXYEC3u6TzwohMvdghHkhBrXWpE".to_string(),
            currency_id: "i3d4vSCbXYEC3u6TzwohMvdghHkhBrXWpE".to_string(),
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            display_ticker: "Floralis".to_string(),
            display_name: "Floralis".to_string(),
            coin_paprika_id: None,
            proto: Protocol::Vrsc,
            compatible_channels: vec![Channel::Vrpc],
            decimals: 8,
            vrpc_endpoints: vec![VRPC_MAINNET.to_string()],
            electrum_endpoints: None,
            seconds_per_block: 60,
            mapped_to: None,
            is_testnet: false,
        };
        let updated = registry
            .add_coin(corrected.clone())
            .expect("update stale duplicate coin");

        assert_eq!(updated.display_ticker, "Floralis");
        assert_eq!(updated.display_name, "Floralis");
        assert_eq!(updated.coin_paprika_id, None);

        let found = registry
            .find_by_id("i3d4vSCbXYEC3u6TzwohMvdghHkhBrXWpE", false)
            .expect("updated dynamic coin");
        assert_eq!(found.display_ticker, "Floralis");
        assert_eq!(found.display_name, "Floralis");
        assert_eq!(found, corrected);
    }

    #[test]
    fn add_coin_rejects_identical_dynamic_duplicate() {
        let registry = CoinRegistry::new();
        let sample = sample_dynamic_coin();
        registry
            .add_coin(sample.clone())
            .expect("add first dynamic coin");

        let result = registry.add_coin(sample);
        assert!(matches!(result, Err(WalletError::AssetAlreadyExists)));
    }

    #[test]
    fn add_coin_persists_dynamic_entries_when_store_is_configured() {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lite_wallet_coin_registry_{}.json", unique_suffix));

        let registry = CoinRegistry::with_dynamic_store(path.clone());
        let added = registry
            .add_coin(sample_dynamic_coin())
            .expect("add dynamic coin");
        assert_eq!(added.id, "iSampleCurrency");

        let reloaded = CoinRegistry::with_dynamic_store(path.clone());
        let found = reloaded
            .find_by_id("iSampleCurrency", false)
            .expect("reloaded coin");
        assert_eq!(found.currency_id, "iSampleCurrency");

        let _ = std::fs::remove_file(path);
    }
}
