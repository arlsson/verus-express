/**
 * Settings store with local persistence for app-wide preferences.
 */

import { writable } from 'svelte/store';
import {
  DEFAULT_DISPLAY_CURRENCY,
  normalizeDisplayCurrency
} from '$lib/utils/fiatDisplay.js';

export interface Settings {
  theme?: 'light' | 'dark' | 'system';
  displayCurrency: string;
}

const SETTINGS_STORAGE_KEY = 'lite_wallet_settings_v1';

const initialState: Settings = {
  displayCurrency: DEFAULT_DISPLAY_CURRENCY
};

function canUseStorage(): boolean {
  return typeof globalThis.localStorage !== 'undefined';
}

function readStoredSettings(): Settings {
  if (!canUseStorage()) return initialState;

  try {
    const raw = globalThis.localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (!raw) return initialState;

    const parsed = JSON.parse(raw) as Partial<Settings> | null;
    return {
      theme:
        parsed?.theme === 'light' || parsed?.theme === 'dark' || parsed?.theme === 'system'
          ? parsed.theme
          : undefined,
      displayCurrency: normalizeDisplayCurrency(parsed?.displayCurrency)
    };
  } catch {
    return initialState;
  }
}

function persistSettings(settings: Settings): void {
  if (!canUseStorage()) return;

  try {
    globalThis.localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Ignore persistence failures (private mode / restricted storage).
  }
}

export const settingsStore = writable<Settings>(readStoredSettings());

settingsStore.subscribe((settings) => {
  persistSettings({
    theme: settings.theme,
    displayCurrency: normalizeDisplayCurrency(settings.displayCurrency)
  });
});

export function setDisplayCurrency(code: string): void {
  settingsStore.update((settings) => ({
    ...settings,
    displayCurrency: normalizeDisplayCurrency(code)
  }));
}

export function resetSettings(): void {
  settingsStore.set(initialState);
}
