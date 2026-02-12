//
// Zcash-compatible transparent sighash (Verus Overwinter/Sapling).

use blake2b_simd::Params;

use crate::core::channels::vrpc::identity::verus_tx::codec::encode_output_bytes;
use crate::core::channels::vrpc::identity::verus_tx::model::{
    consensus_branch_id_for_version, VerusTx,
};
use crate::types::WalletError;

pub const SIGHASH_ALL: u32 = 0x01;
const SIGHASH_NONE: u32 = 0x02;
const SIGHASH_SINGLE: u32 = 0x03;
const SIGHASH_ANYONECANPAY: u32 = 0x80;

const ZERO_HASH: [u8; 32] = [0u8; 32];
const PERSONAL_PREVOUT: [u8; 16] = *b"ZcashPrevoutHash";
const PERSONAL_SEQUENCE: [u8; 16] = *b"ZcashSequencHash";
const PERSONAL_OUTPUTS: [u8; 16] = *b"ZcashOutputsHash";

fn write_compact_size(out: &mut Vec<u8>, value: u64) {
    match value {
        0x00..=0xfc => out.push(value as u8),
        0xfd..=0xffff => {
            out.push(0xfd);
            out.extend_from_slice(&(value as u16).to_le_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(0xfe);
            out.extend_from_slice(&(value as u32).to_le_bytes());
        }
        _ => {
            out.push(0xff);
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
}

fn write_var_slice(out: &mut Vec<u8>, bytes: &[u8]) {
    write_compact_size(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

fn blake2b_hash_32(data: &[u8], personal: &[u8; 16]) -> [u8; 32] {
    let hash = Params::new().hash_length(32).personal(personal).hash(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

fn get_prevout_hash(tx: &VerusTx, hash_type: u32) -> [u8; 32] {
    if hash_type & SIGHASH_ANYONECANPAY != 0 {
        return ZERO_HASH;
    }
    let mut data = Vec::with_capacity(tx.inputs.len() * 36);
    for input in &tx.inputs {
        data.extend_from_slice(&input.prevout_txid_le);
        data.extend_from_slice(&input.prevout_vout.to_le_bytes());
    }
    blake2b_hash_32(&data, &PERSONAL_PREVOUT)
}

fn get_sequence_hash(tx: &VerusTx, hash_type: u32) -> [u8; 32] {
    let hash_mode = hash_type & 0x1f;
    if hash_type & SIGHASH_ANYONECANPAY != 0
        || hash_mode == SIGHASH_SINGLE
        || hash_mode == SIGHASH_NONE
    {
        return ZERO_HASH;
    }

    let mut data = Vec::with_capacity(tx.inputs.len() * 4);
    for input in &tx.inputs {
        data.extend_from_slice(&input.sequence.to_le_bytes());
    }
    blake2b_hash_32(&data, &PERSONAL_SEQUENCE)
}

fn get_outputs_hash(tx: &VerusTx, hash_type: u32, in_index: usize) -> [u8; 32] {
    let hash_mode = hash_type & 0x1f;

    if hash_mode != SIGHASH_SINGLE && hash_mode != SIGHASH_NONE {
        let mut data = Vec::new();
        for output in &tx.outputs {
            data.extend_from_slice(&encode_output_bytes(output));
        }
        return blake2b_hash_32(&data, &PERSONAL_OUTPUTS);
    }

    if hash_mode == SIGHASH_SINGLE && in_index < tx.outputs.len() {
        return blake2b_hash_32(
            &encode_output_bytes(&tx.outputs[in_index]),
            &PERSONAL_OUTPUTS,
        );
    }

    ZERO_HASH
}

pub fn signature_hash(
    tx: &VerusTx,
    in_index: usize,
    prev_out_script: &[u8],
    value: u64,
    hash_type: u32,
) -> Result<[u8; 32], WalletError> {
    if !tx.is_overwinter_compatible() {
        return Err(WalletError::IdentitySignFailed);
    }
    if in_index >= tx.inputs.len() {
        return Err(WalletError::IdentitySignFailed);
    }

    let consensus_branch_id = consensus_branch_id_for_version(tx.version)?;
    let hash_prevouts = get_prevout_hash(tx, hash_type);
    let hash_sequence = get_sequence_hash(tx, hash_type);
    let hash_outputs = get_outputs_hash(tx, hash_type, in_index);
    let hash_joinsplits = ZERO_HASH;
    let hash_shielded_spends = ZERO_HASH;
    let hash_shielded_outputs = ZERO_HASH;

    let mut preimage = Vec::with_capacity(256);
    preimage.extend_from_slice(&tx.header().to_le_bytes());
    preimage.extend_from_slice(&tx.version_group_id.to_le_bytes());
    preimage.extend_from_slice(&hash_prevouts);
    preimage.extend_from_slice(&hash_sequence);
    preimage.extend_from_slice(&hash_outputs);
    preimage.extend_from_slice(&hash_joinsplits);

    if tx.is_sapling_compatible() {
        preimage.extend_from_slice(&hash_shielded_spends);
        preimage.extend_from_slice(&hash_shielded_outputs);
    }

    preimage.extend_from_slice(&tx.lock_time.to_le_bytes());
    preimage.extend_from_slice(&tx.expiry_height.to_le_bytes());

    if tx.is_sapling_compatible() {
        preimage.extend_from_slice(&tx.value_balance.to_le_bytes());
    }

    preimage.extend_from_slice(&hash_type.to_le_bytes());

    let input = &tx.inputs[in_index];
    preimage.extend_from_slice(&input.prevout_txid_le);
    preimage.extend_from_slice(&input.prevout_vout.to_le_bytes());
    write_var_slice(&mut preimage, prev_out_script);
    preimage.extend_from_slice(&value.to_le_bytes());
    preimage.extend_from_slice(&input.sequence.to_le_bytes());

    let mut personalization = [0u8; 16];
    personalization[..12].copy_from_slice(b"ZcashSigHash");
    personalization[12..].copy_from_slice(&consensus_branch_id.to_le_bytes());

    Ok(blake2b_hash_32(&preimage, &personalization))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::channels::vrpc::identity::verus_tx::codec::decode_hex;

    #[test]
    fn matches_reference_sighash_vector_for_v4_p2pkh_input() {
        let tx_hex = "0400008085202f8901ffeeddccbbaa99887766554433221100ffeeddccbbaa998877665544332211000300000000ffffffff0150c30000000000001976a914111111111111111111111111111111111111111188ac00000000000000000000000000000000000000";
        let tx = decode_hex(tx_hex).expect("decode tx");
        let prev_script =
            hex::decode("76a914222222222222222222222222222222222222222288ac").expect("prev script");

        let hash = signature_hash(&tx, 0, &prev_script, 70_000, SIGHASH_ALL).expect("sighash");
        assert_eq!(
            hex::encode(hash),
            "0b5b881fad786e6a87b5c1a20e046d34f1e561bd1c00da4f34f03c269e8104cd"
        );
    }
}
