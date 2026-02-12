//
// Module 4: Session-scoped preflight records. Cleared on lock; do not send to frontend.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Internal preflight record keyed by preflight_id. Used by router to dispatch send; payload is channel-specific.
#[derive(Clone)]
pub struct PreflightRecord {
    pub channel_id: String,
    pub account_id: String,
    pub payload: Value,
}

#[derive(Clone)]
struct StoredPreflightRecord {
    record: PreflightRecord,
    expires_at: Option<Instant>,
}

/// In-memory store of preflight records. Must be cleared when the wallet is locked (session-scoped).
pub struct PreflightStore {
    inner: Mutex<HashMap<String, StoredPreflightRecord>>,
}

impl PreflightStore {
    pub const DEFAULT_TTL: Duration = Duration::from_secs(20 * 60);

    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    fn prune_expired(inner: &mut HashMap<String, StoredPreflightRecord>) {
        let now = Instant::now();
        inner.retain(|_, stored| stored.expires_at.map(|expiry| expiry > now).unwrap_or(true));
    }

    pub fn get(&self, id: &str) -> Option<PreflightRecord> {
        let mut inner = self.inner.lock().expect("preflight store lock");
        Self::prune_expired(&mut inner);
        inner.get(id).map(|stored| stored.record.clone())
    }

    /// Get and consume a preflight record in one step (single-use send semantics).
    pub fn take(&self, id: &str) -> Option<PreflightRecord> {
        let mut inner = self.inner.lock().expect("preflight store lock");
        Self::prune_expired(&mut inner);
        inner.remove(id).map(|stored| stored.record)
    }

    pub fn put(&self, id: String, record: PreflightRecord) {
        self.put_with_ttl(id, record, Some(Self::DEFAULT_TTL));
    }

    pub fn put_with_ttl(&self, id: String, record: PreflightRecord, ttl: Option<Duration>) {
        let expires_at = ttl.map(|dur| Instant::now() + dur);
        self.inner
            .lock()
            .expect("preflight store lock")
            .insert(id, StoredPreflightRecord { record, expires_at });
    }

    /// Clear all records. Must be called when the user locks the wallet.
    pub fn clear(&self) {
        self.inner.lock().expect("preflight store lock").clear();
    }
}

impl Default for PreflightStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_record() -> PreflightRecord {
        PreflightRecord {
            channel_id: "vrpc.Rtest.i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            account_id: "acct".to_string(),
            payload: json!({"hex": "00"}),
        }
    }

    #[test]
    fn take_consumes_record() {
        let store = PreflightStore::new();
        store.put("id".to_string(), make_record());
        assert!(store.get("id").is_some());
        assert!(store.take("id").is_some());
        assert!(store.get("id").is_none());
    }

    #[test]
    fn ttl_expiry_removes_record() {
        let store = PreflightStore::new();
        store.put_with_ttl(
            "id".to_string(),
            make_record(),
            Some(Duration::from_millis(1)),
        );
        std::thread::sleep(Duration::from_millis(5));
        assert!(store.get("id").is_none());
    }
}
