/**
 * Settings store with local persistence for app-wide preferences.
 */

import { writable } from 'svelte/store';
import {
  DEFAULT_DISPLAY_CURRENCY,
  normalizeDisplayCurrency
} from '$lib/utils/fiatDisplay.js';
import {
  type AutoLockMinutes,
  DEFAULT_AUTO_LOCK_MINUTES,
  normalizeAutoLockMinutes
} from '$lib/security/sessionTimeout.js';

export interface Settings {
  theme?: 'light' | 'dark' | 'system';
  displayCurrency: string;
  autoLockMinutes: AutoLockMinutes;
}

const SETTINGS_STORAGE_KEY = 'lite_wallet_settings_v1';

const initialState: Settings = {
  displayCurrency: DEFAULT_DISPLAY_CURRENCY,
  autoLockMinutes: DEFAULT_AUTO_LOCK_MINUTES
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
      displayCurrency: normalizeDisplayCurrency(parsed?.displayCurrency),
      autoLockMinutes: normalizeAutoLockMinutes(parsed?.autoLockMinutes)
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
    displayCurrency: normalizeDisplayCurrency(settings.displayCurrency),
    autoLockMinutes: normalizeAutoLockMinutes(settings.autoLockMinutes)
  });
});

export function setDisplayCurrency(code: string): void {
  settingsStore.update((settings) => ({
    ...settings,
    displayCurrency: normalizeDisplayCurrency(code)
  }));
}

export function setAutoLockMinutes(minutes: unknown): void {
  settingsStore.update((settings) => ({
    ...settings,
    autoLockMinutes: normalizeAutoLockMinutes(minutes)
  }));
}

export function resetSettings(): void {
  settingsStore.set(initialState);
}
