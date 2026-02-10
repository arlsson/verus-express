/**
 * Thin invoke wrappers for wallet Tauri commands.
 * Security: No sensitive data in logs.
 */

import { invoke } from '@tauri-apps/api/core';
import type { BalanceResult, Transaction, WalletNetwork } from '$lib/types/wallet.js';

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

export async function unlockWallet(payload: UnlockWalletPayload): Promise<void> {
  await invoke('unlock_wallet', {
    account_id: payload.account_id,
    password: payload.password
  });
}

export async function lockWallet(): Promise<void> {
  await invoke('lock_wallet');
}

export async function startUpdateEngine(): Promise<void> {
  await invoke('start_update_engine');
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

export async function getBalances(channelId: string): Promise<BalanceResult> {
  return invoke<BalanceResult>('get_balances', { channel_id: channelId });
}

export async function getTransactionHistory(channelId: string): Promise<Transaction[]> {
  return invoke<Transaction[]>('get_transaction_history', { channel_id: channelId });
}
