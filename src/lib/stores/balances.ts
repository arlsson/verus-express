/**
 * Balance store: channel_id -> BalanceResult.
 * Key = channel_id (e.g. vrpc.VRSC.i-5d9c... or btc.BTC). Updated by pull (get_balances) or future wallet://balances-updated.
 */

import { writable } from 'svelte/store';
import type { BalanceResult } from '$lib/types/wallet.js';

const initialState: Record<string, BalanceResult> = {};

export const balanceStore = writable<Record<string, BalanceResult>>(initialState);

export function getBalance(channelId: string, balances: Record<string, BalanceResult>): BalanceResult | undefined {
  return balances[channelId];
}
