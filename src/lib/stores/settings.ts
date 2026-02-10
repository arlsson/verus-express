/**
 * Settings store: minimal placeholder (e.g. theme). Expand later.
 */

import { writable } from 'svelte/store';

export interface Settings {
  theme?: 'light' | 'dark' | 'system';
}

const initialState: Settings = {};

export const settingsStore = writable<Settings>(initialState);
