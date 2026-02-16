/**
 * Thin invoke wrappers for bridge transfer commands.
 * Security: No tx hex or signing payloads leave backend trust boundary.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
  BridgeConversionEstimateRequest,
  BridgeConversionEstimateResult,
  BridgeConversionPathRequest,
  BridgeConversionPathsResult,
  BridgeTransferPreflightParams,
  BridgeTransferPreflightResult
} from '$lib/types/wallet.js';

export async function getBridgeConversionPaths(
  request: BridgeConversionPathRequest
): Promise<BridgeConversionPathsResult> {
  return invoke<BridgeConversionPathsResult>('get_bridge_conversion_paths', {
    request: {
      coinId: request.coinId,
      channelId: request.channelId,
      sourceCurrency: request.sourceCurrency,
      destinationCurrency: request.destinationCurrency ?? null
    }
  });
}

export async function estimateBridgeConversion(
  request: BridgeConversionEstimateRequest
): Promise<BridgeConversionEstimateResult> {
  return invoke<BridgeConversionEstimateResult>('estimate_bridge_conversion', {
    request: {
      coinId: request.coinId,
      channelId: request.channelId,
      sourceCurrency: request.sourceCurrency,
      convertTo: request.convertTo,
      amount: request.amount,
      via: request.via ?? null,
      preconvert: request.preconvert ?? null
    }
  });
}

export async function preflightBridgeTransfer(
  params: BridgeTransferPreflightParams
): Promise<BridgeTransferPreflightResult> {
  return invoke<BridgeTransferPreflightResult>('preflight_bridge_transfer', {
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
