import { invoke } from '@tauri-apps/api/core';
import type {
  AddressBookContact,
  SaveAddressBookContactRequest,
  ValidateDestinationAddressRequest,
  ValidateDestinationAddressResult
} from '$lib/types/addressBook';

export async function listAddressBookContacts(): Promise<AddressBookContact[]> {
  return invoke<AddressBookContact[]>('list_address_book_contacts');
}

export async function saveAddressBookContact(
  request: SaveAddressBookContactRequest
): Promise<AddressBookContact> {
  return invoke<AddressBookContact>('save_address_book_contact', { request });
}

export async function deleteAddressBookContact(contactId: string): Promise<boolean> {
  return invoke<boolean>('delete_address_book_contact', { contact_id: contactId });
}

export async function markAddressBookEndpointUsed(endpointId: string): Promise<boolean> {
  return invoke<boolean>('mark_address_book_endpoint_used', { endpoint_id: endpointId });
}

export async function validateDestinationAddress(
  request: ValidateDestinationAddressRequest
): Promise<ValidateDestinationAddressResult> {
  return invoke<ValidateDestinationAddressResult>('validate_destination_address', { request });
}
