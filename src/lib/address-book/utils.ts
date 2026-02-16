import type { AddressBookContact, AddressBookEndpoint, AddressEndpointKind } from '$lib/types/addressBook';
import type { DestinationAddressKind } from '$lib/components/wallet/sections/transfer-wizard/types';

export function endpointKindForDestinationKind(kind: DestinationAddressKind): AddressEndpointKind {
  if (kind === 'btc') return 'btc';
  if (kind === 'eth') return 'eth';
  return 'vrpc';
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

  return trimmed;
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

export function findMatchingSavedEndpoint(
  contacts: AddressBookContact[],
  kind: AddressEndpointKind,
  address: string
): { contact: AddressBookContact; endpoint: AddressBookEndpoint } | null {
  const normalized = normalizeAddressByKind(kind, address);
  if (!normalized) return null;

  const matches = endpointsForKind(contacts, kind);
  return matches.find(({ endpoint }) => endpoint.normalizedAddress === normalized) ?? null;
}

export function sharesSuspiciousPrefixSuffix(
  contacts: AddressBookContact[],
  kind: AddressEndpointKind,
  address: string
): boolean {
  const normalized = normalizeAddressByKind(kind, address);
  if (!normalized || normalized.length < 12) return false;

  const prefix = normalized.slice(0, 6);
  const suffix = normalized.slice(-6);
  const sameKindEndpoints = endpointsForKind(contacts, kind);

  return sameKindEndpoints.some(({ endpoint }) => {
    if (endpoint.normalizedAddress === normalized) return false;
    return (
      endpoint.normalizedAddress.length >= 12 &&
      endpoint.normalizedAddress.startsWith(prefix) &&
      endpoint.normalizedAddress.endsWith(suffix)
    );
  });
}
