use ethers::providers::Middleware;
use ethers::types::U256;

use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::types::WalletError;

pub const ETH_VERUS_BRIDGE_CONTRACT_RESERVE_TRANSFER_FEE_SATS: u64 = 300_000;
pub const ETH_VERUS_BRIDGE_CONTRACT_PRELAUNCH_RESERVE_TRANSFER_FEE_SATS: u64 = 2_000_000;
pub const ETH_VERUS_BRIDGE_CONTRACT_RESERVE_TRANSFER_FEE_WEI: u64 = 3_000_000_000_000_000;

pub const MINIMUM_GAS_PRICE_WEI_DELEGATOR_CONTRACT: u64 = 3_000_000_000;
pub const GAS_PRICE_MODIFIER_DELEGATOR_CONTRACT: u64 = 4;
pub const GAS_PRICE_MODIFIER_WEI: u64 = 2_000_000_000;
pub const MINIMUM_IMPORT_FEE_WEI: u64 = 10_000_000_000_000_000;
pub const GAS_TRANSACTION_IMPORT_FEE: u64 = 1_400_000;

pub const INITIAL_GAS_LIMIT: u64 = 6_000_000;
pub const FALLBACK_GAS_BRIDGE_TRANSFER: u64 = 1_000_000;
pub const ERC20_BRIDGE_TRANSFER_GAS_LIMIT: u64 = 750_000;
pub const DAI_BRIDGE_TRANSFER_GAS_LIMIT: u64 = 900_000;
pub const FALLBACK_APPROVAL_GAS_COST: u64 = 100_000;

const DAI_CONTRACT_ADDRESS: &str = "0x6b175474e89094c44da98b954eedeac495271d0f";
const USDT_CONTRACT_ADDRESS: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";

#[derive(Debug, Clone, Default)]
pub struct BridgeFeeQuote {
    pub network_fee_wei: String,
    pub bridge_fee_wei: String,
    pub import_fee_wei: Option<String>,
    pub total_max_fee_wei: String,
    pub gas_limit: String,
    pub transfer_gas_limit: String,
    pub approval_gas_limit: String,
    pub max_fee_per_gas: String,
}

pub async fn current_bridge_max_fee_per_gas(
    provider: &EthNetworkProvider,
) -> Result<U256, WalletError> {
    let fee_data = provider
        .rpc_provider
        .estimate_eip1559_fees(None)
        .await
        .map_err(|_| WalletError::NetworkError)?;

    let market = fee_data.0;
    let modified = market.saturating_add(U256::from(GAS_PRICE_MODIFIER_WEI));
    let minimum = U256::from(MINIMUM_GAS_PRICE_WEI_DELEGATOR_CONTRACT);
    Ok(modified.max(minimum))
}

pub fn reserve_transfer_fee_sats(past_prelaunch: bool) -> u64 {
    if past_prelaunch {
        ETH_VERUS_BRIDGE_CONTRACT_RESERVE_TRANSFER_FEE_SATS
    } else {
        ETH_VERUS_BRIDGE_CONTRACT_PRELAUNCH_RESERVE_TRANSFER_FEE_SATS
    }
}

pub fn base_bridge_fee_wei(past_prelaunch: bool) -> U256 {
    if past_prelaunch {
        U256::from(ETH_VERUS_BRIDGE_CONTRACT_RESERVE_TRANSFER_FEE_WEI)
    } else {
        U256::zero()
    }
}

pub fn import_fee_wei(max_fee_per_gas: U256) -> U256 {
    let market = max_fee_per_gas.saturating_mul(U256::from(GAS_TRANSACTION_IMPORT_FEE));
    let minimum = U256::from(MINIMUM_IMPORT_FEE_WEI);
    market.max(minimum)
}

pub fn approval_zero_out_required(token_contract: &str) -> bool {
    token_contract.eq_ignore_ascii_case(USDT_CONTRACT_ADDRESS)
}

pub fn approval_estimate_skip(token_contract: &str) -> bool {
    token_contract.eq_ignore_ascii_case(USDT_CONTRACT_ADDRESS)
}

pub fn transfer_gas_limit_for_token(token_contract: &str) -> U256 {
    if token_contract.eq_ignore_ascii_case(DAI_CONTRACT_ADDRESS) {
        U256::from(DAI_BRIDGE_TRANSFER_GAS_LIMIT)
    } else {
        U256::from(ERC20_BRIDGE_TRANSFER_GAS_LIMIT)
    }
}

pub fn add_fraction(value: U256, divisor: u64) -> U256 {
    if divisor == 0 {
        return value;
    }
    value.saturating_add(value / U256::from(divisor))
}

#[cfg(test)]
mod tests {
    use super::{
        approval_estimate_skip, approval_zero_out_required, base_bridge_fee_wei, import_fee_wei,
        reserve_transfer_fee_sats, MINIMUM_IMPORT_FEE_WEI,
    };
    use ethers::types::U256;

    #[test]
    fn prelaunch_uses_zero_base_bridge_fee_wei() {
        assert!(base_bridge_fee_wei(false).is_zero());
        assert!(!base_bridge_fee_wei(true).is_zero());
    }

    #[test]
    fn reserve_transfer_fee_sats_switches_on_launch_state() {
        assert!(reserve_transfer_fee_sats(true) < reserve_transfer_fee_sats(false));
    }

    #[test]
    fn import_fee_wei_honors_minimum_floor() {
        let low_fee = import_fee_wei(U256::from(1u64));
        assert_eq!(low_fee, U256::from(MINIMUM_IMPORT_FEE_WEI));
    }

    #[test]
    fn usdt_requires_approval_zero_out_and_skip_estimate() {
        assert!(approval_zero_out_required(
            "0xdac17f958d2ee523a2206206994597c13d831ec7"
        ));
        assert!(approval_estimate_skip(
            "0xDAC17F958D2EE523A2206206994597C13D831EC7"
        ));
    }
}
