import { SUPPORTED_LOCALES } from '$lib/i18n';
import type { Locale } from '$lib/i18n';

type TranslateFn = (key: string) => string;

export interface LocaleOption {
  value: Locale;
  flag: string;
  label: string;
}

const localeMetadata: Record<Locale, { flag: string; labelKey: string }> = {
  en: { flag: '🇺🇸', labelKey: 'languageGate.option.en' },
  nl: { flag: '🇳🇱', labelKey: 'languageGate.option.nl' }
};

export function buildLocaleOptions(t: TranslateFn): LocaleOption[] {
  return SUPPORTED_LOCALES.map((locale) => ({
    value: locale,
    flag: localeMetadata[locale].flag,
    label: t(localeMetadata[locale].labelKey)
  }));
}
