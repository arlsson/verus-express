/**
 * Thin invoke wrappers for wallet Tauri commands.
 * Security: No sensitive data in logs.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
  ActiveAssetsState,
  BalanceResult,
  CoinScopesResult,
  DlightProverStatusResult,
  DlightRuntimeStatusResult,
  DlightSeedStatusResult,
  SetupDlightSeedRequest,
  SetupDlightSeedResult,
  Transaction,
  TransactionHistoryPage,
  WalletNetwork
} from '$lib/types/wallet.js';

export interface UnlockWalletPayload {
  account_id: string;
  password: string;
}

export interface ActiveWalletResponse {
  wallet_name: string;
  network: WalletNetwork;
  emoji: string;
  color: string;
}

export interface AddressResponse {
  vrsc_address: string;
  eth_address: string;
  btc_address: string;
}

export interface StartUpdateEngineOptions {
  includeTransactions?: boolean;
  priorityCoinIds?: string[];
  priorityChannelIds?: string[];
}

export async function unlockWallet(payload: UnlockWalletPayload): Promise<void> {
  await invoke('unlock_wallet', {
    account_id: payload.account_id,
    password: payload.password
  });
}

export async function lockWallet(): Promise<void> {
  await invoke('lock_wallet');
}

export async function startUpdateEngine(options: StartUpdateEngineOptions | boolean = false): Promise<void> {
  const resolvedOptions =
    typeof options === 'boolean'
      ? { includeTransactions: options }
      : options;

  await invoke('start_update_engine', {
    request: {
      include_transactions: resolvedOptions.includeTransactions ?? false,
      priority_coin_ids: resolvedOptions.priorityCoinIds ?? [],
      priority_channel_ids: resolvedOptions.priorityChannelIds ?? []
    }
  });
}

export async function isUnlocked(): Promise<boolean> {
  return invoke<boolean>('is_unlocked');
}

export async function getActiveWallet(): Promise<ActiveWalletResponse | null> {
  return invoke<ActiveWalletResponse | null>('get_active_wallet');
}

export async function getAddresses(): Promise<AddressResponse> {
  return invoke<AddressResponse>('get_addresses');
}

export async function getCoinScopes(coinId: string): Promise<CoinScopesResult> {
  return invoke<CoinScopesResult>('get_coin_scopes', { coin_id: coinId });
}

export async function getWatchedVrpcAddresses(): Promise<string[]> {
  return invoke<string[]>('get_watched_vrpc_addresses');
}

export async function setWatchedVrpcAddresses(addresses: string[]): Promise<string[]> {
  return invoke<string[]>('set_watched_vrpc_addresses', { addresses });
}

export async function getActiveAssets(): Promise<ActiveAssetsState> {
  return invoke<ActiveAssetsState>('get_active_assets');
}

export async function setActiveAssets(coinIds: string[]): Promise<ActiveAssetsState> {
  return invoke<ActiveAssetsState>('set_active_assets', { coin_ids: coinIds });
}

export async function getDlightSeedStatus(): Promise<DlightSeedStatusResult> {
  return invoke<DlightSeedStatusResult>('get_dlight_seed_status');
}

export async function setupDlightSeed(
  request: SetupDlightSeedRequest
): Promise<SetupDlightSeedResult> {
  return invoke<SetupDlightSeedResult>('setup_dlight_seed', {
    request: {
      mode: request.mode,
      import_text: request.importText ?? null
    }
  });
}

export async function getDlightRuntimeStatus(
  channelId: string,
  coinId?: string
): Promise<DlightRuntimeStatusResult> {
  return invoke<DlightRuntimeStatusResult>('get_dlight_runtime_status', {
    channel_id: channelId,
    ...(coinId ? { coin_id: coinId } : {})
  });
}

export async function getDlightProverStatus(): Promise<DlightProverStatusResult> {
  return invoke<DlightProverStatusResult>('get_dlight_prover_status');
}

export async function readClipboardText(): Promise<string> {
  return invoke<string>('read_clipboard_text');
}

export async function getBalances(channelId: string, coinId?: string): Promise<BalanceResult> {
  return invoke<BalanceResult>('get_balances', {
    channel_id: channelId,
    ...(coinId ? { coin_id: coinId } : {})
  });
}

export async function getTransactionHistory(channelId: string, coinId?: string): Promise<Transaction[]> {
  return invoke<Transaction[]>('get_transaction_history', {
    channel_id: channelId,
    ...(coinId ? { coin_id: coinId } : {})
  });
}

export async function getTransactionHistoryPage(
  channelId: string,
  coinId?: string,
  cursor?: string,
  limit = 50
): Promise<TransactionHistoryPage> {
  return invoke<TransactionHistoryPage>('get_transaction_history_page', {
    request: {
      channelId,
      ...(coinId ? { coinId } : {}),
      ...(cursor ? { cursor } : {}),
      limit
    }
  });
}
