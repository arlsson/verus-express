use std::time::Duration;

use ethers::providers::{Http, Provider};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::core::channels::eth::config::EthChannelConfig;
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

#[derive(Debug, Clone)]
pub struct EthNetworkProvider {
    pub chain_id: u64,
    pub rpc_provider: Provider<Http>,
    pub history_provider: EtherscanHistoryClient,
}

pub struct EthProviderPool {
    mainnet: Option<EthNetworkProvider>,
    testnet: Option<EthNetworkProvider>,
    disabled_reason: Option<String>,
}

impl EthProviderPool {
    pub fn new() -> Self {
        let cfg = match EthChannelConfig::from_env() {
            Ok(cfg) => cfg,
            Err(_) => {
                return Self {
                    mainnet: None,
                    testnet: None,
                    disabled_reason: Some(
                        "Missing INFURA_PROJECT_ID or ETHERSCAN_API_KEY environment variables"
                            .to_string(),
                    ),
                }
            }
        };

        let mainnet_rpc = match Provider::<Http>::try_from(cfg.mainnet_rpc_url.as_str()) {
            Ok(provider) => provider.interval(Duration::from_millis(250)),
            Err(err) => {
                return Self {
                    mainnet: None,
                    testnet: None,
                    disabled_reason: Some(format!("Invalid ETH mainnet RPC URL: {}", err)),
                }
            }
        };

        let testnet_rpc = match Provider::<Http>::try_from(cfg.testnet_rpc_url.as_str()) {
            Ok(provider) => provider.interval(Duration::from_millis(250)),
            Err(err) => {
                return Self {
                    mainnet: None,
                    testnet: None,
                    disabled_reason: Some(format!("Invalid ETH testnet RPC URL: {}", err)),
                }
            }
        };

        let history_client = EtherscanHistoryClient::new(
            cfg.etherscan_api_key,
            cfg.etherscan_mainnet_url,
            cfg.etherscan_testnet_url,
        );

        Self {
            mainnet: Some(EthNetworkProvider {
                chain_id: 1,
                rpc_provider: mainnet_rpc,
                history_provider: history_client.clone(),
            }),
            testnet: Some(EthNetworkProvider {
                chain_id: 5,
                rpc_provider: testnet_rpc,
                history_provider: history_client,
            }),
            disabled_reason: None,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.mainnet.is_some() && self.testnet.is_some()
    }

    pub fn disabled_reason(&self) -> Option<&str> {
        self.disabled_reason.as_deref()
    }

    pub fn for_network(&self, network: WalletNetwork) -> Result<&EthNetworkProvider, WalletError> {
        if !self.is_enabled() {
            return Err(WalletError::EthNotConfigured);
        }

        match network {
            WalletNetwork::Mainnet => self.mainnet.as_ref().ok_or(WalletError::EthNotConfigured),
            WalletNetwork::Testnet => self.testnet.as_ref().ok_or(WalletError::EthNotConfigured),
        }
    }
}

impl Default for EthProviderPool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct EtherscanHistoryClient {
    api_key: String,
    mainnet_url: String,
    testnet_url: String,
    client: Client,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct EtherscanTxRecord {
    #[serde(default)]
    pub block_number: String,
    #[serde(default)]
    pub time_stamp: String,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub nonce: String,
    #[serde(default)]
    pub block_hash: String,
    #[serde(default)]
    pub transaction_index: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub gas: String,
    #[serde(default)]
    pub gas_price: String,
    #[serde(default)]
    pub gas_used: String,
    #[serde(default)]
    pub cumulative_gas_used: String,
    #[serde(default)]
    pub contract_address: String,
    #[serde(default)]
    pub confirmations: String,
}

impl EtherscanHistoryClient {
    pub fn new(api_key: String, mainnet_url: String, testnet_url: String) -> Self {
        Self {
            api_key,
            mainnet_url,
            testnet_url,
            client: Client::builder()
                .connect_timeout(Duration::from_secs(4))
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    pub async fn get_eth_history(
        &self,
        network: WalletNetwork,
        address: &str,
    ) -> Result<Vec<EtherscanTxRecord>, WalletError> {
        self.fetch_history(network, "txlist", address, None).await
    }

    pub async fn get_erc20_history(
        &self,
        network: WalletNetwork,
        address: &str,
        contract_address: &str,
    ) -> Result<Vec<EtherscanTxRecord>, WalletError> {
        self.fetch_history(network, "tokentx", address, Some(contract_address))
            .await
    }

    async fn fetch_history(
        &self,
        network: WalletNetwork,
        action: &str,
        address: &str,
        contract_address: Option<&str>,
    ) -> Result<Vec<EtherscanTxRecord>, WalletError> {
        let url = match network {
            WalletNetwork::Mainnet => &self.mainnet_url,
            WalletNetwork::Testnet => &self.testnet_url,
        };

        let mut query: Vec<(&str, String)> = vec![
            ("module", "account".to_string()),
            ("action", action.to_string()),
            ("address", address.to_string()),
            ("startblock", "0".to_string()),
            ("endblock", "99999999".to_string()),
            ("sort", "asc".to_string()),
            ("apikey", self.api_key.clone()),
        ];

        if let Some(contract) = contract_address {
            query.push(("contractaddress", contract.to_string()));
        }

        let response = self
            .client
            .get(url)
            .query(&query)
            .send()
            .await
            .map_err(|_| WalletError::NetworkError)?;

        if !response.status().is_success() {
            return Err(WalletError::NetworkError);
        }

        let payload: Value = response.json().await.map_err(|_| WalletError::OperationFailed)?;

        let status = payload
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or_default();
        let message = payload
            .get("message")
            .and_then(|s| s.as_str())
            .unwrap_or_default();

        if status == "0" && message.eq_ignore_ascii_case("No transactions found") {
            return Ok(vec![]);
        }

        let records = payload
            .get("result")
            .and_then(|v| v.as_array())
            .ok_or(WalletError::OperationFailed)?;

        let mut out = Vec::with_capacity(records.len());
        for entry in records {
            let tx: EtherscanTxRecord = serde_json::from_value(entry.clone())
                .map_err(|_| WalletError::OperationFailed)?;
            out.push(tx);
        }

        Ok(out)
    }
}
