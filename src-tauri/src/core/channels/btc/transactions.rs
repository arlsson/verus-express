//
// Module 5d: BTC transaction history via Mempool.space address/txs/chain and mempool.

use std::collections::HashSet;

use crate::core::channels::btc::provider::{BtcProvider, MempoolTx};
use crate::types::transaction::Transaction;
use crate::types::WalletError;

const SATOSHI_PER_COIN: f64 = 100_000_000.0;
const MEMPOOL_CHAIN_PAGE_SIZE: usize = 25;
const MAX_CHAIN_PAGE_FETCHES: usize = 12;

#[derive(Debug, Clone)]
pub struct BtcTransactionsPage {
    pub transactions: Vec<Transaction>,
    pub next_last_seen_txid: Option<String>,
    pub has_more: bool,
}

fn satoshi_to_decimal(sat: u64) -> String {
    format!("{:.8}", (sat as f64) / SATOSHI_PER_COIN)
}

fn map_mempool_tx_to_transaction(tx: MempoolTx) -> Transaction {
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

    Transaction {
        txid: tx.txid,
        amount: satoshi_to_decimal(amount),
        from_address: String::new(),
        to_address: String::new(),
        confirmations,
        timestamp: if timestamp > 0 { Some(timestamp) } else { None },
        pending,
    }
}

fn dedupe_and_sort_transactions(items: Vec<Transaction>) -> Vec<Transaction> {
    let mut seen = HashSet::<String>::new();
    let mut deduped = Vec::with_capacity(items.len());
    for tx in items {
        if seen.insert(tx.txid.clone()) {
            deduped.push(tx);
        }
    }

    deduped.sort_by(|a, b| {
        b.pending
            .cmp(&a.pending)
            .then(b.timestamp.unwrap_or(0).cmp(&a.timestamp.unwrap_or(0)))
    });
    deduped
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
        let mut cursor: Option<String> = None;
        loop {
            let batch = if let Some(ref last_seen_txid) = cursor {
                provider
                    .get_address_txs_chain_after(addr, last_seen_txid)
                    .await?
            } else {
                provider.get_address_txs(addr).await?
            };

            if batch.is_empty() {
                break;
            }

            let batch_len = batch.len();
            cursor = batch.last().map(|tx| tx.txid.clone());
            all.extend(batch.into_iter().map(map_mempool_tx_to_transaction));

            if batch_len < MEMPOOL_CHAIN_PAGE_SIZE {
                break;
            }
            if cursor.is_none() {
                break;
            }
        }
    }

    Ok(dedupe_and_sort_transactions(all))
}

pub async fn get_transactions_page(
    provider: &BtcProvider,
    addresses: &[String],
    last_seen_txid: Option<&str>,
    limit: usize,
) -> Result<BtcTransactionsPage, WalletError> {
    let safe_limit = limit.clamp(1, 100);
    let Some(address) = addresses.first() else {
        return Ok(BtcTransactionsPage {
            transactions: vec![],
            next_last_seen_txid: None,
            has_more: false,
        });
    };

    let mut fetched = Vec::<Transaction>::new();
    let mut cursor = last_seen_txid.map(str::to_string);
    let mut exhausted = false;
    let mut likely_more = false;

    for _ in 0..MAX_CHAIN_PAGE_FETCHES {
        let batch = if let Some(ref seen_txid) = cursor {
            provider
                .get_address_txs_chain_after(address, seen_txid)
                .await?
        } else {
            provider.get_address_txs(address).await?
        };

        if batch.is_empty() {
            exhausted = true;
            break;
        }

        let batch_len = batch.len();
        cursor = batch.last().map(|tx| tx.txid.clone());
        fetched.extend(batch.into_iter().map(map_mempool_tx_to_transaction));

        if fetched.len() > safe_limit {
            likely_more = true;
            break;
        }

        if batch_len < MEMPOOL_CHAIN_PAGE_SIZE {
            exhausted = true;
            break;
        }

        likely_more = true;
        if cursor.is_none() {
            exhausted = true;
            break;
        }
    }

    let mut transactions = dedupe_and_sort_transactions(fetched);
    if transactions.len() > safe_limit {
        transactions.truncate(safe_limit);
        likely_more = true;
    }

    let has_more = likely_more && (!exhausted || transactions.len() >= safe_limit);
    let next_last_seen_txid = if has_more {
        transactions
            .last()
            .map(|tx| tx.txid.clone())
            .or(cursor.clone())
    } else {
        None
    };

    Ok(BtcTransactionsPage {
        transactions,
        next_last_seen_txid,
        has_more,
    })
}
