//
// Module 5d: BTC balance via Mempool.space address API. Maps to BalanceResult (decimal strings).

use crate::core::channels::btc::provider::{BtcProvider, UtxoEntry};
use crate::types::transaction::BalanceResult;
use crate::types::WalletError;

const SATOSHI_PER_COIN: f64 = 100_000_000.0;

fn satoshi_to_decimal(sat: u64) -> String {
    format!("{:.8}", (sat as f64) / SATOSHI_PER_COIN)
}

/// Fetch balance for addresses and return as BalanceResult (sum of all addresses).
pub async fn get_balances(
    provider: &BtcProvider,
    addresses: &[String],
) -> Result<BalanceResult, WalletError> {
    if addresses.is_empty() {
        return Ok(BalanceResult {
            confirmed: "0".to_string(),
            pending: "0".to_string(),
            total: "0".to_string(),
        });
    }

    let mut confirmed: u64 = 0;
    let mut pending: u64 = 0;

    for addr in addresses {
        let info = provider.get_address_info(addr).await?;
        let chain = info
            .chain_stats
            .funded_txo_sum
            .saturating_sub(info.chain_stats.spent_txo_sum);
        let mempool = info
            .mempool_stats
            .funded_txo_sum
            .saturating_sub(info.mempool_stats.spent_txo_sum);
        confirmed = confirmed.saturating_add(chain);
        pending = pending.saturating_add(mempool);
    }

    let total = confirmed.saturating_add(pending);
    Ok(BalanceResult {
        confirmed: satoshi_to_decimal(confirmed),
        pending: satoshi_to_decimal(pending),
        total: satoshi_to_decimal(total),
    })
}

/// Sum spendable amount from UTXO list (for preflight).
pub fn sum_utxos(utxos: &[UtxoEntry]) -> u64 {
    utxos.iter().map(|u| u.value).sum()
}
