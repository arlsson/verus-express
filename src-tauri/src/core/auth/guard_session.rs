//
// Temporary in-memory guard session manager for signed-out revoke/recover flows.
// Security: no persistence; derived keys are zeroized when sessions are removed.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::types::wallet::{DerivedKeys, WalletNetwork};
use crate::types::WalletError;

const DEFAULT_GUARD_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Clone)]
pub struct GuardSession {
    pub id: String,
    pub keys: DerivedKeys,
    pub network: WalletNetwork,
    pub created_at: Instant,
    pub expires_at: Instant,
}

pub struct GuardSessionManager {
    sessions: HashMap<String, GuardSession>,
    ttl: Duration,
}

impl GuardSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            ttl: DEFAULT_GUARD_TTL,
        }
    }

    fn prune_expired(&mut self) {
        let now = Instant::now();
        self.sessions.retain(|_, session| session.expires_at > now);
    }

    pub fn begin_session(&mut self, keys: DerivedKeys, network: WalletNetwork) -> GuardSession {
        self.prune_expired();
        let id = Uuid::new_v4().to_string();
        let now = Instant::now();
        let session = GuardSession {
            id: id.clone(),
            keys,
            network,
            created_at: now,
            expires_at: now + self.ttl,
        };
        self.sessions.insert(id, session.clone());
        session
    }

    pub fn get_session(&mut self, session_id: &str) -> Result<GuardSession, WalletError> {
        self.prune_expired();
        self.sessions
            .get(session_id)
            .cloned()
            .ok_or(WalletError::GuardSessionNotFound)
    }

    pub fn end_session(&mut self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some()
    }

    pub fn clear(&mut self) {
        self.sessions.clear();
    }
}

impl Default for GuardSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::crypto::derive_keys_v1;
    use crate::core::crypto::Network;

    #[test]
    fn begin_and_end_guard_session() {
        let keys = derive_keys_v1("guard test seed", Network::Mainnet).expect("derive");
        let mut manager = GuardSessionManager::new();
        let session = manager.begin_session(keys, WalletNetwork::Mainnet);
        assert!(manager.get_session(&session.id).is_ok());
        assert!(manager.end_session(&session.id));
        assert!(manager.get_session(&session.id).is_err());
    }
}
