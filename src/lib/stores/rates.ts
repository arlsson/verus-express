/**
 * Fiat rates store: coinId -> fiat rates + optional 24h USD percentage change.
 */

import { writable } from 'svelte/store';

export interface CoinRatesSnapshot {
  rates: Record<string, number>;
  usdChange24hPct: number | null;
}

const initialState: Record<string, CoinRatesSnapshot> = {};

export const ratesStore = writable<Record<string, CoinRatesSnapshot>>(initialState);
