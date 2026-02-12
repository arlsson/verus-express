//
// Script classification and scriptSig builders for Verus identity tx signing.

use crate::core::channels::vrpc::identity::verus_tx::model::InputSignMode;
use crate::types::WalletError;

const OP_DUP: u8 = 0x76;
const OP_HASH160: u8 = 0xa9;
const OP_EQUALVERIFY: u8 = 0x88;
const OP_CHECKSIG: u8 = 0xac;
const OP_CHECKCRYPTOCONDITION: u8 = 0xcc;
const OP_DROP: u8 = 0x75;

fn read_push_data(script: &[u8], start: usize) -> Option<(usize, &[u8])> {
    let opcode = *script.get(start)?;
    match opcode {
        1..=75 => {
            let len = opcode as usize;
            let begin = start + 1;
            let end = begin + len;
            Some((end, script.get(begin..end)?))
        }
        76 => {
            let len = *script.get(start + 1)? as usize;
            let begin = start + 2;
            let end = begin + len;
            Some((end, script.get(begin..end)?))
        }
        77 => {
            let lo = *script.get(start + 1)? as usize;
            let hi = *script.get(start + 2)? as usize;
            let len = lo | (hi << 8);
            let begin = start + 3;
            let end = begin + len;
            Some((end, script.get(begin..end)?))
        }
        78 => {
            let b0 = *script.get(start + 1)? as usize;
            let b1 = *script.get(start + 2)? as usize;
            let b2 = *script.get(start + 3)? as usize;
            let b3 = *script.get(start + 4)? as usize;
            let len = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
            let begin = start + 5;
            let end = begin + len;
            Some((end, script.get(begin..end)?))
        }
        _ => None,
    }
}

pub fn is_p2pkh_script(script: &[u8]) -> bool {
    script.len() == 25
        && script[0] == OP_DUP
        && script[1] == OP_HASH160
        && script[2] == 0x14
        && script[23] == OP_EQUALVERIFY
        && script[24] == OP_CHECKSIG
}

pub fn is_smart_transaction_script(script: &[u8]) -> bool {
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

pub fn classify_prevout_script(script: &[u8]) -> Result<InputSignMode, WalletError> {
    if is_p2pkh_script(script) {
        Ok(InputSignMode::P2pkh)
    } else if is_smart_transaction_script(script) {
        Ok(InputSignMode::SmartTransaction)
    } else {
        Err(WalletError::IdentityBuildFailed)
    }
}

fn push_data(script: &mut Vec<u8>, data: &[u8]) -> Result<(), WalletError> {
    let len = data.len();
    if len <= 75 {
        script.push(len as u8);
    } else if len <= 0xff {
        script.push(76);
        script.push(len as u8);
    } else if len <= 0xffff {
        script.push(77);
        script.extend_from_slice(&(len as u16).to_le_bytes());
    } else if len <= 0xffff_ffff {
        script.push(78);
        script.extend_from_slice(&(len as u32).to_le_bytes());
    } else {
        return Err(WalletError::IdentitySignFailed);
    }
    script.extend_from_slice(data);
    Ok(())
}

pub fn build_p2pkh_script_sig(
    der_plus_hashtype: &[u8],
    compressed_pubkey: &[u8],
) -> Result<Vec<u8>, WalletError> {
    if compressed_pubkey.len() != 33 {
        return Err(WalletError::IdentitySignFailed);
    }
    let mut script = Vec::with_capacity(der_plus_hashtype.len() + compressed_pubkey.len() + 8);
    push_data(&mut script, der_plus_hashtype)?;
    push_data(&mut script, compressed_pubkey)?;
    Ok(script)
}

pub fn build_single_push_script_sig(payload: &[u8]) -> Result<Vec<u8>, WalletError> {
    let mut script = Vec::with_capacity(payload.len() + 6);
    push_data(&mut script, payload)?;
    Ok(script)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_p2pkh() {
        let script =
            hex::decode("76a914111111111111111111111111111111111111111188ac").expect("script");
        assert_eq!(
            classify_prevout_script(&script).expect("classify"),
            InputSignMode::P2pkh
        );
    }

    #[test]
    fn classifies_smart_transaction_shape() {
        let script = hex::decode("0401010101cc08020202020202020275").expect("script");
        assert_eq!(
            classify_prevout_script(&script).expect("classify"),
            InputSignMode::SmartTransaction
        );
    }

    #[test]
    fn builds_single_push_script_sig_for_smart_signature_chunk() {
        let payload = hex::decode(
            "0101010121025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486402142a1545964b8ca2663a957d37e1d7e33fb3b1a4d214b0c657a04ee3aacd81237bf657565beddf2fab20829de48258bc96b88aaf6f0ceef7832c265e2da2852",
        )
        .expect("payload");
        let sig = build_single_push_script_sig(&payload).expect("script sig");
        assert_eq!(
            hex::encode(sig),
            "4c670101010121025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486402142a1545964b8ca2663a957d37e1d7e33fb3b1a4d214b0c657a04ee3aacd81237bf657565beddf2fab20829de48258bc96b88aaf6f0ceef7832c265e2da2852"
        );
    }
}
