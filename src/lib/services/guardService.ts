/**
 * Thin invoke wrappers for signed-out guard session and identity tx flows.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
  BeginGuardSessionRequest,
  BeginGuardSessionResult,
  EndGuardSessionRequest,
  EndGuardSessionResult,
  GuardIdentityLookupRequest,
  GuardIdentityLookupResult,
  GuardIdentityPreflightRequest,
  GuardIdentitySendRequest,
  GuardPreflightResult,
  GuardSendResult
} from '$lib/types/wallet.js';

export async function beginGuardSession(
  request: BeginGuardSessionRequest
): Promise<BeginGuardSessionResult> {
  return invoke<BeginGuardSessionResult>('begin_guard_session', {
    request: {
      importText: request.importText,
      importMode: request.importMode,
      network: request.network
    }
  });
}

export async function endGuardSession(request: EndGuardSessionRequest): Promise<EndGuardSessionResult> {
  return invoke<EndGuardSessionResult>('end_guard_session', {
    request: {
      guardSessionId: request.guardSessionId
    }
  });
}

export async function preflightGuardIdentityUpdate(
  request: GuardIdentityPreflightRequest
): Promise<GuardPreflightResult> {
  return invoke<GuardPreflightResult>('preflight_guard_identity_update', {
    request: {
      guardSessionId: request.guardSessionId,
      params: request.params
    }
  });
}

export async function lookupGuardTargetIdentity(
  request: GuardIdentityLookupRequest
): Promise<GuardIdentityLookupResult> {
  return invoke<GuardIdentityLookupResult>('lookup_guard_target_identity', {
    request: {
      guardSessionId: request.guardSessionId,
      targetIdentity: request.targetIdentity
    }
  });
}

export async function sendGuardIdentityUpdate(
  request: GuardIdentitySendRequest
): Promise<GuardSendResult> {
  return invoke<GuardSendResult>('send_guard_identity_update', {
    request: {
      guardSessionId: request.guardSessionId,
      preflightId: request.preflightId
    }
  });
}
