/**
 * Thin invoke wrappers for coin registry Tauri commands.
 */

import { invoke } from '@tauri-apps/api/core';
import type { CoinDefinition } from '$lib/types/wallet.js';

export async function getCoinRegistry(): Promise<CoinDefinition[]> {
  return invoke<CoinDefinition[]>('get_coin_registry');
}

export async function addPbaasCurrency(definition: CoinDefinition): Promise<void> {
  await invoke('add_pbaas_currency', { definition });
}
