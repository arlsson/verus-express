export type AddressEndpointKind = 'vrpc' | 'btc' | 'eth';

export type AddressBookEndpoint = {
  id: string;
  kind: AddressEndpointKind;
  address: string;
  normalizedAddress: string;
  label: string;
  lastUsedAt: number | null;
  createdAt: number;
  updatedAt: number;
};

export type AddressBookContact = {
  id: string;
  displayName: string;
  note: string | null;
  createdAt: number;
  updatedAt: number;
  endpoints: AddressBookEndpoint[];
};

export type SaveAddressBookEndpointInput = {
  id?: string;
  kind: AddressEndpointKind;
  address: string;
  label: string;
};

export type SaveAddressBookContactRequest = {
  id?: string;
  displayName: string;
  note?: string | null;
  endpoints: SaveAddressBookEndpointInput[];
};

export type ValidateDestinationAddressRequest = {
  kind: AddressEndpointKind;
  address: string;
};

export type ValidateDestinationAddressResult = {
  valid: boolean;
  normalizedAddress: string | null;
  reason: string | null;
};

