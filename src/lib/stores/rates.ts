/**
 * Fiat rates store: coinId -> { currency -> rate }.
 * Placeholder until Module 7 (Update Engine) emits wallet://rates-updated.
 */

import { writable } from 'svelte/store';

const initialState: Record<string, Record<string, number>> = {};

export const ratesStore = writable<Record<string, Record<string, number>>>(initialState);
