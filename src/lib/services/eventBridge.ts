/**
 * Tauri event bridge: listen for wallet://* events and update Svelte stores.
 * Module 7 Update Engine emits balances-updated and transactions-updated; bridge updates stores.
 * Call setupWalletEventBridge() when wallet layout mounts; call returned cleanup on destroy.
 */

import { listen } from '@tauri-apps/api/event';
import { get } from 'svelte/store';
import { balanceStore } from '$lib/stores/balances.js';
import { transactionStore } from '$lib/stores/transactions.js';
import { networkStore } from '$lib/stores/network.js';
import { ratesStore } from '$lib/stores/rates.js';
import { walletChannelsStore } from '$lib/stores/walletChannels.js';
import { pushWalletError } from '$lib/stores/walletErrors.js';
import type { BalanceResult, ChainInfo, Transaction } from '$lib/types/wallet.js';
import { canonicalizeVrpcChannelId } from '$lib/utils/channelId.js';

const BALANCES_UPDATED = 'wallet://balances-updated';
const TRANSACTIONS_UPDATED = 'wallet://transactions-updated';
const INFO_UPDATED = 'wallet://info-updated';
const RATES_UPDATED = 'wallet://rates-updated';
const ERROR = 'wallet://error';

interface BalancesUpdatedPayload {
  coinId?: string;
  channel?: string;
  channelId?: string;
  confirmed?: string;
  pending?: string;
  total?: string;
}

interface TransactionsUpdatedPayload {
  coinId?: string;
  channel?: string;
  channelId?: string;
  transactions?: Transaction[];
}

interface InfoUpdatedPayload {
  coinId?: string;
  channel?: string;
  channelId?: string;
  blocks?: number;
  longestChain?: number;
  syncing?: boolean;
}

interface RatesUpdatedPayload {
  coinId?: string;
  rates?: Record<string, number>;
}

interface UpdateErrorPayload {
  dataType?: string;
  coinId?: string;
  channel?: string;
  message?: string;
}

function normalizeChannelKey(key: string): string {
  if (!key.startsWith('vrpc.')) return key;
  const { vrpcAddress } = get(walletChannelsStore);
  return canonicalizeVrpcChannelId(key, vrpcAddress ?? undefined);
}

/**
 * Builds store key from payload. Backend sends channel = channel_id (e.g. vrpc.VRSC.i5w5... or btc.BTC).
 */
function balanceKey(p: BalancesUpdatedPayload): string {
  if (p.channel) return p.channel;
  if (p.channelId) return p.channelId;
  return p.coinId ?? 'unknown';
}

function txKey(p: TransactionsUpdatedPayload): string {
  if (p.channel) return p.channel;
  if (p.channelId) return p.channelId;
  return p.coinId ?? 'unknown';
}

function infoKey(p: InfoUpdatedPayload): string {
  if (p.channelId) return p.channelId;
  if (p.coinId && p.channel) return `${p.channel}.${p.coinId}`;
  return p.coinId ?? p.channel ?? 'unknown';
}

/**
 * Registers all wallet event listeners and returns a cleanup function.
 */
export async function setupWalletEventBridge(): Promise<() => void> {
  const unsubs: (() => void)[] = [];

  const unBalances = await listen<BalancesUpdatedPayload>(BALANCES_UPDATED, (event) => {
    const p = event.payload;
    const key = normalizeChannelKey(balanceKey(p));
    const value: BalanceResult = {
      confirmed: p.confirmed ?? '0',
      pending: p.pending ?? '0',
      total: p.total ?? '0'
    };
    balanceStore.update((m) => ({ ...m, [key]: value }));
  });
  unsubs.push(() => unBalances());

  const unTx = await listen<TransactionsUpdatedPayload>(TRANSACTIONS_UPDATED, (event) => {
    const p = event.payload;
    const key = normalizeChannelKey(txKey(p));
    const list = p.transactions ?? [];
    transactionStore.update((m) => ({ ...m, [key]: list }));
  });
  unsubs.push(() => unTx());

  const unInfo = await listen<InfoUpdatedPayload>(INFO_UPDATED, (event) => {
    const p = event.payload;
    const key = normalizeChannelKey(infoKey(p));
    const value: ChainInfo = {
      blocks: p.blocks,
      longestChain: p.longestChain,
      syncing: p.syncing
    };
    networkStore.update((m) => ({ ...m, [key]: value }));
  });
  unsubs.push(() => unInfo());

  const unRates = await listen<RatesUpdatedPayload>(RATES_UPDATED, (event) => {
    const p = event.payload;
    const coinId = p.coinId ?? 'default';
    const rates = p.rates ?? {};
    ratesStore.update((m) => ({ ...m, [coinId]: rates }));
  });
  unsubs.push(() => unRates());

  const unError = await listen<UpdateErrorPayload>(ERROR, (event) => {
    const p = event.payload;
    const channel = p.channel ? normalizeChannelKey(p.channel) : '';
    const type = p.dataType ?? 'wallet';
    const message = p.message ?? 'Temporarily unavailable';
    const prefix = channel ? `${type} (${channel})` : type;
    pushWalletError(`${prefix}: ${message}`);
  });
  unsubs.push(() => unError());

  return () => {
    unsubs.forEach((u) => u());
  };
}
