import { get, writable } from 'svelte/store';
import type { CoinScope } from '$lib/types/wallet.js';

const SCOPE_SELECTION_STORAGE_KEY = 'wallet.coin-scope-selection.v1';

type ScopeSelectionSnapshot = {
  addresses: Record<string, string>;
  systems: Record<string, string>;
};

const EMPTY_SELECTION_SNAPSHOT: ScopeSelectionSnapshot = {
  addresses: {},
  systems: {}
};

function readSelectionSnapshot(): ScopeSelectionSnapshot {
  if (typeof globalThis.localStorage === 'undefined') {
    return EMPTY_SELECTION_SNAPSHOT;
  }

  try {
    const raw = globalThis.localStorage.getItem(SCOPE_SELECTION_STORAGE_KEY);
    if (!raw) return EMPTY_SELECTION_SNAPSHOT;
    const parsed = JSON.parse(raw) as Partial<ScopeSelectionSnapshot>;
    return {
      addresses:
        parsed.addresses && typeof parsed.addresses === 'object'
          ? (parsed.addresses as Record<string, string>)
          : {},
      systems:
        parsed.systems && typeof parsed.systems === 'object'
          ? (parsed.systems as Record<string, string>)
          : {}
    };
  } catch {
    return EMPTY_SELECTION_SNAPSHOT;
  }
}

function persistSelectionSnapshot(snapshot: ScopeSelectionSnapshot): void {
  if (typeof globalThis.localStorage === 'undefined') return;

  try {
    globalThis.localStorage.setItem(SCOPE_SELECTION_STORAGE_KEY, JSON.stringify(snapshot));
  } catch {
    // Ignore persistence failures (private mode / restricted storage).
  }
}

function persistCurrentSelections(): void {
  persistSelectionSnapshot({
    addresses: get(selectedAddressByCoinId),
    systems: get(selectedSystemByCoinId)
  });
}

const persistedSelections = readSelectionSnapshot();

export const scopesByCoinId = writable<Record<string, CoinScope[]>>({});
export const selectedAddressByCoinId = writable<Record<string, string>>(persistedSelections.addresses);
export const selectedSystemByCoinId = writable<Record<string, string>>(persistedSelections.systems);

export function setCoinScopes(coinId: string, scopes: CoinScope[]): void {
  scopesByCoinId.update((state) => ({
    ...state,
    [coinId]: scopes
  }));
}

export function clearCoinScopes(): void {
  scopesByCoinId.set({});
}

export function setSelectedScopeAddress(coinId: string, address: string): void {
  selectedAddressByCoinId.update((state) => ({
    ...state,
    [coinId]: address
  }));
  persistCurrentSelections();
}

export function setSelectedScopeSystem(coinId: string, systemId: string): void {
  selectedSystemByCoinId.update((state) => ({
    ...state,
    [coinId]: systemId
  }));
  persistCurrentSelections();
}
