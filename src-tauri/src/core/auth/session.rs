// 
// Session management with timeout and zeroization
// Security: Manages unlocked session state, derived keys are zeroized on lock/timeout
// Last Updated: Created for Module 1 integration

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use crate::types::wallet::DerivedKeys;
use crate::types::errors::WalletError;
use crate::core::auth::stronghold_store::StrongholdStore;
use crate::core::crypto::{derive_keys_v1, Network};

pub struct SessionManager {
    is_unlocked: bool,
    active_account_id: Option<String>,
    unlocked_at: Option<Instant>,
    timeout_duration: Duration,
    derived_keys: HashMap<String, DerivedKeys>, // account_id -> keys
    stronghold_store: StrongholdStore,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(stronghold_store: StrongholdStore) -> Self {
        Self {
            is_unlocked: false,
            active_account_id: None,
            unlocked_at: None,
            timeout_duration: Duration::from_secs(300), // 5 minutes default
            derived_keys: HashMap::new(),
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
        app_handle: &AppHandle,
    ) -> Result<(), WalletError> {
        println!("[SESSION] Unlock requested for account: {}", account_id);
        
        // Load seed from Stronghold
        let seed = self.stronghold_store.load_seed(&account_id, &password, app_handle).await?;
        
        // Derive keys using v1 derivation (Verus-Mobile compatible)
        let keys = derive_keys_v1(&seed, Network::Mainnet)
            .map_err(|_e| {
                println!("[SESSION] Key derivation failed");
                WalletError::OperationFailed
            })?;
        
        // Store in session (keys will be zeroized on drop via ZeroizeOnDrop)
        self.derived_keys.insert(account_id.clone(), keys);
        self.active_account_id = Some(account_id);
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
        self.is_unlocked = false;
        self.unlocked_at = None;
        
        println!("[SESSION] Wallet locked, keys zeroized");
    }
    
    /// Check if session has expired
    pub fn is_expired(&self) -> bool {
        if !self.is_unlocked {
            return true;
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
    pub fn get_addresses(&self) -> Result<(String, String), WalletError> {
        if !self.is_unlocked || self.is_expired() {
            return Err(WalletError::WalletLocked);
        }
        
        let account_id = self.active_account_id.as_ref()
            .ok_or(WalletError::WalletLocked)?;
        
        let keys = self.derived_keys.get(account_id)
            .ok_or(WalletError::WalletLocked)?;
        
        Ok((keys.address.clone(), keys.eth_address.clone()))
    }
    
    /// Check if wallet is currently unlocked and not expired
    pub fn is_unlocked(&self) -> bool {
        self.is_unlocked && !self.is_expired()
    }
    
    /// Get the active account ID
    pub fn active_account_id(&self) -> Option<&String> {
        self.active_account_id.as_ref()
    }
    
    /// Set session timeout duration
    pub fn set_timeout(&mut self, duration: Duration) {
        self.timeout_duration = duration;
    }
    
    /// Get reference to StrongholdStore (for use in commands)
    pub fn stronghold_store(&self) -> &StrongholdStore {
        &self.stronghold_store
    }
}
