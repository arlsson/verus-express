use serde::{Deserialize, Serialize};
use zcash_client_backend::encoding::decode_payment_address;
use zcash_protocol::constants::mainnet;

use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DlightDestinationKind {
    Shielded,
    Transparent,
}

pub fn classify_dlight_destination(
    address: &str,
    network: WalletNetwork,
) -> Result<DlightDestinationKind, WalletError> {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return Err(WalletError::InvalidAddress);
    }

    if is_shielded_sapling_address(trimmed, network) {
        return Ok(DlightDestinationKind::Shielded);
    }

    if is_transparent_verus_address(trimmed) {
        return Ok(DlightDestinationKind::Transparent);
    }

    Err(WalletError::InvalidAddress)
}

pub fn is_transparent_verus_address(address: &str) -> bool {
    let trimmed = address.trim();
    let length = trimmed.len();
    if !(25..=61).contains(&length) {
        return false;
    }

    let mut chars = trimmed.chars();
    let Some(prefix) = chars.next() else {
        return false;
    };
    if prefix != 'R' && prefix != 'i' {
        return false;
    }

    chars.all(is_base58_char)
}

pub fn is_shielded_sapling_address(address: &str, network: WalletNetwork) -> bool {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return false;
    }

    let normalized = trimmed.to_ascii_lowercase();
    let hrp = sapling_payment_address_hrp(network);

    decode_payment_address(hrp, normalized.as_str()).is_ok()
}

fn sapling_payment_address_hrp(_network: WalletNetwork) -> &'static str {
    // Parity policy: use zs-addresses on both mainnet and testnet.
    mainnet::HRP_SAPLING_PAYMENT_ADDRESS
}

fn is_base58_char(ch: char) -> bool {
    matches!(
        ch,
        '1'..='9'
            | 'A'..='H'
            | 'J'..='N'
            | 'P'..='Z'
            | 'a'..='k'
            | 'm'..='z'
    )
}

#[cfg(test)]
mod tests {
    use super::{classify_dlight_destination, DlightDestinationKind};
    use crate::types::wallet::WalletNetwork;
    use crate::types::WalletError;

    #[test]
    fn classify_supports_transparent_verus_addresses() {
        let r_address = "RAutMoGh771ECTDbTq2qwwZo7MF5Tov3ka";
        let i_address = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";

        assert_eq!(
            classify_dlight_destination(r_address, WalletNetwork::Mainnet)
                .expect("valid R address"),
            DlightDestinationKind::Transparent
        );
        assert_eq!(
            classify_dlight_destination(i_address, WalletNetwork::Mainnet)
                .expect("valid i address"),
            DlightDestinationKind::Transparent
        );
    }

    #[test]
    fn classify_supports_zs_on_testnet() {
        let testnet =
            "zs1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75c8v35z";
        assert_eq!(
            classify_dlight_destination(testnet, WalletNetwork::Testnet)
                .expect("valid testnet sapling address"),
            DlightDestinationKind::Shielded
        );
    }

    #[test]
    fn classify_rejects_legacy_testnet_sapling_prefix() {
        let legacy_testnet = "ztestsapling1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75ss7jnk";
        assert!(matches!(
            classify_dlight_destination(legacy_testnet, WalletNetwork::Testnet),
            Err(WalletError::InvalidAddress)
        ));
    }

    #[test]
    fn classify_rejects_non_supported_formats() {
        assert!(matches!(
            classify_dlight_destination("alice@", WalletNetwork::Mainnet),
            Err(WalletError::InvalidAddress)
        ));
        assert!(matches!(
            classify_dlight_destination("0x1234", WalletNetwork::Mainnet),
            Err(WalletError::InvalidAddress)
        ));
    }
}
