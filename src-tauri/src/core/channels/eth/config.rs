use std::env;

use crate::types::WalletError;

const DEFAULT_MAINNET_INFURA_URL: &str = "https://mainnet.infura.io/v3/{project_id}";
const DEFAULT_TESTNET_INFURA_URL: &str = "https://goerli.infura.io/v3/{project_id}";
const DEFAULT_ETHERSCAN_MAINNET_URL: &str = "https://api.etherscan.io/api";
const DEFAULT_ETHERSCAN_TESTNET_URL: &str = "https://api-goerli.etherscan.io/api";

#[derive(Debug, Clone)]
pub struct EthChannelConfig {
    pub etherscan_api_key: String,
    pub mainnet_rpc_url: String,
    pub testnet_rpc_url: String,
    pub etherscan_mainnet_url: String,
    pub etherscan_testnet_url: String,
}

impl EthChannelConfig {
    pub fn from_env() -> Result<Self, WalletError> {
        let infura_project_id = read_required_env("INFURA_PROJECT_ID")?;
        let etherscan_api_key = read_required_env("ETHERSCAN_API_KEY")?;

        let mainnet_rpc_url = read_optional_env("ETH_MAINNET_RPC_URL").unwrap_or_else(|| {
            DEFAULT_MAINNET_INFURA_URL.replace("{project_id}", &infura_project_id)
        });
        let testnet_rpc_url = read_optional_env("ETH_TESTNET_RPC_URL").unwrap_or_else(|| {
            DEFAULT_TESTNET_INFURA_URL.replace("{project_id}", &infura_project_id)
        });

        let etherscan_mainnet_url = read_optional_env("ETHERSCAN_MAINNET_URL")
            .unwrap_or_else(|| DEFAULT_ETHERSCAN_MAINNET_URL.to_string());
        let etherscan_testnet_url = read_optional_env("ETHERSCAN_TESTNET_URL")
            .unwrap_or_else(|| DEFAULT_ETHERSCAN_TESTNET_URL.to_string());

        Ok(Self {
            etherscan_api_key,
            mainnet_rpc_url,
            testnet_rpc_url,
            etherscan_mainnet_url,
            etherscan_testnet_url,
        })
    }
}

fn read_optional_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn read_required_env(key: &str) -> Result<String, WalletError> {
    read_optional_env(key).ok_or(WalletError::EthNotConfigured)
}
