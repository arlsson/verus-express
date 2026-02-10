//
// Module 5d: BTC channel — balance, transactions, preflight, send for Bitcoin (P2PKH, same key as VRSC/ETH).
// Uses Mempool.space REST API (allowlist). Never sign UI-supplied tx hex.

mod balance;
mod preflight;
mod provider;
mod send;
mod transactions;

pub use balance::get_balances as get_balances_btc;
pub use preflight::preflight as preflight_btc;
pub use provider::{BtcProvider, BtcProviderPool};
pub use send::send as send_btc;
pub use transactions::get_transactions as get_transactions_btc;
