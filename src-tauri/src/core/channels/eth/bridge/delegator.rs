//
// Verus bridge delegator contract metadata and helpers.

use ethers::contract::abigen;
use ethers::types::Address;

use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

pub const VERUS_BRIDGE_DELEGATOR_MAINNET_CONTRACT: &str =
    "0x71518580f36FeCEFfE0721F06bA4703218cD7F63";
pub const VERUS_BRIDGE_DELEGATOR_GOERLI_CONTRACT: &str =
    "0x85a7de2278e52327471e174aeeb280cdfdc6a68a";

abigen!(
    VerusBridgeDelegatorContract,
    r#"[
      {
        "inputs": [],
        "name": "bridgeConverterActive",
        "outputs": [{"internalType": "bool", "name": "", "type": "bool"}],
        "stateMutability": "view",
        "type": "function"
      },
      {
        "inputs": [
          {"internalType": "uint256", "name": "start", "type": "uint256"},
          {"internalType": "uint256", "name": "end", "type": "uint256"}
        ],
        "name": "getTokenList",
        "outputs": [
          {
            "components": [
              {"internalType": "address", "name": "iaddress", "type": "address"},
              {"internalType": "address", "name": "erc20ContractAddress", "type": "address"},
              {"internalType": "address", "name": "launchSystemID", "type": "address"},
              {"internalType": "uint8", "name": "flags", "type": "uint8"},
              {"internalType": "string", "name": "name", "type": "string"},
              {"internalType": "string", "name": "ticker", "type": "string"},
              {"internalType": "uint256", "name": "tokenID", "type": "uint256"}
            ],
            "internalType": "struct VerusObjects.setupToken[]",
            "name": "",
            "type": "tuple[]"
          }
        ],
        "stateMutability": "nonpayable",
        "type": "function"
      },
      {
        "inputs": [
          {
            "components": [
              {"internalType": "uint32", "name": "version", "type": "uint32"},
              {
                "components": [
                  {"internalType": "address", "name": "currency", "type": "address"},
                  {"internalType": "uint64", "name": "amount", "type": "uint64"}
                ],
                "internalType": "struct VerusObjects.CCurrencyValueMap",
                "name": "currencyvalue",
                "type": "tuple"
              },
              {"internalType": "uint32", "name": "flags", "type": "uint32"},
              {"internalType": "address", "name": "feecurrencyid", "type": "address"},
              {"internalType": "uint64", "name": "fees", "type": "uint64"},
              {
                "components": [
                  {"internalType": "uint8", "name": "destinationtype", "type": "uint8"},
                  {"internalType": "bytes", "name": "destinationaddress", "type": "bytes"}
                ],
                "internalType": "struct VerusObjectsCommon.CTransferDestination",
                "name": "destination",
                "type": "tuple"
              },
              {"internalType": "address", "name": "destcurrencyid", "type": "address"},
              {"internalType": "address", "name": "destsystemid", "type": "address"},
              {"internalType": "address", "name": "secondreserveid", "type": "address"}
            ],
            "internalType": "struct VerusObjects.CReserveTransfer",
            "name": "_transfer",
            "type": "tuple"
          }
        ],
        "name": "sendTransfer",
        "outputs": [],
        "stateMutability": "payable",
        "type": "function"
      }
    ]"#,
);

pub fn delegator_contract_for_network(network: WalletNetwork) -> Result<Address, WalletError> {
    let raw = match network {
        WalletNetwork::Mainnet => VERUS_BRIDGE_DELEGATOR_MAINNET_CONTRACT,
        WalletNetwork::Testnet => VERUS_BRIDGE_DELEGATOR_GOERLI_CONTRACT,
    };

    raw.parse::<Address>()
        .map_err(|_| WalletError::OperationFailed)
}

pub fn delegator_contract_for_chain_id(chain_id: u64) -> Result<Address, WalletError> {
    let network = if chain_id == 1 {
        WalletNetwork::Mainnet
    } else {
        WalletNetwork::Testnet
    };
    delegator_contract_for_network(network)
}

#[cfg(test)]
mod tests {
    use super::{delegator_contract_for_chain_id, delegator_contract_for_network};
    use crate::types::wallet::WalletNetwork;
    use ethers::types::Address;

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

    #[test]
    fn chain_id_resolves_expected_network_contracts() {
        let mainnet = delegator_contract_for_chain_id(1).expect("mainnet");
        let testnet = delegator_contract_for_chain_id(5).expect("testnet");
        assert_ne!(mainnet, testnet);
    }
}
