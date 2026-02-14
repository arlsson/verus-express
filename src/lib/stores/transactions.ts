/**
 * Transaction history store: channel_id -> coin_id -> Transaction[].
 */

import { writable } from 'svelte/store';
import type { Transaction } from '$lib/types/wallet.js';

export type TransactionsByChannel = Record<string, Record<string, Transaction[]>>;

const initialState: TransactionsByChannel = {};

export const transactionStore = writable<TransactionsByChannel>(initialState);

export function getTransactions(
  channelId: string,
  coinId: string,
  transactions: TransactionsByChannel
): Transaction[] {
  return transactions[channelId]?.[coinId] ?? [];
}
