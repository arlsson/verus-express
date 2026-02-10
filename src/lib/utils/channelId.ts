/**
 * Helpers for channel_id used by backend (vrpc.<address>.<systemId>, btc.<coinId>).
 * Store keys match these IDs for pull and future event push.
 */

import type { CoinDefinition } from '$lib/types/wallet.js';

/**
 * Canonical VRPC channel format: vrpc.<address>.<systemId>.
 * Legacy compatibility format (temporary): vrpc.<coinId>.<systemId>.
 */
export function buildVrpcChannelId(address: string, systemId: string): string {
  return `vrpc.${address}.${systemId}`;
}

export function parseVrpcChannelId(channelId: string): { middle: string; systemId: string } | null {
  if (!channelId.startsWith('vrpc.')) return null;
  const rest = channelId.slice('vrpc.'.length);
  const idx = rest.lastIndexOf('.');
  if (idx <= 0 || idx >= rest.length - 1) return null;
  return {
    middle: rest.slice(0, idx),
    systemId: rest.slice(idx + 1)
  };
}

export function isLikelyVrpcAddress(value: string): boolean {
  if (!value) return false;
  return value.startsWith('R') || value.startsWith('i') || value.includes('@');
}

export function canonicalizeVrpcChannelId(channelId: string, vrpcAddress?: string): string {
  const parsed = parseVrpcChannelId(channelId);
  if (!parsed) return channelId;
  if (isLikelyVrpcAddress(parsed.middle)) {
    return buildVrpcChannelId(parsed.middle, parsed.systemId);
  }
  if (vrpcAddress) {
    return buildVrpcChannelId(vrpcAddress, parsed.systemId);
  }
  return channelId;
}

/** Build channel_id for a coin for balance/tx/preflight. */
export function channelIdForCoin(coin: CoinDefinition, vrpcAddress?: string): string | null {
  if (coin.compatibleChannels.includes('vrpc')) {
    if (vrpcAddress) return buildVrpcChannelId(vrpcAddress, coin.systemId);
    // Temporary legacy fallback if address has not been loaded yet.
    return `vrpc.${coin.id}.${coin.systemId}`;
  }
  if (coin.compatibleChannels.includes('btc')) {
    return `btc.${coin.id}`;
  }
  return null;
}

/** Default channel IDs to fetch on wallet load: first VRSC and first BTC. */
export function defaultChannelIds(coins: CoinDefinition[], vrpcAddress?: string): string[] {
  const ids: string[] = [];
  for (const coin of coins) {
    const cid = channelIdForCoin(coin, vrpcAddress);
    if (cid && !ids.includes(cid)) {
      ids.push(cid);
    }
  }
  return ids;
}

/** Primary display channel (first VRPC or first available). */
export function primaryChannelId(coins: CoinDefinition[], vrpcAddress?: string): string | null {
  const vrpc = coins.find((c) => c.compatibleChannels.includes('vrpc'));
  if (vrpc) return channelIdForCoin(vrpc, vrpcAddress);
  const btc = coins.find((c) => c.compatibleChannels.includes('btc'));
  if (btc) return channelIdForCoin(btc, vrpcAddress);
  return null;
}
