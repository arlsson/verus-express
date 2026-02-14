import type { Protocol, WalletNetwork } from '$lib/types/wallet.js';

type SupportedAssetProto = Protocol | 'fiat';

export interface SupportedAssetInput {
  id: string;
  currencyId: string;
  proto: SupportedAssetProto;
  isTestnet: boolean;
}

/**
 * Central wallet capability gate.
 * v1 support: VRSC/PBaaS, BTC, ETH, and ERC20 (Ethereum mainnet only).
 */
export function isWalletSupportedAsset(asset: SupportedAssetInput, network: WalletNetwork): boolean {
  const isTestnet = network === 'testnet';
  if (asset.isTestnet !== isTestnet) {
    return false;
  }

  if (asset.proto === 'vrsc' || asset.proto === 'eth') {
    return true;
  }

  if (asset.proto === 'erc20') {
    return network === 'mainnet' && asset.currencyId.startsWith('0x');
  }

  if (asset.proto === 'btc') {
    return asset.id === 'BTC' || asset.id === 'BTCTEST';
  }

  return false;
}
