use serde_json::Value;
use zcash_client_backend::encoding::{decode_payment_address, encode_payment_address};
use zcash_protocol::constants::mainnet;

use crate::core::channels::vrpc::VrpcProvider;
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

use super::destination::DlightDestinationKind;

const VERUS_R_ADDRESS_VERSION: u8 = 60;

#[derive(Debug, Clone)]
pub struct ResolvedDlightRecipient {
    pub display_to_address: String,
    pub delivery_to_address: String,
    pub destination_kind: DlightDestinationKind,
    pub shielded: Option<sapling::PaymentAddress>,
    pub transparent: Option<zcash_transparent::address::TransparentAddress>,
}

impl ResolvedDlightRecipient {
    pub fn is_shielded(&self) -> bool {
        matches!(self.destination_kind, DlightDestinationKind::Shielded)
    }
}

pub async fn resolve_dlight_recipient(
    input: &str,
    network: WalletNetwork,
    vrpc_provider: &VrpcProvider,
) -> Result<ResolvedDlightRecipient, WalletError> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.ends_with('@') {
        return Err(WalletError::InvalidAddress);
    }

    if let Some(payment_address) = decode_sapling_address(trimmed, network) {
        let delivery = encode_sapling_address(&payment_address, network);
        return Ok(ResolvedDlightRecipient {
            display_to_address: trimmed.to_string(),
            delivery_to_address: delivery,
            destination_kind: DlightDestinationKind::Shielded,
            shielded: Some(payment_address),
            transparent: None,
        });
    }

    if is_identity_address(trimmed) {
        let resolved_r = resolve_identity_primary_r_address(vrpc_provider, trimmed).await?;
        let transparent = decode_r_address(&resolved_r)?;
        return Ok(ResolvedDlightRecipient {
            display_to_address: trimmed.to_string(),
            delivery_to_address: resolved_r,
            destination_kind: DlightDestinationKind::Transparent,
            shielded: None,
            transparent: Some(transparent),
        });
    }

    let transparent = decode_r_address(trimmed)?;
    Ok(ResolvedDlightRecipient {
        display_to_address: trimmed.to_string(),
        delivery_to_address: trimmed.to_string(),
        destination_kind: DlightDestinationKind::Transparent,
        shielded: None,
        transparent: Some(transparent),
    })
}

fn decode_sapling_address(input: &str, network: WalletNetwork) -> Option<sapling::PaymentAddress> {
    let normalized = input.trim().to_ascii_lowercase();
    let hrp = sapling_payment_address_hrp(network);
    decode_payment_address(hrp, normalized.as_str()).ok()
}

fn encode_sapling_address(address: &sapling::PaymentAddress, network: WalletNetwork) -> String {
    let hrp = sapling_payment_address_hrp(network);
    encode_payment_address(hrp, address)
}

fn sapling_payment_address_hrp(_network: WalletNetwork) -> &'static str {
    // Parity policy: use zs-addresses on both mainnet and testnet.
    mainnet::HRP_SAPLING_PAYMENT_ADDRESS
}

fn is_identity_address(input: &str) -> bool {
    let trimmed = input.trim();
    if !trimmed.starts_with('i') || trimmed.len() < 20 {
        return false;
    }
    trimmed.chars().all(is_base58_char)
}

fn is_base58_char(ch: char) -> bool {
    matches!(
        ch,
        '1'..='9' | 'A'..='H' | 'J'..='N' | 'P'..='Z' | 'a'..='k' | 'm'..='z'
    )
}

pub(crate) fn decode_r_address(
    input: &str,
) -> Result<zcash_transparent::address::TransparentAddress, WalletError> {
    let trimmed = input.trim();
    if !trimmed.starts_with('R') {
        return Err(WalletError::InvalidAddress);
    }

    let decoded = bs58::decode(trimmed)
        .with_check(None)
        .into_vec()
        .map_err(|_| WalletError::InvalidAddress)?;

    if decoded.len() != 21 || decoded[0] != VERUS_R_ADDRESS_VERSION {
        return Err(WalletError::InvalidAddress);
    }

    let mut hash = [0u8; 20];
    hash.copy_from_slice(&decoded[1..]);
    Ok(zcash_transparent::address::TransparentAddress::PublicKeyHash(hash))
}

async fn resolve_identity_primary_r_address(
    provider: &VrpcProvider,
    identity: &str,
) -> Result<String, WalletError> {
    let raw: Value = provider.getidentity(identity).await?;
    let identity_obj = raw.get("identity").unwrap_or(&raw);

    if let Some(primary_addresses) = identity_obj
        .get("primaryaddresses")
        .and_then(|value| value.as_array())
    {
        for candidate in primary_addresses.iter().filter_map(|value| value.as_str()) {
            let trimmed = candidate.trim();
            if decode_r_address(trimmed).is_ok() {
                return Ok(trimmed.to_string());
            }
        }
    }

    if let Some(single) = parse_string(identity_obj.get("primaryaddress")) {
        if decode_r_address(&single).is_ok() {
            return Ok(single);
        }
    }

    Err(WalletError::InvalidAddress)
}

fn parse_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::{decode_r_address, decode_sapling_address};
    use crate::types::wallet::WalletNetwork;
    use crate::types::WalletError;

    #[test]
    fn decode_r_address_accepts_valid_verus_pkh() {
        let parsed = decode_r_address("RAutMoGh771ECTDbTq2qwwZo7MF5Tov3ka");
        assert!(parsed.is_ok());
    }

    #[test]
    fn decode_r_address_rejects_identity_addr() {
        assert!(matches!(
            decode_r_address("i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV"),
            Err(WalletError::InvalidAddress)
        ));
    }

    #[test]
    fn decode_sapling_address_accepts_zs_on_testnet() {
        assert!(decode_sapling_address(
            "zs1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75c8v35z",
            WalletNetwork::Testnet
        )
        .is_some());
    }

    #[test]
    fn decode_sapling_address_rejects_legacy_testnet_prefix() {
        assert!(
            decode_sapling_address(
                "ztestsapling1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75ss7jnk",
                WalletNetwork::Testnet
            )
            .is_none()
        );
    }
}
