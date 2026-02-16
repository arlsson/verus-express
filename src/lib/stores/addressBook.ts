import { writable } from 'svelte/store';
import type { AddressBookContact } from '$lib/types/addressBook';

export const addressBookStore = writable<AddressBookContact[]>([]);

export function setAddressBookContacts(contacts: AddressBookContact[]): void {
  addressBookStore.set(contacts);
}

export function upsertAddressBookContact(contact: AddressBookContact): void {
  addressBookStore.update((contacts) => {
    const index = contacts.findIndex((existing) => existing.id === contact.id);
    if (index === -1) {
      return [...contacts, contact];
    }

    const next = [...contacts];
    next[index] = contact;
    return next;
  });
}

export function removeAddressBookContact(contactId: string): void {
  addressBookStore.update((contacts) => contacts.filter((contact) => contact.id !== contactId));
}

