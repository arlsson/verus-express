// 
// Authentication and session management module
// Security: Handles secure seed storage and session lifecycle with zeroization
// Last Updated: Created for Module 1 integration

pub mod session;
pub mod stronghold_store;

pub use session::SessionManager;
pub use stronghold_store::StrongholdStore;
