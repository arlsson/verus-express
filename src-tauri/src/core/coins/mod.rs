//
// Module 3: Coin Registry — coin types and registry (static + dynamic PBaaS).
// Endpoints are allowlist-only; see registry.rs for policy.

mod registry;
mod types;

pub use registry::CoinRegistry;
pub use types::{Channel, CoinDefinition, Protocol};
