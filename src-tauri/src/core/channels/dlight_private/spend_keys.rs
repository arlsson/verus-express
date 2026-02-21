use zcash_client_backend::encoding::{decode_extended_spending_key, encode_payment_address};
use zcash_client_backend::keys::{UnifiedFullViewingKey, UnifiedSpendingKey};
use zcash_client_backend::scanning::ScanningKeys;
use zcash_protocol::consensus::{MainNetwork, TestNetwork};
use zcash_protocol::constants::{mainnet, testnet};
use zip32::{AccountId, Scope};

use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

#[derive(Debug, Clone)]
pub struct DlightSpendKeyMaterial {
    sapling_extsk: sapling::zip32::ExtendedSpendingKey,
    sapling_dfvk: sapling::zip32::DiversifiableFullViewingKey,
}

impl DlightSpendKeyMaterial {
    pub fn from_seed_material(
        seed_material: &str,
        network: WalletNetwork,
        expected_scope_address: &str,
    ) -> Result<Self, WalletError> {
        let normalized = seed_material.trim();
        if normalized.is_empty() {
            return Err(WalletError::InvalidSeedPhrase);
        }

        let sapling_extsk = if normalized.starts_with("secret-extended-key-") {
            let hrp = sapling_extsk_hrp(network);
            decode_extended_spending_key(hrp, normalized)
                .map_err(|_| WalletError::InvalidImportText)?
        } else {
            let mnemonic =
                bip39::Mnemonic::parse(normalized).map_err(|_| WalletError::InvalidSeedPhrase)?;
            let seed = mnemonic.to_seed_normalized("");
            let usk = match network {
                WalletNetwork::Mainnet => {
                    UnifiedSpendingKey::from_seed(&MainNetwork, &seed, AccountId::ZERO)
                        .map_err(|_| WalletError::InvalidSeedPhrase)?
                }
                WalletNetwork::Testnet => {
                    UnifiedSpendingKey::from_seed(&TestNetwork, &seed, AccountId::ZERO)
                        .map_err(|_| WalletError::InvalidSeedPhrase)?
                }
            };
            usk.sapling().clone()
        };

        let sapling_dfvk = sapling_extsk.to_diversifiable_full_viewing_key();
        let derived_scope = scope_address_for_dfvk(&sapling_dfvk, network)?;
        if derived_scope != expected_scope_address.trim() {
            return Err(WalletError::InvalidPreflight);
        }

        Ok(Self {
            sapling_extsk,
            sapling_dfvk,
        })
    }

    pub fn to_scanning_keys(&self) -> Result<ScanningKeys<u32, (u32, Scope)>, WalletError> {
        #[allow(deprecated)]
        let ext_fvk = self.sapling_extsk.to_extended_full_viewing_key();
        let ufvk = UnifiedFullViewingKey::from_sapling_extended_full_viewing_key(ext_fvk)
            .map_err(|_| WalletError::OperationFailed)?;
        Ok(ScanningKeys::from_account_ufvks(vec![(0u32, ufvk)]))
    }

    pub fn sapling_extsk(&self) -> &sapling::zip32::ExtendedSpendingKey {
        &self.sapling_extsk
    }

    pub fn sapling_extsks_for_builder(&self) -> [sapling::zip32::ExtendedSpendingKey; 2] {
        [
            self.sapling_extsk.clone(),
            self.sapling_extsk.derive_internal(),
        ]
    }

    pub fn sapling_fvk_for_scope(&self, scope: Scope) -> sapling::keys::FullViewingKey {
        match scope {
            Scope::External => self.sapling_dfvk.fvk().clone(),
            Scope::Internal => self.sapling_dfvk.to_internal_fvk(),
        }
    }

    pub fn sapling_ovk_for_scope(&self, scope: Scope) -> sapling::keys::OutgoingViewingKey {
        self.sapling_dfvk.to_ovk(scope)
    }

    pub fn scope_address(&self, network: WalletNetwork) -> Result<String, WalletError> {
        scope_address_for_dfvk(&self.sapling_dfvk, network)
    }

    pub fn external_payment_address(&self) -> sapling::PaymentAddress {
        self.sapling_dfvk.default_address().1
    }

    pub fn change_payment_address(&self) -> sapling::PaymentAddress {
        self.sapling_dfvk.change_address().1
    }
}

fn sapling_extsk_hrp(network: WalletNetwork) -> &'static str {
    match network {
        WalletNetwork::Mainnet => mainnet::HRP_SAPLING_EXTENDED_SPENDING_KEY,
        WalletNetwork::Testnet => testnet::HRP_SAPLING_EXTENDED_SPENDING_KEY,
    }
}

fn sapling_payment_address_hrp(network: WalletNetwork) -> &'static str {
    let _ = network;
    // Parity policy: use zs-addresses on both mainnet and testnet.
    mainnet::HRP_SAPLING_PAYMENT_ADDRESS
}

fn scope_address_for_dfvk(
    dfvk: &sapling::zip32::DiversifiableFullViewingKey,
    network: WalletNetwork,
) -> Result<String, WalletError> {
    let (_, addr) = dfvk.default_address();
    Ok(encode_payment_address(
        sapling_payment_address_hrp(network),
        &addr,
    ))
}
