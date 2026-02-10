/**
 * Non-blocking wallet error state used for update/polling visibility.
 */

import { writable } from 'svelte/store';

export interface WalletErrorsState {
  latest: string | null;
  history: string[];
}

const initialState: WalletErrorsState = {
  latest: null,
  history: []
};
const DEDUPE_WINDOW_MS = 10_000;
const SEEN_RETENTION_MS = DEDUPE_WINDOW_MS * 6;
const recentByMessage = new Map<string, number>();

export const walletErrorsStore = writable<WalletErrorsState>(initialState);

function normalizeMessage(message: string): string {
  return message.trim().replace(/\s+/g, ' ');
}

function seenRecently(message: string, now: number): boolean {
  const last = recentByMessage.get(message);
  return last !== undefined && now - last < DEDUPE_WINDOW_MS;
}

function rememberMessage(message: string, now: number): void {
  recentByMessage.set(message, now);
  for (const [key, timestamp] of recentByMessage.entries()) {
    if (now - timestamp > SEEN_RETENTION_MS) {
      recentByMessage.delete(key);
    }
  }
}

export function pushWalletError(message: string): void {
  const text = normalizeMessage(message);
  if (!text) return;
  const now = Date.now();
  if (seenRecently(text, now)) return;
  rememberMessage(text, now);

  walletErrorsStore.update((s) => {
    if (s.latest === text) return s;
    return {
      latest: text,
      history: [text, ...s.history].slice(0, 20)
    };
  });
}

export function clearWalletErrors(): void {
  recentByMessage.clear();
  walletErrorsStore.set(initialState);
}

export function dismissWalletError(): void {
  walletErrorsStore.update((s) => ({ ...s, latest: null }));
}
