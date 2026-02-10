//
// Verus-Mobile v1 key derivation implementation
// Security: Implements exact parity with Verus-Mobile newsend2 branch (sha256+iguana clamp)
// Last Updated: Added Bitcoin P2PKH address (same key, version byte 0x00) for Verus-Mobile parity

use crate::core::crypto::eth_keys::derive_eth_address;
use crate::core::crypto::wif_encoding::{
    encode_wif, generate_p2pkh_address, generate_p2pkh_address_with_version, Network,
};
use crate::types::errors::WalletError;
use crate::types::wallet::DerivedKeys;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};

/// Bitcoin mainnet P2PKH version byte (addresses start with "1")
const BITCOIN_P2PKH_VERSION_MAINNET: u8 = 0x00;
/// Bitcoin testnet P2PKH version byte (addresses start with "m" or "n")
const BITCOIN_P2PKH_VERSION_TESTNET: u8 = 0x6F;

/// Derive keys using Verus-Mobile v1 derivation method
///
/// Flow:
/// 1. SHA256(seed string as UTF-8) → 32 bytes
/// 2. Apply iguana clamp bit operations
/// 3. Validate scalar is in secp256k1 curve range [1, n-1]
/// 4. Derive compressed secp256k1 public key
/// 5. Encode WIF and P2PKH address
/// 6. Derive ETH keys from same scalar
///
/// Security: Never logs seed or derived keys
pub fn derive_keys_v1(seed: &str, network: Network) -> Result<DerivedKeys, WalletError> {
    // 1. SHA256(seed string as UTF-8 bytes) - no normalization
    let mut priv_bytes = sha256_digest(seed.as_bytes());

    // 2. Apply iguana clamp (ensures scalar is in valid range)
    // bytes[0] &= 248  (clear bottom 3 bits)
    priv_bytes[0] &= 248;
    // bytes[31] &= 127 (clear top bit)
    priv_bytes[31] &= 127;
    // bytes[31] |= 64  (set second-highest bit)
    priv_bytes[31] |= 64;

    // 3. Validate scalar is in range [1, n-1] where n is curve order
    let scalar = SecretKey::from_slice(&priv_bytes).map_err(|e| {
        // Never log the actual scalar value
        WalletError::Internal(format!("Invalid scalar: {}", e))
    })?;

    // Verify scalar is not zero (shouldn't happen after clamp, but check anyway)
    if scalar.as_ref().iter().all(|&b| b == 0) {
        return Err(WalletError::Internal("Derived scalar is zero".to_string()));
    }

    // 4. Derive compressed secp256k1 public key (33 bytes)
    let secp = Secp256k1::new();
    let pub_key = PublicKey::from_secret_key(&secp, &scalar);
    let pub_hex = hex::encode(pub_key.serialize());

    // 5. Encode WIF
    let wif = encode_wif(&priv_bytes, network)?;

    // 6. Generate P2PKH address
    let address = generate_p2pkh_address(&pub_key, network)?;

    // 7. Derive ETH keys from same scalar
    let eth_private_key = hex::encode(&priv_bytes);
    let eth_address = derive_eth_address(&pub_key)?;

    // 8. Bitcoin P2PKH address (same key, network-matched version byte)
    let btc_version = match network {
        Network::Mainnet => BITCOIN_P2PKH_VERSION_MAINNET,
        Network::Testnet => BITCOIN_P2PKH_VERSION_TESTNET,
    };
    let btc_address = generate_p2pkh_address_with_version(&pub_key, btc_version)?;

    Ok(DerivedKeys {
        wif,
        address,
        pub_hex,
        eth_private_key,
        eth_address,
        btc_address,
    })
}

/// Compute SHA256 digest of input bytes
fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    bytes
}
