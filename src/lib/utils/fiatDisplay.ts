import verusCoinCatalog from '$lib/coins/verusCoinCatalog.generated.json';

type CatalogIcon =
  | {
      kind: 'fiat-symbol';
      symbol?: string;
    }
  | {
      kind: string;
      symbol?: string;
    };

type CatalogEntry = {
  id: string;
  displayName: string;
  proto: string;
  icon?: CatalogIcon;
};

export interface FiatCurrencyOption {
  code: string;
  name: string;
  symbol: string;
}

export const DEFAULT_DISPLAY_CURRENCY = 'USD';
export const QUICK_PICK_DISPLAY_CURRENCIES = ['USD', 'EUR', 'GBP'] as const;

const catalogEntries = verusCoinCatalog as CatalogEntry[];

const fiatCurrencyOptions: FiatCurrencyOption[] = (() => {
  const byCode = new Map<string, FiatCurrencyOption>();

  for (const entry of catalogEntries) {
    if (entry.proto !== 'fiat') continue;
    const code = entry.id.trim().toUpperCase();
    if (!code) continue;

    const symbol =
      entry.icon?.kind === 'fiat-symbol' && typeof entry.icon.symbol === 'string'
        ? entry.icon.symbol
        : code.slice(0, 1);

    byCode.set(code, {
      code,
      name: entry.displayName.trim() || code,
      symbol
    });
  }

  return Array.from(byCode.values()).sort((left, right) =>
    left.code.localeCompare(right.code, undefined, { sensitivity: 'base' })
  );
})();

const supportedCodes = new Set(fiatCurrencyOptions.map((option) => option.code));

export function getFiatCurrencyOptions(): FiatCurrencyOption[] {
  return fiatCurrencyOptions;
}

export function normalizeDisplayCurrency(value: string | null | undefined): string {
  if (typeof value !== 'string') return DEFAULT_DISPLAY_CURRENCY;
  const normalized = value.trim().toUpperCase();
  if (!normalized) return DEFAULT_DISPLAY_CURRENCY;
  return supportedCodes.has(normalized) ? normalized : DEFAULT_DISPLAY_CURRENCY;
}

export function filterFiatCurrencyOptions(
  options: FiatCurrencyOption[],
  query: string
): FiatCurrencyOption[] {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) return options;

  return options.filter((option) => {
    const code = option.code.toLowerCase();
    const name = option.name.toLowerCase();
    return code.includes(normalizedQuery) || name.includes(normalizedQuery);
  });
}

export function getRateForCurrency(
  rateMap: Record<string, number> | undefined,
  currencyCode: string
): number | null {
  if (!rateMap) return null;

  const normalizedCurrency = currencyCode.trim().toUpperCase();
  if (!normalizedCurrency) return null;

  const direct = rateMap[normalizedCurrency];
  if (typeof direct === 'number' && Number.isFinite(direct) && direct > 0) {
    return direct;
  }

  const lower = rateMap[normalizedCurrency.toLowerCase()];
  if (typeof lower === 'number' && Number.isFinite(lower) && lower > 0) {
    return lower;
  }

  return null;
}

type FormatOptions = {
  minimumFractionDigits?: number;
  maximumFractionDigits?: number;
};

export function formatFiatAmount(
  value: number,
  intlLocale: string,
  currencyCode: string,
  options: FormatOptions = {}
): string {
  const normalizedCurrency = normalizeDisplayCurrency(currencyCode);

  try {
    return new Intl.NumberFormat(intlLocale, {
      style: 'currency',
      currency: normalizedCurrency,
      minimumFractionDigits: options.minimumFractionDigits ?? 2,
      maximumFractionDigits: options.maximumFractionDigits ?? 2
    }).format(value);
  } catch {
    return `${value.toFixed(2)} ${normalizedCurrency}`;
  }
}

export function formatFiatAmountParts(
  value: number,
  intlLocale: string,
  currencyCode: string,
  options: FormatOptions = {}
): { symbol: string; value: string } {
  const normalizedCurrency = normalizeDisplayCurrency(currencyCode);
  const formatter = new Intl.NumberFormat(intlLocale, {
    style: 'currency',
    currency: normalizedCurrency,
    minimumFractionDigits: options.minimumFractionDigits ?? 2,
    maximumFractionDigits: options.maximumFractionDigits ?? 2
  });
  const parts = formatter.formatToParts(value);
  const symbol = parts
    .filter((part) => part.type === 'currency')
    .map((part) => part.value)
    .join('')
    .trim();
  const numericValue = parts
    .filter((part) => part.type !== 'currency')
    .map((part) => part.value)
    .join('')
    .trim();

  return {
    symbol,
    value: numericValue || formatter.format(value)
  };
}
