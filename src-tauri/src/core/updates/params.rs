//
// Module 7: Update engine interval and expiry params. Mirrors Verus-Mobile defaultUpdateParams.js.
// Jitter ±15% to avoid thundering herd across coins.

use std::time::Duration;

/// Balances: consider stale after 60s, refresh every 300s (5 min).
pub const BALANCE_EXPIRE_SECS: u64 = 60;
pub const BALANCE_REFRESH_SECS: u64 = 300;

/// Transactions: same as balances.
pub const TRANSACTION_EXPIRE_SECS: u64 = 60;
pub const TRANSACTION_REFRESH_SECS: u64 = 300;

/// Chain info: fixed 30s for first version (full lifecycle deferred).
pub const CHAIN_INFO_REFRESH_SECS: u64 = 30;

/// Fiat rates: stub; same intervals when implemented.
pub const RATES_EXPIRE_SECS: u64 = 60;
pub const RATES_REFRESH_SECS: u64 = 300;

/// Jitter: ±15% of base duration (plan: "matching mobile's 30% margin" — use 15% each side).
pub const JITTER_PERCENT: f64 = 0.15;

/// Returns a duration with ±15% jitter applied. Uses rand for randomness.
pub fn jitter_duration(base_secs: u64) -> Duration {
    let base = base_secs as f64;
    let margin = base * JITTER_PERCENT;
    let offset = (rand::random::<f64>() * 2.0 - 1.0) * margin;
    let secs = (base + offset).max(1.0);
    Duration::from_secs_f64(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jitter_stays_positive() {
        for _ in 0..100 {
            let d = jitter_duration(300);
            assert!(d.as_secs() >= 1, "jitter should not go below 1s");
        }
    }
}
