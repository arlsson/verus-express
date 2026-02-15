use std::sync::Arc;

use ethers::abi::Abi;
use ethers::contract::Contract;
use ethers::middleware::SignerMiddleware;
use ethers::providers::Middleware;
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{Address, Eip1559TransactionRequest, U256};
use tokio::sync::Mutex;

use crate::core::auth::SessionManager;
use crate::core::channels::eth::preflight::EthPreflightPayload;
use crate::core::channels::eth::provider::EthProviderPool;
use crate::core::channels::store::PreflightStore;
use crate::types::transaction::SendResult;
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

const ERC20_TRANSFER_ABI: &str = r#"[
  {
    "constant": false,
    "inputs": [
      {"name": "_to", "type": "address"},
      {"name": "_value", "type": "uint256"}
    ],
    "name": "transfer",
    "outputs": [{"name": "", "type": "bool"}],
    "type": "function"
  }
]"#;

pub async fn send(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    provider_pool: &EthProviderPool,
) -> Result<SendResult, WalletError> {
    let record = preflight_store
        .take(preflight_id)
        .ok_or(WalletError::InvalidPreflight)?;

    let payload: EthPreflightPayload =
        serde_json::from_value(record.payload).map_err(|_| WalletError::InvalidPreflight)?;

    let session = session_manager.lock().await;
    let active_id = session
        .active_account_id()
        .ok_or(WalletError::WalletLocked)?;
    if active_id.as_str() != record.account_id {
        return Err(WalletError::InvalidPreflight);
    }

    let wallet_network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let eth_private_key = session.get_eth_private_key_for_signing()?;
    drop(session);

    let network_provider = provider_pool.for_network(wallet_network)?;
    let private_hex = if eth_private_key.starts_with("0x") {
        eth_private_key
    } else {
        format!("0x{}", eth_private_key)
    };

    let wallet = private_hex
        .parse::<LocalWallet>()
        .map_err(|_| WalletError::OperationFailed)?
        .with_chain_id(network_provider.chain_id);

    let signer = Arc::new(SignerMiddleware::new(
        network_provider.rpc_provider.clone(),
        wallet,
    ));

    match payload {
        EthPreflightPayload::Eth {
            from_address,
            to_address,
            value_wei,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            fee,
            value,
            ..
        } => {
            let parsed_from: Address = from_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let parsed_to: Address = to_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let value_wei = parse_u256(&value_wei)?;
            let gas_limit = parse_u256(&gas_limit)?;
            let max_fee_per_gas = parse_u256(&max_fee_per_gas)?;
            let max_priority_fee_per_gas = parse_u256(&max_priority_fee_per_gas)?;

            let tx = Eip1559TransactionRequest::new()
                .from(parsed_from)
                .to(parsed_to)
                .value(value_wei)
                .gas(gas_limit)
                .max_fee_per_gas(max_fee_per_gas)
                .max_priority_fee_per_gas(max_priority_fee_per_gas)
                .chain_id(network_provider.chain_id);

            let pending = signer
                .send_transaction(tx, None)
                .await
                .map_err(|_| WalletError::NetworkError)?;

            Ok(SendResult {
                txid: format!("{:#x}", pending.tx_hash()),
                fee,
                value,
                to_address,
                from_address,
            })
        }
        EthPreflightPayload::Erc20 {
            from_address,
            to_address,
            token_address,
            token_value_raw,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            max_fee_cap,
            fee,
            value,
            ..
        } => {
            let parsed_from: Address = from_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let parsed_to: Address = to_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let token_address: Address = token_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let amount_raw = parse_u256(&token_value_raw)?;
            let gas_limit = parse_u256(&gas_limit)?;
            let max_fee_per_gas = parse_u256(&max_fee_per_gas)?;
            let max_priority_fee_per_gas = parse_u256(&max_priority_fee_per_gas)?;
            let max_fee_cap = parse_u256(&max_fee_cap)?;

            let fee_data = network_provider
                .rpc_provider
                .estimate_eip1559_fees(None)
                .await
                .map_err(|_| WalletError::NetworkError)?;
            let current_max_fee = fee_data.0;

            if fee_drift_exceeds_cap(gas_limit, current_max_fee, max_fee_cap) {
                return Err(WalletError::OperationFailed);
            }

            let abi: Abi = serde_json::from_str(ERC20_TRANSFER_ABI)
                .map_err(|_| WalletError::OperationFailed)?;
            let contract = Contract::new(token_address, abi, signer.clone());

            let transfer_call = contract
                .method::<_, bool>("transfer", (parsed_to, amount_raw))
                .map_err(|_| WalletError::OperationFailed)?;

            let configured_call = transfer_call
                .from(parsed_from)
                .gas(gas_limit)
                .gas_price(max_fee_per_gas.max(max_priority_fee_per_gas));

            let pending = configured_call
                .send()
                .await
                .map_err(|_| WalletError::NetworkError)?;

            Ok(SendResult {
                txid: format!("{:#x}", pending.tx_hash()),
                fee,
                value,
                to_address,
                from_address,
            })
        }
    }
}

fn parse_u256(input: &str) -> Result<U256, WalletError> {
    U256::from_dec_str(input.trim()).map_err(|_| WalletError::OperationFailed)
}

fn fee_drift_exceeds_cap(
    gas_limit: U256,
    current_max_fee_per_gas: U256,
    max_fee_cap: U256,
) -> bool {
    gas_limit.saturating_mul(current_max_fee_per_gas) > max_fee_cap
}

#[cfg(test)]
mod tests {
    use super::fee_drift_exceeds_cap;
    use ethers::types::U256;

    #[test]
    fn fee_drift_exceeds_cap_returns_true_when_current_fee_is_higher_than_preflight_cap() {
        let gas_limit = U256::from(100_000u64);
        let current_fee = U256::from(40u64);
        let cap = U256::from(3_900_000u64);

        assert!(fee_drift_exceeds_cap(gas_limit, current_fee, cap));
    }

    #[test]
    fn fee_drift_exceeds_cap_returns_false_when_within_cap() {
        let gas_limit = U256::from(100_000u64);
        let current_fee = U256::from(39u64);
        let cap = U256::from(3_900_000u64);

        assert!(!fee_drift_exceeds_cap(gas_limit, current_fee, cap));
    }
}
