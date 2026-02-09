// 
// Cryptographic operations module
// Security: Implements Verus-Mobile v1 key derivation with secure memory handling
// Last Updated: Created for Module 2 integration

pub mod key_derivation_v1;
pub mod eth_keys;
pub mod wif_encoding;

pub use key_derivation_v1::derive_keys_v1;
pub use wif_encoding::Network;

#[cfg(test)]
mod tests {
    use super::{derive_keys_v1, Network};

    /// Test that key derivation produces consistent results
    /// TODO: Add known test vectors from Verus-Mobile for exact parity verification
    #[test]
    fn test_key_derivation_consistency() {
        let seed = "test seed phrase for wallet derivation testing purposes";
        let network = Network::Mainnet;
        
        let keys1 = derive_keys_v1(seed, network).expect("First derivation should succeed");
        let keys2 = derive_keys_v1(seed, network).expect("Second derivation should succeed");
        
        // Same seed should produce same keys
        assert_eq!(keys1.wif, keys2.wif);
        assert_eq!(keys1.address, keys2.address);
        assert_eq!(keys1.eth_address, keys2.eth_address);
    }
    
    /// Test that different seeds produce different keys
    #[test]
    fn test_key_derivation_uniqueness() {
        let seed1 = "first test seed phrase";
        let seed2 = "second test seed phrase";
        let network = Network::Mainnet;
        
        let keys1 = derive_keys_v1(seed1, network).expect("First derivation should succeed");
        let keys2 = derive_keys_v1(seed2, network).expect("Second derivation should succeed");
        
        // Different seeds should produce different keys
        assert_ne!(keys1.wif, keys2.wif);
        assert_ne!(keys1.address, keys2.address);
        assert_ne!(keys1.eth_address, keys2.eth_address);
    }
    
    // TODO: Test Verus-Mobile v1 parity
    // Add known test vectors from Verus-Mobile newsend2 branch:
    // - Seed phrase
    // - Expected WIF
    // - Expected VRSC address
    // - Expected ETH address
    // 
    // Example format:
    // #[test]
    // fn test_verus_mobile_v1_parity() {
    //     let seed = "known seed from Verus-Mobile";
    //     let keys = derive_keys_v1(seed, Network::Mainnet).unwrap();
    //     assert_eq!(keys.address, "expected_vrsc_address");
    //     assert_eq!(keys.eth_address, "expected_eth_address");
    // }
}
