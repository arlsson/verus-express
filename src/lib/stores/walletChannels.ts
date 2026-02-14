/**
 * Canonical wallet channels keyed by coin id.
 * VRPC channels use address-scoped format: vrpc.<address>.<systemId>.
 */

import { writable } from 'svelte/store';
import type { CoinDefinition } from '$lib/types/wallet.js';
import { channelIdForCoin } from '$lib/utils/channelId.js';

export interface WalletChannelsState {
  byCoinId: Record<string, string>;
  channels: string[];
  primaryChannelId: string | null;
  vrpcAddress: string | null;
}

const initialState: WalletChannelsState = {
  byCoinId: {},
  channels: [],
  primaryChannelId: null,
  vrpcAddress: null
};

export const walletChannelsStore = writable<WalletChannelsState>(initialState);

export function buildWalletChannels(
  coins: CoinDefinition[],
  vrpcAddress: string | null
): WalletChannelsState {
  const byCoinId: Record<string, string> = {};
  const channels: string[] = [];

  for (const coin of coins) {
    const channelId = channelIdForCoin(coin, vrpcAddress ?? undefined);
    if (!channelId) continue;
    byCoinId[coin.id] = channelId;
    if (!channels.includes(channelId)) {
      channels.push(channelId);
    }
  }

  const primaryVrpcCoin = coins.find((c) => c.compatibleChannels.includes('vrpc'));
  const primaryBtcCoin = coins.find((c) => c.compatibleChannels.includes('btc'));
  const primaryEthCoin = coins.find((c) => c.compatibleChannels.includes('eth'));
  const primaryErc20Coin = coins.find((c) => c.compatibleChannels.includes('erc20'));

  const primaryChannelId =
    (primaryVrpcCoin ? byCoinId[primaryVrpcCoin.id] : null) ??
    (primaryBtcCoin ? byCoinId[primaryBtcCoin.id] : null) ??
    (primaryEthCoin ? byCoinId[primaryEthCoin.id] : null) ??
    (primaryErc20Coin ? byCoinId[primaryErc20Coin.id] : null) ??
    channels[0] ??
    null;

  return {
    byCoinId,
    channels,
    primaryChannelId,
    vrpcAddress
  };
}

export function resetWalletChannels(): void {
  walletChannelsStore.set(initialState);
}
