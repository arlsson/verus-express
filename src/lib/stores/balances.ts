/**
 * Balance store: channel_id -> coin_id -> BalanceResult.
 * Mirrors mobile parity shape to avoid collapsing PBaaS balances on shared VRPC channels.
 */

import { writable } from 'svelte/store';
import type { BalanceResult } from '$lib/types/wallet.js';

export type BalancesByChannel = Record<string, Record<string, BalanceResult>>;

const initialState: BalancesByChannel = {};

export const balanceStore = writable<BalancesByChannel>(initialState);

export function getBalance(
  channelId: string,
  coinId: string,
  balances: BalancesByChannel
): BalanceResult | undefined {
  return balances[channelId]?.[coinId];
}
