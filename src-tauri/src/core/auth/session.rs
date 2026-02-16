//
// Session management with timeout and zeroization
// Security: Manages unlocked session state, derived keys are zeroized on lock/timeout
// Last Updated: get_addresses now returns (vrsc, eth, btc) for Bitcoin P2PKH parity

use crate::core::auth::stronghold_store::StrongholdStore;
use crate::core::crypto::{derive_keys_from_material, Network};
use crate::types::errors::WalletError;
use crate::types::wallet::{DerivedKeys, WalletNetwork, WalletSecretKind};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use zeroize::Zeroizing;

pub struct SessionManager {
    is_unlocked: bool,
    active_account_id: Option<String>,
    unlocked_at: Option<Instant>,
    timeout_duration: Duration,
    derived_keys: HashMap<String, DerivedKeys>, // account_id -> keys
    active_network: Option<WalletNetwork>,
    stronghold_password_hash: Option<Zeroizing<Vec<u8>>>,
    stronghold_store: StrongholdStore,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(stronghold_store: StrongholdStore) -> Self {
        Self {
            is_unlocked: false,
            active_account_id: None,
            unlocked_at: None,
            // Parity default: no auto-expiry. Session ends on explicit lock.
            timeout_duration: Duration::from_secs(0),
            derived_keys: HashMap::new(),
            active_network: None,
            stronghold_password_hash: None,
            stronghold_store,
        }
    }

    /// Unlock wallet session by loading seed and deriving keys
    ///
    /// Security: Derives keys in memory, stores in session-scoped HashMap
    /// Keys will be zeroized when HashMap is dropped (via ZeroizeOnDrop)
    pub async fn unlock(
        &mut self,
        account_id: String,
        password: String,
        wallet_network: WalletNetwork,
        wallet_secret_kind: WalletSecretKind,
        app_handle: &AppHandle,
    ) -> Result<(), WalletError> {
        println!("[SESSION] Unlock requested for account: {}", account_id);

        // Load seed from Stronghold
        let seed = self
            .stronghold_store
            .load_seed(&account_id, &password, app_handle)
            .await?;

        let derivation_network = match wallet_network {
            WalletNetwork::Mainnet => Network::Mainnet,
            WalletNetwork::Testnet => Network::Testnet,
        };

        // Derive keys from the imported secret material type.
        let keys = derive_keys_from_material(&seed, wallet_secret_kind, derivation_network)
            .map_err(|_e| {
                println!("[SESSION] Key derivation failed");
                WalletError::OperationFailed
            })?;

        // Store in session (keys will be zeroized on drop via ZeroizeOnDrop)
        self.derived_keys.insert(account_id.clone(), keys);
        self.active_account_id = Some(account_id);
        self.active_network = Some(wallet_network);
        self.stronghold_password_hash =
            Some(Zeroizing::new(StrongholdStore::hash_password(&password)));
        self.is_unlocked = true;
        self.unlocked_at = Some(Instant::now());

        println!("[SESSION] Unlock successful");
        Ok(())
    }

    /// Lock wallet session and zeroize all derived keys
    ///
    /// Security: Clears HashMap containing DerivedKeys, triggering ZeroizeOnDrop
    pub fn lock(&mut self) {
        println!("[SESSION] Locking wallet");

        // Clear derived keys (ZeroizeOnDrop triggers here)
        self.derived_keys.clear();
        self.active_account_id = None;
        self.active_network = None;
        self.stronghold_password_hash = None;
        self.is_unlocked = false;
        self.unlocked_at = None;

        println!("[SESSION] Wallet locked, keys zeroized");
    }

    /// Check if session has expired
    pub fn is_expired(&self) -> bool {
        if !self.is_unlocked {
            return true;
        }

        // Disabled timeout by default. Keep configurable for future optional auto-lock.
        if self.timeout_duration.as_secs() == 0 {
            return false;
        }

        if let Some(unlocked_at) = self.unlocked_at {
            unlocked_at.elapsed() > self.timeout_duration
        } else {
            true
        }
    }

    /// Get derived addresses for active account
    ///
    /// Security: Returns addresses only, never private keys
    pub fn get_addresses(&self) -> Result<(String, String, String), WalletError> {
        if !self.is_unlocked || self.is_expired() {
            return Err(WalletError::WalletLocked);
        }

        let account_id = self
            .active_account_id
            .as_ref()
            .ok_or(WalletError::WalletLocked)?;

        let keys = self
            .derived_keys
            .get(account_id)
            .ok_or(WalletError::WalletLocked)?;

        Ok((
            keys.address.clone(),
            keys.eth_address.clone(),
            keys.btc_address.clone(),
        ))
    }

    /// Check if wallet is currently unlocked and not expired
    pub fn is_unlocked(&self) -> bool {
        self.is_unlocked && !self.is_expired()
    }

    /// Get the active account ID
    pub fn active_account_id(&self) -> Option<&String> {
        self.active_account_id.as_ref()
    }

    /// Returns the selected wallet network for the active session.
    pub fn active_network(&self) -> Option<WalletNetwork> {
        self.active_network
    }

    /// Set session timeout duration
    pub fn set_timeout(&mut self, duration: Duration) {
        self.timeout_duration = duration;
    }

    /// Get reference to StrongholdStore (for use in commands)
    pub fn stronghold_store(&self) -> &StrongholdStore {
        &self.stronghold_store
    }

    /// Returns a copy of the current unlocked Stronghold password hash bytes for storage commands.
    /// Security: only available while unlocked; caller should zeroize after use.
    pub fn stronghold_password_hash_for_storage(&self) -> Result<Zeroizing<Vec<u8>>, WalletError> {
        if !self.is_unlocked || self.is_expired() {
            return Err(WalletError::WalletLocked);
        }

        let hash = self
            .stronghold_password_hash
            .as_ref()
            .ok_or(WalletError::WalletLocked)?;
        Ok(Zeroizing::new(hash.to_vec()))
    }

    /// Returns the WIF for the active account for signing only (VRPC/BTC send flow).
    /// Security: Must only be used in the send flow; never log or expose this value.
    pub fn get_wif_for_signing(&self) -> Result<String, WalletError> {
        if !self.is_unlocked || self.is_expired() {
            return Err(WalletError::WalletLocked);
        }
        let account_id = self
            .active_account_id
            .as_ref()
            .ok_or(WalletError::WalletLocked)?;
        let keys = self
            .derived_keys
            .get(account_id)
            .ok_or(WalletError::WalletLocked)?;
        Ok(keys.wif.clone())
    }

    /// Returns the Ethereum private key for signing ETH/ERC20 transactions.
    /// Security: Must remain backend-only and never cross the command boundary.
    pub fn get_eth_private_key_for_signing(&self) -> Result<String, WalletError> {
        if !self.is_unlocked || self.is_expired() {
            return Err(WalletError::WalletLocked);
        }
        let account_id = self
            .active_account_id
            .as_ref()
            .ok_or(WalletError::WalletLocked)?;
        let keys = self
            .derived_keys
            .get(account_id)
            .ok_or(WalletError::WalletLocked)?;
        Ok(keys.eth_private_key.clone())
    }
}
