// 
// WIF and P2PKH address encoding with Verus network parameters
// Security: Implements Verus-Mobile compatible encoding (no sensitive data in logs)
// Last Updated: Created for Module 2 integration

use secp256k1::PublicKey;
use bs58;
use sha2::{Sha256, Digest};
use ripemd::Ripemd160;
use crate::types::errors::WalletError;

/// Network type for Verus (mainnet or testnet)
#[derive(Debug, Clone, Copy)]
pub enum Network {
    Mainnet,
    Testnet,
}

impl Network {
    /// WIF version byte for Verus network
    pub fn wif_version(&self) -> u8 {
        match self {
            Network::Mainnet => 0xBC,  // 188
            Network::Testnet => 0x80,  // 128
        }
    }
    
    /// P2PKH address version byte for Verus network
    pub fn p2pkh_version(&self) -> u8 {
        match self {
            Network::Mainnet => 0x3C,  // 60
            Network::Testnet => 0x6F,  // 111
        }
    }
}

/// Encode a 32-byte private key as WIF (Wallet Import Format)
/// Format: [version byte][32-byte key][0x01 compression flag] + checksum → base58check
pub fn encode_wif(priv_key: &[u8; 32], network: Network) -> Result<String, WalletError> {
    let mut data = Vec::with_capacity(34);
    data.push(network.wif_version());
    data.extend_from_slice(priv_key);
    data.push(0x01); // compressed flag
    
    Ok(bs58::encode(data).with_check().into_string())
}

/// Decode a WIF string to extract the 32-byte private key
pub fn decode_wif(wif: &str, network: Network) -> Result<[u8; 32], WalletError> {
    let decoded = bs58::decode(wif)
        .with_check(None)
        .into_vec()
        .map_err(|_| WalletError::InvalidAddress)?;
    
    // Verify version byte
    if decoded.is_empty() || decoded[0] != network.wif_version() {
        return Err(WalletError::InvalidAddress);
    }
    
    // Extract 32-byte private key (skip version byte and compression flag)
    if decoded.len() < 34 {
        return Err(WalletError::InvalidAddress);
    }
    
    let mut priv_key = [0u8; 32];
    priv_key.copy_from_slice(&decoded[1..33]);
    
    Ok(priv_key)
}

/// Generate a P2PKH address from a compressed public key
/// Format: [version byte][RIPEMD160(SHA256(pubkey))] + checksum → base58check
pub fn generate_p2pkh_address(pub_key: &PublicKey, network: Network) -> Result<String, WalletError> {
    // Serialize compressed public key (33 bytes)
    let pub_bytes = pub_key.serialize();
    
    // Compute hash160: RIPEMD160(SHA256(pubkey))
    let hash = hash160(&pub_bytes);
    
    // Prepend version byte
    let mut data = Vec::with_capacity(21);
    data.push(network.p2pkh_version());
    data.extend_from_slice(&hash);
    
    Ok(bs58::encode(data).with_check().into_string())
}

/// Compute hash160: RIPEMD160(SHA256(data))
fn hash160(data: &[u8]) -> [u8; 20] {
    // SHA256
    let sha = Sha256::digest(data);
    
    // RIPEMD160
    let mut hasher = Ripemd160::new();
    hasher.update(&sha);
    let ripemd = hasher.finalize();
    
    let mut result = [0u8; 20];
    result.copy_from_slice(&ripemd[..]);
    result
}
