//
// Module 5d: BTC transaction history via Mempool.space address/txs/chain and mempool.

use crate::core::channels::btc::provider::{BtcProvider, MempoolTx};
use crate::types::transaction::Transaction;
use crate::types::WalletError;

const SATOSHI_PER_COIN: f64 = 100_000_000.0;

fn satoshi_to_decimal(sat: u64) -> String {
    format!("{:.8}", (sat as f64) / SATOSHI_PER_COIN)
}

/// Fetch transaction history for addresses (confirmed + mempool).
pub async fn get_transactions(
    provider: &BtcProvider,
    addresses: &[String],
) -> Result<Vec<Transaction>, WalletError> {
    if addresses.is_empty() {
        return Ok(vec![]);
    }

    let mut all = Vec::new();
    for addr in addresses {
        let chain = provider.get_address_txs(addr).await?;
        for tx in chain {
            let (confirmations, timestamp, pending) = if let Some(ref st) = tx.status {
                (
                    if st.confirmed { 1i64 } else { 0 },
                    st.block_time.unwrap_or(0),
                    !st.confirmed,
                )
            } else {
                (0, 0, true)
            };
            let amount = tx
                .vout
                .iter()
                .filter_map(|v| v.get("value").and_then(|n| n.as_u64()))
                .sum::<u64>();
            all.push(Transaction {
                txid: tx.txid,
                amount: satoshi_to_decimal(amount),
                from_address: String::new(),
                to_address: String::new(),
                confirmations,
                timestamp: if timestamp > 0 { Some(timestamp) } else { None },
                pending,
            });
        }
    }
    all.sort_by(|a, b| b.timestamp.unwrap_or(0).cmp(&a.timestamp.unwrap_or(0)));
    Ok(all)
}
