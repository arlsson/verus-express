/**
 * Coin registry store: list of CoinDefinition from get_coin_registry.
 * Populated on wallet load via coinsService.getCoinRegistry().
 */

import { writable } from 'svelte/store';
import type { CoinDefinition } from '$lib/types/wallet.js';

const initialState: CoinDefinition[] = [];

export const coinsStore = writable<CoinDefinition[]>(initialState);
