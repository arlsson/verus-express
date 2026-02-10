/**
 * Transaction history store: channel_id -> Transaction[].
 * Key = channel_id. Updated by pull (get_transaction_history) or future wallet://transactions-updated.
 */

import { writable } from 'svelte/store';
import type { Transaction } from '$lib/types/wallet.js';

const initialState: Record<string, Transaction[]> = {};

export const transactionStore = writable<Record<string, Transaction[]>>(initialState);

export function getTransactions(channelId: string, transactions: Record<string, Transaction[]>): Transaction[] {
  return transactions[channelId] ?? [];
}
