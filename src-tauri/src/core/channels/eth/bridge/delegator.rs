//
// Verus bridge delegator contract metadata and helpers.

use ethers::types::Address;

use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

pub const VERUS_BRIDGE_DELEGATOR_MAINNET_CONTRACT: &str =
    "0x71518580f36FeCEFfE0721F06bA4703218cD7F63";
pub const VERUS_BRIDGE_DELEGATOR_GOERLI_CONTRACT: &str =
    "0x85a7de2278e52327471e174aeeb280cdfdc6a68a";

pub fn delegator_contract_for_network(network: WalletNetwork) -> Result<Address, WalletError> {
    let raw = match network {
        WalletNetwork::Mainnet => VERUS_BRIDGE_DELEGATOR_MAINNET_CONTRACT,
        WalletNetwork::Testnet => VERUS_BRIDGE_DELEGATOR_GOERLI_CONTRACT,
    };

    raw.parse::<Address>()
        .map_err(|_| WalletError::OperationFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mainnet_delegator_is_valid_eth_address() {
        let addr = delegator_contract_for_network(WalletNetwork::Mainnet).expect("mainnet address");
        assert_ne!(addr, Address::zero());
    }

    #[test]
    fn testnet_delegator_is_valid_eth_address() {
        let addr = delegator_contract_for_network(WalletNetwork::Testnet).expect("testnet address");
        assert_ne!(addr, Address::zero());
    }
}
