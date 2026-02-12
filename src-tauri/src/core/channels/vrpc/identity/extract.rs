//
// Helpers to extract identity definition output from a raw transaction.

use std::io::Cursor;

use bitcoin::consensus::Decodable;

use crate::types::WalletError;

const OP_CHECKCRYPTOCONDITION: u8 = 0xCC;
const OP_DROP: u8 = 0x75;

#[derive(Debug, Clone)]
pub struct IdentityOutputRef {
    pub vout: u32,
    pub script_hex: String,
}

fn read_push_data(script: &[u8], start: usize) -> Option<(usize, &[u8])> {
    let opcode = *script.get(start)?;
    match opcode {
        1..=75 => {
            let len = opcode as usize;
            let begin = start + 1;
            let end = begin + len;
            let data = script.get(begin..end)?;
            Some((end, data))
        }
        76 => {
            let len = *script.get(start + 1)? as usize;
            let begin = start + 2;
            let end = begin + len;
            let data = script.get(begin..end)?;
            Some((end, data))
        }
        77 => {
            let lo = *script.get(start + 1)? as usize;
            let hi = *script.get(start + 2)? as usize;
            let len = lo | (hi << 8);
            let begin = start + 3;
            let end = begin + len;
            let data = script.get(begin..end)?;
            Some((end, data))
        }
        78 => {
            let b0 = *script.get(start + 1)? as usize;
            let b1 = *script.get(start + 2)? as usize;
            let b2 = *script.get(start + 3)? as usize;
            let b3 = *script.get(start + 4)? as usize;
            let len = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
            let begin = start + 5;
            let end = begin + len;
            let data = script.get(begin..end)?;
            Some((end, data))
        }
        _ => None,
    }
}

fn is_identity_cc_script(script: &[u8]) -> bool {
    let Some((idx_after_master, _master)) = read_push_data(script, 0) else {
        return false;
    };
    if script.get(idx_after_master) != Some(&OP_CHECKCRYPTOCONDITION) {
        return false;
    }
    let Some((idx_after_params, _params)) = read_push_data(script, idx_after_master + 1) else {
        return false;
    };
    script.get(idx_after_params) == Some(&OP_DROP) && idx_after_params + 1 == script.len()
}

pub fn extract_identity_output(
    raw_tx_hex: &str,
    hinted_vout: Option<u32>,
) -> Result<IdentityOutputRef, WalletError> {
    let raw = hex::decode(raw_tx_hex.trim_start_matches("0x"))
        .or_else(|_| hex::decode(raw_tx_hex))
        .map_err(|_| WalletError::IdentityBuildFailed)?;
    let mut cursor = Cursor::new(&raw[..]);
    let tx: bitcoin::Transaction = bitcoin::Transaction::consensus_decode(&mut cursor)
        .map_err(|_| WalletError::IdentityBuildFailed)?;

    if let Some(vout) = hinted_vout {
        if let Some(out) = tx.output.get(vout as usize) {
            let script = out.script_pubkey.as_bytes();
            if is_identity_cc_script(script) {
                return Ok(IdentityOutputRef {
                    vout,
                    script_hex: hex::encode(script),
                });
            }
        }
    }

    for (idx, out) in tx.output.iter().enumerate() {
        let script = out.script_pubkey.as_bytes();
        if is_identity_cc_script(script) {
            return Ok(IdentityOutputRef {
                vout: idx as u32,
                script_hex: hex::encode(script),
            });
        }
    }

    Err(WalletError::IdentityBuildFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_push(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(data.len() as u8);
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn detects_identity_cc_script_shape() {
        let mut script = Vec::new();
        script.extend_from_slice(&make_push(&[1u8; 4]));
        script.push(OP_CHECKCRYPTOCONDITION);
        script.extend_from_slice(&make_push(&[2u8; 8]));
        script.push(OP_DROP);
        assert!(is_identity_cc_script(&script));
    }

    #[test]
    fn rejects_wrong_script_shape() {
        let script = vec![0x51, 0x21, 0x02, 0x03, 0x04];
        assert!(!is_identity_cc_script(&script));
    }
}
