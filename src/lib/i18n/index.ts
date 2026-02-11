import { derived, writable } from 'svelte/store';
import { en } from './locales/en';
import { nl } from './locales/nl';

export type Locale = 'en' | 'nl';

export type TranslationParams = Record<string, string | number>;

type Dictionary = Record<string, string>;

const DEFAULT_LOCALE: Locale = 'en';
const dictionaries: Record<Locale, Dictionary> = {
  en,
  nl
};

function toSupportedLocale(input?: string | null): Locale {
  if (!input) return DEFAULT_LOCALE;
  const normalized = input.toLowerCase();
  if (normalized.startsWith('nl')) return 'nl';
  return 'en';
}

function interpolate(template: string, params?: TranslationParams): string {
  if (!params) return template;
  return template.replace(/\{([a-zA-Z0-9_]+)\}/g, (_match, token: string) => {
    const value = params[token];
    return value === undefined ? `{${token}}` : String(value);
  });
}

function translate(locale: Locale, key: string, params?: TranslationParams): string {
  const primary = dictionaries[locale][key];
  const fallback = dictionaries.en[key];
  const message = primary ?? fallback ?? key;
  return interpolate(message, params);
}

export const localeStore = writable<Locale>(DEFAULT_LOCALE);

export const i18nStore = derived(localeStore, ($locale) => {
  const intlLocale = $locale === 'nl' ? 'nl-NL' : 'en-US';

  return {
    locale: $locale,
    intlLocale,
    t: (key: string, params?: TranslationParams): string => translate($locale, key, params),
    formatNumber: (value: number, options?: Intl.NumberFormatOptions): string =>
      new Intl.NumberFormat(intlLocale, options).format(value),
    formatDate: (value: number | Date, options?: Intl.DateTimeFormatOptions): string => {
      const date = value instanceof Date ? value : new Date(value);
      return new Intl.DateTimeFormat(intlLocale, options).format(date);
    }
  };
});

function detectLocaleFromNavigator(): Locale {
  if (typeof globalThis.navigator === 'undefined') return DEFAULT_LOCALE;
  const candidates =
    Array.isArray(globalThis.navigator.languages) && globalThis.navigator.languages.length > 0
      ? globalThis.navigator.languages
      : [globalThis.navigator.language];

  for (const candidate of candidates) {
    const supported = toSupportedLocale(candidate);
    if (supported in dictionaries) return supported;
  }

  return DEFAULT_LOCALE;
}

function applyDocumentLang(locale: Locale): void {
  if (typeof globalThis.document === 'undefined') return;
  globalThis.document.documentElement.lang = locale;
}

localeStore.subscribe((locale) => {
  applyDocumentLang(locale);
});

export function setLocale(locale: Locale | string): void {
  localeStore.set(toSupportedLocale(locale));
}

export function initI18n(): void {
  localeStore.set(detectLocaleFromNavigator());
}

export function networkLocaleKey(network?: 'mainnet' | 'testnet'): 'common.network.mainnet' | 'common.network.testnet' {
  return network === 'testnet' ? 'common.network.testnet' : 'common.network.mainnet';
}
