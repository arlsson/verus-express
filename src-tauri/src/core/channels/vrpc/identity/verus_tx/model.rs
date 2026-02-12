//
// Verus Overwinter/Sapling transparent transaction model used by identity flows.

use crate::types::WalletError;

pub const OVERWINTER_MASK: u32 = 1 << 31;
pub const SAPLING_VERSION_GROUP_ID: u32 = 0x892f2085;
pub const OVERWINTER_VERSION_GROUP_ID: u32 = 0x03c4_8270;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerusTxIn {
    pub prevout_txid_le: [u8; 32],
    pub prevout_vout: u32,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerusTxOut {
    pub value: u64,
    pub script_pub_key: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerusTx {
    pub version: u32,
    pub overwintered: bool,
    pub version_group_id: u32,
    pub inputs: Vec<VerusTxIn>,
    pub outputs: Vec<VerusTxOut>,
    pub lock_time: u32,
    pub expiry_height: u32,
    // Transparent-only scope: valueBalance must remain zero.
    pub value_balance: i64,
}

impl VerusTx {
    pub fn is_overwinter_compatible(&self) -> bool {
        self.overwintered && self.version >= 3
    }

    pub fn is_sapling_compatible(&self) -> bool {
        self.overwintered && self.version >= 4
    }

    pub fn header(&self) -> u32 {
        let masked = self.version & 0x7fff_ffff;
        if self.overwintered {
            masked | OVERWINTER_MASK
        } else {
            masked
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSignMode {
    P2pkh,
    SmartTransaction,
}

pub fn consensus_branch_id_for_version(version: u32) -> Result<u32, WalletError> {
    match version {
        3 => Ok(0x5ba8_1b19),
        4 => Ok(0x76b8_09bb),
        _ => Err(WalletError::IdentitySignFailed),
    }
}

pub fn txid_hex_to_le_bytes(txid_hex: &str) -> Result<[u8; 32], WalletError> {
    let mut bytes = hex::decode(txid_hex).map_err(|_| WalletError::IdentityBuildFailed)?;
    if bytes.len() != 32 {
        return Err(WalletError::IdentityBuildFailed);
    }
    bytes.reverse();
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

pub fn txid_le_bytes_to_hex(txid_le: &[u8; 32]) -> String {
    let mut bytes = txid_le.to_vec();
    bytes.reverse();
    hex::encode(bytes)
}
