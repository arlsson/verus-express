//
// Module 4: Session-scoped preflight records. Cleared on lock; do not send to frontend.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

/// Internal preflight record keyed by preflight_id. Used by router to dispatch send; payload is channel-specific.
#[derive(Clone)]
pub struct PreflightRecord {
    pub channel_id: String,
    pub account_id: String,
    pub payload: Value,
}

/// In-memory store of preflight records. Must be cleared when the wallet is locked (session-scoped).
pub struct PreflightStore {
    inner: Mutex<HashMap<String, PreflightRecord>>,
}

impl PreflightStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, id: &str) -> Option<PreflightRecord> {
        self.inner
            .lock()
            .expect("preflight store lock")
            .get(id)
            .cloned()
    }

    pub fn put(&self, id: String, record: PreflightRecord) {
        self.inner
            .lock()
            .expect("preflight store lock")
            .insert(id, record);
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
