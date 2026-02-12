/**
 * Thin invoke wrapper for advanced VRPC transfer preflight.
 */

import { invoke } from '@tauri-apps/api/core';
import type { VrpcTransferPreflightParams, VrpcTransferPreflightResult } from '$lib/types/wallet.js';

export async function preflightVrpcTransfer(
  params: VrpcTransferPreflightParams
): Promise<VrpcTransferPreflightResult> {
  return invoke<VrpcTransferPreflightResult>('preflight_vrpc_transfer', {
    params: {
      coinId: params.coinId,
      channelId: params.channelId,
      sourceAddress: params.sourceAddress ?? null,
      destination: params.destination,
      amount: params.amount,
      convertTo: params.convertTo ?? null,
      exportTo: params.exportTo ?? null,
      via: params.via ?? null,
      feeCurrency: params.feeCurrency ?? null,
      feeSatoshis: params.feeSatoshis ?? null,
      preconvert: params.preconvert ?? null,
      mapTo: params.mapTo ?? null,
      vdxfTag: params.vdxfTag ?? null,
      memo: params.memo ?? null
    }
  });
}
