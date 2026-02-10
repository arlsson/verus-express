/**
 * Frontend types mirroring backend serialized shapes (camelCase).
 * Used by stores, services, and UI. No sensitive fields.
 * Last Updated: Module 9 — balance, transaction, coin, preflight, send types.
 */

export interface BalanceResult {
  confirmed: string;
  pending: string;
  total: string;
}

export interface Transaction {
  txid: string;
  amount: string;
  fromAddress: string;
  toAddress: string;
  confirmations: number;
  timestamp?: number;
  pending: boolean;
}

export type WalletNetwork = 'mainnet' | 'testnet';

export type Protocol = 'vrsc' | 'btc' | 'eth' | 'erc20';
export type Channel = 'vrpc' | 'btc' | 'eth' | 'erc20';

export interface CoinDefinition {
  id: string;
  currencyId: string;
  systemId: string;
  displayTicker: string;
  displayName: string;
  proto: Protocol;
  compatibleChannels: Channel[];
  decimals: number;
  vrpcEndpoints: string[];
  electrumEndpoints?: string[] | null;
  secondsPerBlock: number;
  mappedTo?: string | null;
  isTestnet: boolean;
}

export interface PreflightWarning {
  warningType: string;
  message: string;
}

export interface PreflightResult {
  preflightId: string;
  fee: string;
  feeCurrency: string;
  value: string;
  amountSubmitted: string;
  toAddress: string;
  fromAddress: string;
  feeTakenFromAmount: boolean;
  feeTakenMessage?: string | null;
  warnings: PreflightWarning[];
  memo?: string | null;
}

export interface PreflightParams {
  coinId: string;
  channelId: string;
  toAddress: string;
  amount: string;
  memo?: string | null;
}

export interface SendRequest {
  preflightId: string;
}

export interface SendResult {
  txid: string;
  fee: string;
  value: string;
  toAddress: string;
  fromAddress: string;
}

/** Placeholder for wallet://info-updated (Module 7). */
export interface ChainInfo {
  blocks?: number;
  longestChain?: number;
  syncing?: boolean;
}
