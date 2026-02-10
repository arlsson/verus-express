//
// Module 3: Coin Registry — Protocol, Channel, and CoinDefinition types.
// Used for static and dynamic coin definitions; endpoints are allowlist-only.

use serde::{Deserialize, Serialize};

/// Blockchain protocol for a coin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Vrsc,
    Btc,
    Eth,
    Erc20,
}

/// Channel used to interact with a coin (maps to Verus-Mobile channel IDs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Vrpc,
    Btc,
    Eth,
    Erc20,
}

/// Definition of a supported coin. All fields are safe for IPC/frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoinDefinition {
    pub id: String,
    pub currency_id: String,
    pub system_id: String,
    pub display_ticker: String,
    pub display_name: String,
    pub proto: Protocol,
    pub compatible_channels: Vec<Channel>,
    pub decimals: u8,
    pub vrpc_endpoints: Vec<String>,
    pub electrum_endpoints: Option<Vec<String>>,
    pub seconds_per_block: u64,
    pub mapped_to: Option<String>,
    pub is_testnet: bool,
}
