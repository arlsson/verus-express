/**
 * Fiat rates store: coinId -> { currency -> rate }.
 */

import { writable } from 'svelte/store';

const initialState: Record<string, Record<string, number>> = {};

export const ratesStore = writable<Record<string, Record<string, number>>>(initialState);
