import type { DestinationAddressKind } from '$lib/components/wallet/sections/transfer-wizard/types';

const ETH_ADDRESS_PATTERN = /^0x[a-fA-F0-9]{40}$/;
const BTC_BECH32_ADDRESS_PATTERN = /^(bc1|tb1)[ac-hj-np-z02-9]{11,71}$/i;
const BTC_BASE58_ADDRESS_PATTERN = /^[13mn2][a-km-zA-HJ-NP-Z1-9]{25,39}$/;
const VRPC_HANDLE_PATTERN = /^[A-Za-z0-9._-]+@$/;
const VRPC_TRANSPARENT_PATTERN = /^[Ri][a-km-zA-HJ-NP-Z1-9]{24,60}$/;
const SAPLING_PATTERN = /^zs[0-9a-z]{60,140}$/i;

export type DlightDestinationKind = 'shielded' | 'transparent';

export function isEthereumAddress(value: string): boolean {
  return ETH_ADDRESS_PATTERN.test(value.trim());
}

export function isBitcoinAddress(value: string): boolean {
  const input = value.trim();
  return BTC_BECH32_ADDRESS_PATTERN.test(input) || BTC_BASE58_ADDRESS_PATTERN.test(input);
}

export function isVrpcHandleAddress(value: string): boolean {
  return VRPC_HANDLE_PATTERN.test(value.trim());
}

export function isVrpcTransparentAddress(value: string): boolean {
  return VRPC_TRANSPARENT_PATTERN.test(value.trim());
}

export function isDlightShieldedAddress(value: string): boolean {
  return SAPLING_PATTERN.test(value.trim());
}

export function classifyDlightDestinationAddress(value: string): DlightDestinationKind | null {
  const input = value.trim();
  if (!input) return null;
  if (isDlightShieldedAddress(input)) return 'shielded';
  if (isVrpcTransparentAddress(input)) return 'transparent';
  return null;
}

export function validateDestinationAddressForKind(
  value: string,
  kind: DestinationAddressKind
): boolean {
  const input = value.trim();
  if (!input) return false;

  if (kind === 'eth') return isEthereumAddress(input);
  if (kind === 'btc') return isBitcoinAddress(input);
  if (kind === 'dlight') return classifyDlightDestinationAddress(input) !== null;
  // VRPC routes can still target shielded recipients on Verus.
  return isVrpcHandleAddress(input) || isVrpcTransparentAddress(input) || isDlightShieldedAddress(input);
}
