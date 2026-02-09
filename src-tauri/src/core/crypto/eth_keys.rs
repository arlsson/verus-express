// 
// Ethereum key derivation from Verus keys
// Security: ETH uses same secp256k1 key as Verus, different encoding/address format
// Last Updated: Created for Module 2 integration

use secp256k1::PublicKey;
use tiny_keccak::{Keccak, Hasher};
use crate::types::errors::WalletError;
use crate::core::crypto::wif_encoding::{Network, decode_wif};

/// Derive Ethereum address from secp256k1 public key
/// 
/// Flow:
/// 1. Get uncompressed public key (65 bytes: 0x04 + x + y)
/// 2. Skip first byte (0x04), hash remaining 64 bytes with keccak256
/// 3. Take last 20 bytes → ETH address
pub fn derive_eth_address(pub_key: &PublicKey) -> Result<String, WalletError> {
    // Get uncompressed public key (65 bytes: 0x04 + x + y)
    let uncompressed = pub_key.serialize_uncompressed();
    
    // Skip first byte (0x04 compression indicator), hash remaining 64 bytes
    let mut keccak = Keccak::v256();
    keccak.update(&uncompressed[1..]);
    let mut hash = [0u8; 32];
    keccak.finalize(&mut hash);
    
    // Take last 20 bytes for address
    let address_bytes = &hash[12..];
    
    Ok(format!("0x{}", hex::encode(address_bytes)))
}

/// Convert WIF to Ethereum private key (hex format)
/// 
/// Both use the same secp256k1 scalar, just different encoding:
/// - WIF: base58check encoded with version byte
/// - ETH: hex string (0x prefix optional)
pub fn wif_to_eth_private_key(wif: &str, network: Network) -> Result<String, WalletError> {
    let decoded = decode_wif(wif, network)?;
    Ok(hex::encode(decoded))
}
