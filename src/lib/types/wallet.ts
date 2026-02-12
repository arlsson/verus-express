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

export type IdentityOperation = 'update' | 'revoke' | 'recover';

export interface IdentityPatch {
  primaryAddresses?: string[] | null;
  recoveryAuthority?: string | null;
  revocationAuthority?: string | null;
  privateAddress?: string | null;
}

export interface HighRiskChange {
  changeType: string;
  beforeValue?: string | null;
  afterValue?: string | null;
}

export interface IdentityWarning {
  warningType: string;
  message: string;
}

export interface IdentityPreflightParams {
  coinId: string;
  channelId: string;
  operation: IdentityOperation;
  targetIdentity: string;
  patch?: IdentityPatch | null;
  memo?: string | null;
}

export interface IdentityPreflightResult {
  preflightId: string;
  operation: IdentityOperation;
  targetIdentity: string;
  fromAddress: string;
  fee: string;
  feeCurrency: string;
  highRiskChanges: HighRiskChange[];
  warnings: IdentityWarning[];
  memo?: string | null;
}

export interface IdentitySendRequest {
  preflightId: string;
}

export interface IdentitySendResult {
  txid: string;
  operation: IdentityOperation;
  targetIdentity: string;
  fee: string;
  fromAddress: string;
}

export interface VrpcTransferPreflightParams {
  coinId: string;
  channelId: string;
  sourceAddress?: string | null;
  destination: string;
  amount: string;
  convertTo?: string | null;
  exportTo?: string | null;
  via?: string | null;
  feeCurrency?: string | null;
  feeSatoshis?: string | null;
  preconvert?: boolean | null;
  mapTo?: string | null;
  vdxfTag?: string | null;
  memo?: string | null;
}

export interface VrpcTransferPreflightResult {
  preflightId: string;
  fee: string;
  feeCurrency: string;
  value: string;
  amountSubmitted: string;
  amountAdjusted?: string | null;
  toAddress: string;
  fromAddress: string;
  warnings: PreflightWarning[];
  memo?: string | null;
}

export interface BeginGuardSessionRequest {
  importText: string;
  network: WalletNetwork;
}

export interface BeginGuardSessionResult {
  guardSessionId: string;
  secretKind: 'seed_text' | 'wif' | 'private_key_hex' | string;
  vrscAddress: string;
  ethAddress: string;
  btcAddress: string;
  network: WalletNetwork;
}

export interface EndGuardSessionRequest {
  guardSessionId: string;
}

export interface EndGuardSessionResult {
  ended: boolean;
}

export interface GuardIdentityPreflightRequest {
  guardSessionId: string;
  params: IdentityPreflightParams;
}

export interface GuardIdentitySendRequest {
  guardSessionId: string;
  preflightId: string;
}

export type GuardFlowMode = 'revoke' | 'recover';
export type GuardFlowStep = 'secret' | 'target' | 'patch' | 'review' | 'result';

export interface GuardRecoverDraft {
  primaryAddress: string;
  recoveryAuthority: string;
  revocationAuthority: string;
  privateAddress: string;
}

export type GuardFlowErrorCode =
  | 'InvalidImportText'
  | 'GuardSessionNotFound'
  | 'IdentityNotFound'
  | 'IdentityInvalidState'
  | 'IdentityUnsupportedAuthority'
  | 'InvalidPreflight'
  | 'InsufficientFunds'
  | 'NetworkError'
  | 'OperationFailed'
  | 'IdentityBuildFailed'
  | 'IdentitySignFailed'
  | 'Unknown';

export type GuardPreflightResult = IdentityPreflightResult;
export type GuardSendResult = IdentitySendResult;

/** Placeholder for wallet://info-updated (Module 7). */
export interface ChainInfo {
  blocks?: number;
  longestChain?: number;
  syncing?: boolean;
}
