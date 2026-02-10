/**
 * Network/chain info store: key (channel or coin) -> ChainInfo.
 * Placeholder until Module 7 (Update Engine) emits wallet://info-updated.
 */

import { writable } from 'svelte/store';
import type { ChainInfo } from '$lib/types/wallet.js';

const initialState: Record<string, ChainInfo> = {};

export const networkStore = writable<Record<string, ChainInfo>>(initialState);
