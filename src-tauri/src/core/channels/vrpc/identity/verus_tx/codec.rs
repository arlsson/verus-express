//
// Verus transaction codec for Overwinter/Sapling transparent-only subset.

use std::io::{Cursor, Read};

use crate::core::channels::vrpc::identity::verus_tx::model::{
    VerusTx, VerusTxIn, VerusTxOut, OVERWINTER_VERSION_GROUP_ID, SAPLING_VERSION_GROUP_ID,
};
use crate::types::WalletError;

fn read_exact(cursor: &mut Cursor<&[u8]>, len: usize) -> Result<Vec<u8>, WalletError> {
    let mut buf = vec![0u8; len];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| WalletError::IdentityBuildFailed)?;
    Ok(buf)
}

fn read_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, WalletError> {
    Ok(read_exact(cursor, 1)?[0])
}

fn read_u32_le(cursor: &mut Cursor<&[u8]>) -> Result<u32, WalletError> {
    let mut b = [0u8; 4];
    b.copy_from_slice(&read_exact(cursor, 4)?);
    Ok(u32::from_le_bytes(b))
}

fn read_u64_le(cursor: &mut Cursor<&[u8]>) -> Result<u64, WalletError> {
    let mut b = [0u8; 8];
    b.copy_from_slice(&read_exact(cursor, 8)?);
    Ok(u64::from_le_bytes(b))
}

fn read_i64_le(cursor: &mut Cursor<&[u8]>) -> Result<i64, WalletError> {
    let mut b = [0u8; 8];
    b.copy_from_slice(&read_exact(cursor, 8)?);
    Ok(i64::from_le_bytes(b))
}

fn read_compact_size(cursor: &mut Cursor<&[u8]>) -> Result<u64, WalletError> {
    let first = read_u8(cursor)?;
    match first {
        0x00..=0xfc => Ok(first as u64),
        0xfd => {
            let mut b = [0u8; 2];
            b.copy_from_slice(&read_exact(cursor, 2)?);
            Ok(u16::from_le_bytes(b) as u64)
        }
        0xfe => {
            let mut b = [0u8; 4];
            b.copy_from_slice(&read_exact(cursor, 4)?);
            Ok(u32::from_le_bytes(b) as u64)
        }
        0xff => {
            let mut b = [0u8; 8];
            b.copy_from_slice(&read_exact(cursor, 8)?);
            Ok(u64::from_le_bytes(b))
        }
    }
}

fn compact_size_len(value: u64) -> usize {
    match value {
        0x00..=0xfc => 1,
        0xfd..=0xffff => 3,
        0x1_0000..=0xffff_ffff => 5,
        _ => 9,
    }
}

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

fn read_var_slice(cursor: &mut Cursor<&[u8]>) -> Result<Vec<u8>, WalletError> {
    let len = read_compact_size(cursor)? as usize;
    read_exact(cursor, len)
}

