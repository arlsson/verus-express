//
// Module 3: Coin Registry — coin types and registry (static + dynamic PBaaS).
// Endpoint defaults come from runtime config; see registry.rs.

mod registry;
mod types;

pub use registry::CoinRegistry;
pub use types::{Channel, CoinDefinition, Protocol};
