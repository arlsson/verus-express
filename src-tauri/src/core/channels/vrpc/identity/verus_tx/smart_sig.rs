//
// Verus SmartTransactionSignatures serialization.

use crate::core::channels::vrpc::identity::verus_tx::codec::write_compact_size_to_vec;
use crate::types::WalletError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartTransactionSignature {
    pub sig_type: u8,
    pub pub_key_data: Vec<u8>,
    pub one_signature: Vec<u8>,
}

impl SmartTransactionSignature {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out =
            Vec::with_capacity(1 + 9 + self.pub_key_data.len() + 9 + self.one_signature.len());
        out.push(self.sig_type);
        write_compact_size_to_vec(&mut out, self.pub_key_data.len() as u64);
        out.extend_from_slice(&self.pub_key_data);
        write_compact_size_to_vec(&mut out, self.one_signature.len() as u64);
        out.extend_from_slice(&self.one_signature);
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartTransactionSignatures {
    pub version: u8,
    pub sig_hash_type: u8,
    pub signatures: Vec<SmartTransactionSignature>,
}

impl SmartTransactionSignatures {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(2 + 9 + self.signatures.len() * 100);
        out.push(self.version);
        out.push(self.sig_hash_type);
        write_compact_size_to_vec(&mut out, self.signatures.len() as u64);
        for sig in &self.signatures {
            out.extend_from_slice(&sig.to_bytes());
        }
        out
    }
}

pub fn build_single_signature_chunk(
    pub_key_data: &[u8],
    compact_signature_64: &[u8],
    sig_hash_type: u8,
) -> Result<Vec<u8>, WalletError> {
    if pub_key_data.len() != 33 || compact_signature_64.len() != 64 {
        return Err(WalletError::IdentitySignFailed);
    }
    let sigs = SmartTransactionSignatures {
        version: 1,
        sig_hash_type,
        signatures: vec![SmartTransactionSignature {
            sig_type: 1,
            pub_key_data: pub_key_data.to_vec(),
            one_signature: compact_signature_64.to_vec(),
        }],
    };
    Ok(sigs.to_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_single_signature_reference_vector() {
        let pubkey =
            hex::decode("025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486")
                .expect("pubkey");
        let compact_sig = hex::decode(
            "2142a1545964b8ca2663a957d37e1d7e33fb3b1a4d214b0c657a04ee3aacd81237bf657565beddf2fab20829de48258bc96b88aaf6f0ceef7832c265e2da2852",
        )
        .expect("compact signature");

        let chunk = build_single_signature_chunk(&pubkey, &compact_sig, 1).expect("chunk");
        assert_eq!(
            hex::encode(chunk),
            "0101010121025f7117a78150fe2ef97db7cfc83bd57b2e2c0d0dd25eaf467a4a1c2a45ce1486402142a1545964b8ca2663a957d37e1d7e33fb3b1a4d214b0c657a04ee3aacd81237bf657565beddf2fab20829de48258bc96b88aaf6f0ceef7832c265e2da2852"
        );
    }
}