fn write_var_slice(out: &mut Vec<u8>, bytes: &[u8]) {
    write_compact_size(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

pub fn decode_hex(raw_hex: &str) -> Result<VerusTx, WalletError> {
    let raw = hex::decode(raw_hex.trim_start_matches("0x"))
        .or_else(|_| hex::decode(raw_hex))
        .map_err(|_| WalletError::IdentityBuildFailed)?;
    decode_bytes(&raw)
}

pub fn decode_bytes(raw: &[u8]) -> Result<VerusTx, WalletError> {
    let mut cursor = Cursor::new(raw);

    let header = read_u32_le(&mut cursor)?;
    let overwintered = (header >> 31) == 1;
    let version = header & 0x7fff_ffff;
    if !overwintered || !(version == 3 || version == 4) {
        return Err(WalletError::IdentityBuildFailed);
    }

    let version_group_id = read_u32_le(&mut cursor)?;
    if version == 3 && version_group_id != OVERWINTER_VERSION_GROUP_ID {
        return Err(WalletError::IdentityBuildFailed);
    }
    if version >= 4 && version_group_id != SAPLING_VERSION_GROUP_ID {
        return Err(WalletError::IdentityBuildFailed);
    }

    let input_len = read_compact_size(&mut cursor)? as usize;
    let mut inputs = Vec::with_capacity(input_len);
    for _ in 0..input_len {
        let mut txid = [0u8; 32];
        txid.copy_from_slice(&read_exact(&mut cursor, 32)?);
        let vout = read_u32_le(&mut cursor)?;
        let script_sig = read_var_slice(&mut cursor)?;
        let sequence = read_u32_le(&mut cursor)?;
        inputs.push(VerusTxIn {
            prevout_txid_le: txid,
            prevout_vout: vout,
            script_sig,
            sequence,
        });
    }

    let output_len = read_compact_size(&mut cursor)? as usize;
    let mut outputs = Vec::with_capacity(output_len);
    for _ in 0..output_len {
        let value = read_u64_le(&mut cursor)?;
        let script_pub_key = read_var_slice(&mut cursor)?;
        outputs.push(VerusTxOut {
            value,
            script_pub_key,
        });
    }

    let lock_time = read_u32_le(&mut cursor)?;
    let expiry_height = read_u32_le(&mut cursor)?;

    let mut value_balance = 0i64;
    if version >= 4 {
        value_balance = read_i64_le(&mut cursor)?;
        if value_balance != 0 {
            return Err(WalletError::IdentityBuildFailed);
        }
        let shielded_spends = read_compact_size(&mut cursor)?;
        if shielded_spends != 0 {
            return Err(WalletError::IdentityBuildFailed);
        }
        let shielded_outputs = read_compact_size(&mut cursor)?;
        if shielded_outputs != 0 {
            return Err(WalletError::IdentityBuildFailed);
        }
    }

    let joinsplit_len = read_compact_size(&mut cursor)?;
    if joinsplit_len != 0 {
        return Err(WalletError::IdentityBuildFailed);
    }

    if cursor.position() != raw.len() as u64 {
        return Err(WalletError::IdentityBuildFailed);
    }

    Ok(VerusTx {
        version,
        overwintered,
        version_group_id,
        inputs,
        outputs,
        lock_time,
        expiry_height,
        value_balance,
    })
}

pub fn encode_hex(tx: &VerusTx) -> Result<String, WalletError> {
    Ok(hex::encode(encode_bytes(tx)?))
}

pub fn encode_bytes(tx: &VerusTx) -> Result<Vec<u8>, WalletError> {
    if !tx.overwintered || !(tx.version == 3 || tx.version == 4) {
        return Err(WalletError::IdentityBuildFailed);
    }
    if tx.version == 3 && tx.version_group_id != OVERWINTER_VERSION_GROUP_ID {
        return Err(WalletError::IdentityBuildFailed);
    }
    if tx.version >= 4 && tx.version_group_id != SAPLING_VERSION_GROUP_ID {
        return Err(WalletError::IdentityBuildFailed);
    }
    if tx.value_balance != 0 {
        return Err(WalletError::IdentityBuildFailed);
    }

    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(&tx.header().to_le_bytes());
    out.extend_from_slice(&tx.version_group_id.to_le_bytes());

    write_compact_size(&mut out, tx.inputs.len() as u64);
    for input in &tx.inputs {
        out.extend_from_slice(&input.prevout_txid_le);
        out.extend_from_slice(&input.prevout_vout.to_le_bytes());
        write_var_slice(&mut out, &input.script_sig);
        out.extend_from_slice(&input.sequence.to_le_bytes());
    }

    write_compact_size(&mut out, tx.outputs.len() as u64);
    for output in &tx.outputs {
        out.extend_from_slice(&output.value.to_le_bytes());
        write_var_slice(&mut out, &output.script_pub_key);
    }

    out.extend_from_slice(&tx.lock_time.to_le_bytes());
    out.extend_from_slice(&tx.expiry_height.to_le_bytes());

    if tx.version >= 4 {
        out.extend_from_slice(&tx.value_balance.to_le_bytes());
        write_compact_size(&mut out, 0);
        write_compact_size(&mut out, 0);
    }

    write_compact_size(&mut out, 0);
    Ok(out)
}

pub fn encode_output_bytes(output: &VerusTxOut) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + compact_size_len(output.script_pub_key.len() as u64));
    out.extend_from_slice(&output.value.to_le_bytes());
    write_var_slice(&mut out, &output.script_pub_key);
    out
}

pub fn write_compact_size_to_vec(out: &mut Vec<u8>, value: u64) {
    write_compact_size(out, value);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_V4_HEX: &str = "0400008085202f8901ffeeddccbbaa99887766554433221100ffeeddccbbaa998877665544332211000300000000ffffffff0150c30000000000001976a914111111111111111111111111111111111111111188ac00000000000000000000000000000000000000";

    #[test]
    fn decodes_and_encodes_known_v4_sample() {
        let tx = decode_hex(SAMPLE_V4_HEX).expect("decode");
        assert_eq!(tx.version, 4);
        assert!(tx.overwintered);
        assert_eq!(tx.version_group_id, SAPLING_VERSION_GROUP_ID);
        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.outputs.len(), 1);
        assert_eq!(tx.outputs[0].value, 50_000);
        let reencoded = encode_hex(&tx).expect("encode");
        assert_eq!(reencoded, SAMPLE_V4_HEX);
    }

    #[test]
    fn rejects_non_zero_value_balance() {
        let mut raw = hex::decode(SAMPLE_V4_HEX).expect("sample hex");
        let pos = raw.len() - 11;
        raw[pos] = 1;
        let parsed = decode_bytes(&raw);
        assert!(parsed.is_err());
    }
}
