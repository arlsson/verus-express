import type { AddressBookContact, AddressBookEndpoint, AddressEndpointKind } from '$lib/types/addressBook';
import type { DestinationAddressKind } from '$lib/components/wallet/sections/transfer-wizard/types';
import {
  classifyDlightDestinationAddress,
  isBitcoinAddress,
  isDlightShieldedAddress,
  isEthereumAddress,
  isVrpcHandleAddress,
  isVrpcTransparentAddress,
} from '$lib/transfer/recipientAddressValidation';

export function endpointKindForDestinationKind(kind: DestinationAddressKind): AddressEndpointKind {
  if (kind === 'btc') return 'btc';
  if (kind === 'eth') return 'eth';
  if (kind === 'dlight') return 'zs';
  return 'vrpc';
}

export function endpointKindsForDestinationKind(kind: DestinationAddressKind): AddressEndpointKind[] {
  if (kind === 'dlight') return ['zs', 'vrpc'];
  return [endpointKindForDestinationKind(kind)];
}

export function normalizeAddressByKind(kind: AddressEndpointKind, address: string): string {
  const trimmed = address.trim();
  if (!trimmed) return '';

  if (kind === 'eth') {
    return trimmed.toLowerCase();
  }

  if (kind === 'btc' && /^(bc1|tb1)/i.test(trimmed)) {
    return trimmed.toLowerCase();
  }

  if (kind === 'vrpc' && trimmed.endsWith('@')) {
    const name = trimmed.slice(0, -1).toLowerCase();
    return `${name}@`;
  }

  if (kind === 'zs') {
    return trimmed.toLowerCase();
  }

  return trimmed;
}

export function inferEndpointKindForDestinationAddress(
  destinationKind: DestinationAddressKind,
  address: string
): AddressEndpointKind | null {
  const input = address.trim();
  if (!input) return null;

  if (destinationKind === 'eth') return isEthereumAddress(input) ? 'eth' : null;
  if (destinationKind === 'btc') return isBitcoinAddress(input) ? 'btc' : null;
  if (destinationKind === 'vrpc') {
    return isVrpcTransparentAddress(input) || isVrpcHandleAddress(input) ? 'vrpc' : null;
  }

  const dlightKind = classifyDlightDestinationAddress(input);
  if (dlightKind === 'shielded') return 'zs';
  if (dlightKind === 'transparent') return 'vrpc';
  return null;
}

export function normalizeAddressByDestinationKind(
  destinationKind: DestinationAddressKind,
  address: string
): string {
  const endpointKind = inferEndpointKindForDestinationAddress(destinationKind, address);
  if (!endpointKind) return '';
  return normalizeAddressByKind(endpointKind, address);
}

export function isEndpointCompatibleWithDestinationKind(
  endpoint: AddressBookEndpoint,
  destinationKind: DestinationAddressKind
): boolean {
  if (destinationKind === 'dlight') {
    if (endpoint.kind === 'zs') return isDlightShieldedAddress(endpoint.address);
    if (endpoint.kind === 'vrpc') return isVrpcTransparentAddress(endpoint.address);
    return false;
  }

  return endpoint.kind === endpointKindForDestinationKind(destinationKind);
}

export function endpointsForKind(
  contacts: AddressBookContact[],
  kind: AddressEndpointKind
): Array<{ contact: AddressBookContact; endpoint: AddressBookEndpoint }> {
  return contacts.flatMap((contact) =>
    contact.endpoints
      .filter((endpoint) => endpoint.kind === kind)
      .map((endpoint) => ({ contact, endpoint }))
  );
}

export function endpointsForDestinationKind(
  contacts: AddressBookContact[],
  destinationKind: DestinationAddressKind
): Array<{ contact: AddressBookContact; endpoint: AddressBookEndpoint }> {
  return contacts.flatMap((contact) =>
    contact.endpoints
      .filter((endpoint) => isEndpointCompatibleWithDestinationKind(endpoint, destinationKind))
      .map((endpoint) => ({ contact, endpoint }))
  );
}

export function findMatchingSavedEndpoint(
  contacts: AddressBookContact[],
  destinationKind: DestinationAddressKind,
  address: string
): { contact: AddressBookContact; endpoint: AddressBookEndpoint } | null {
  const endpointKind = inferEndpointKindForDestinationAddress(destinationKind, address);
  if (!endpointKind) return null;
  const normalized = normalizeAddressByKind(endpointKind, address);
  if (!normalized) return null;

  const matches = endpointsForDestinationKind(contacts, destinationKind).filter(
    ({ endpoint }) => endpoint.kind === endpointKind
  );
  return matches.find(({ endpoint }) => normalizeAddressByKind(endpoint.kind, endpoint.normalizedAddress) === normalized) ?? null;
}

export function sharesSuspiciousPrefixSuffix(
  contacts: AddressBookContact[],
  destinationKind: DestinationAddressKind,
  address: string
): boolean {
  const endpointKind = inferEndpointKindForDestinationAddress(destinationKind, address);
  if (!endpointKind) return false;
  const normalized = normalizeAddressByKind(endpointKind, address);
  if (!normalized || normalized.length < 12) return false;

  const prefix = normalized.slice(0, 6);
  const suffix = normalized.slice(-6);
  const sameKindEndpoints = endpointsForDestinationKind(contacts, destinationKind).filter(
    ({ endpoint }) => endpoint.kind === endpointKind
  );

  return sameKindEndpoints.some(({ endpoint }) => {
    const endpointNormalized = normalizeAddressByKind(endpoint.kind, endpoint.normalizedAddress);
    if (endpointNormalized === normalized) return false;
    return (
      endpointNormalized.length >= 12 &&
      endpointNormalized.startsWith(prefix) &&
      endpointNormalized.endsWith(suffix)
    );
  });
}
