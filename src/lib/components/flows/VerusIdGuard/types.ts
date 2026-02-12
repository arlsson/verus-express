import type { GuardPreflightResult, GuardSendResult, WalletNetwork } from '$lib/types/wallet.js';

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

export interface GuardReviewContext {
  mode: GuardFlowMode;
  network: WalletNetwork;
  targetIdentity: string;
  authorityAddress: string;
  preflight: GuardPreflightResult;
  recoverDraft: GuardRecoverDraft;
}

export interface GuardResultContext {
  mode: GuardFlowMode;
  sendResult: GuardSendResult | null;
  errorMessage: string;
}
