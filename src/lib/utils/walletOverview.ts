import type { WalletChannelsState } from '$lib/stores/walletChannels.js';
import type { BalanceResult, CoinDefinition, WalletNetwork } from '$lib/types/wallet.js';

export const OVERVIEW_UNAVAILABLE_DISPLAY = '—';
export const OVERVIEW_FIAT_CODE = 'USD';

export interface WalletOverviewRowViewModel {
  key: string;
  coinId: string;
  ticker: string;
  name: string;
  hasBalance: boolean;
  hasSnapshot: boolean;
  cryptoAmountDisplay: string;
  fiatValueDisplay: string;
  unitRateDisplay: string | null;
  fiatSortValue: number;
}

export interface WalletOverviewViewModel {
  heroFiatDisplay: string;
  heroFiatSymbolDisplay: string;
  heroFiatValueDisplay: string;
  heroPrimaryCryptoDisplay: string;
  rows: WalletOverviewRowViewModel[];
  hasUsableLiveData: boolean;
  primaryTicker: string;
}

export interface BuildWalletOverviewParams {
  coins: CoinDefinition[];
  walletChannels: WalletChannelsState;
  balances: Record<string, BalanceResult>;
  rates: Record<string, Record<string, number>>;
  intlLocale: string;
  network?: WalletNetwork;
}

export function formatUsdAmount(value: number, intlLocale: string): string {
  return new Intl.NumberFormat(intlLocale, {
    style: 'currency',
    currency: OVERVIEW_FIAT_CODE,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  }).format(value);
}

export function formatUsdAmountParts(value: number, intlLocale: string): {
  symbol: string;
  value: string;
} {
  const formatter = new Intl.NumberFormat(intlLocale, {
    style: 'currency',
    currency: OVERVIEW_FIAT_CODE,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
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

export function formatCryptoAmount(
  value: number,
  ticker: string,
  intlLocale: string,
  minimumFractionDigits: number,
  maximumFractionDigits: number
): string {
  return `${new Intl.NumberFormat(intlLocale, {
    minimumFractionDigits,
    maximumFractionDigits
  }).format(value)} ${ticker}`;
}

function toFiniteNumber(value: unknown): number | null {
  if (typeof value === 'number') {
    return Number.isFinite(value) ? value : null;
  }

  if (typeof value === 'string') {
    const trimmed = value.trim();
    if (!trimmed) return null;
    const parsed = Number(trimmed);
    return Number.isFinite(parsed) ? parsed : null;
  }

  return null;
}

function hasOwn<K extends string>(source: Record<string, unknown>, key: K): boolean {
  return Object.prototype.hasOwnProperty.call(source, key);
}

function getUsdRate(rateMap?: Record<string, number>): number | null {
  if (!rateMap) return null;
  const candidate = rateMap.USD ?? rateMap.usd;
  if (typeof candidate !== 'number' || !Number.isFinite(candidate)) {
    return null;
  }
  return candidate;
}

function resolvePrimaryCoin(
  coins: CoinDefinition[],
  walletChannels: WalletChannelsState,
  network?: WalletNetwork
): CoinDefinition | null {
  const primaryChannelId = walletChannels.primaryChannelId;
  const primaryCoinIdFromChannel = primaryChannelId
    ? Object.entries(walletChannels.byCoinId).find(([, channel]) => channel === primaryChannelId)?.[0] ?? null
    : null;

  const defaultPrimaryId = network === 'testnet' ? 'VRSCTEST' : 'VRSC';

  return (
    (primaryCoinIdFromChannel ? coins.find((coin) => coin.id === primaryCoinIdFromChannel) : null) ??
    coins.find((coin) => coin.id === defaultPrimaryId) ??
    coins.find((coin) => coin.compatibleChannels.includes('vrpc')) ??
    coins[0] ??
    null
  );
}

export function sortWalletOverviewRows(rows: WalletOverviewRowViewModel[]): WalletOverviewRowViewModel[] {
  return rows.sort((a, b) => {
    if (a.hasBalance !== b.hasBalance) {
      return a.hasBalance ? -1 : 1;
    }

    if (a.hasBalance && b.hasBalance && a.fiatSortValue !== b.fiatSortValue) {
      return b.fiatSortValue - a.fiatSortValue;
    }

    const nameCompare = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
    if (nameCompare !== 0) return nameCompare;
    return a.ticker.localeCompare(b.ticker, undefined, { sensitivity: 'base' });
  });
}

export function buildWalletOverviewViewModel({
  coins,
  walletChannels,
  balances,
  rates,
  intlLocale,
  network
}: BuildWalletOverviewParams): WalletOverviewViewModel {
  const primaryCoin = resolvePrimaryCoin(coins, walletChannels, network);
  const fallbackPrimaryTicker = network === 'testnet' ? 'VRSCTEST' : 'VRSC';
  const primaryTicker = primaryCoin?.displayTicker ?? fallbackPrimaryTicker;
  const primaryChannelId =
    (primaryCoin ? walletChannels.byCoinId[primaryCoin.id] : null) ?? walletChannels.primaryChannelId;
  const hasPrimarySnapshot = primaryChannelId ? hasOwn(balances, primaryChannelId) : false;
  const primaryTotal = hasPrimarySnapshot && primaryChannelId ? toFiniteNumber(balances[primaryChannelId]?.total) : null;

  const rows = coins.map<WalletOverviewRowViewModel>((coin) => {
    const channelId = walletChannels.byCoinId[coin.id];
    const hasSnapshot = channelId ? hasOwn(balances, channelId) : false;
    const totalAmount = hasSnapshot && channelId ? toFiniteNumber(balances[channelId]?.total) : null;
    const amountValue = totalAmount ?? 0;
    const hasBalance = hasSnapshot && amountValue > 0;
    const usdRate = getUsdRate(rates[coin.id]);
    const fiatValue = hasSnapshot && usdRate !== null ? amountValue * usdRate : null;
    const rowFractionDigits = Math.max(0, Math.min(4, coin.decimals));

    return {
      key: coin.id,
      coinId: coin.id,
      ticker: coin.displayTicker,
      name: coin.displayName,
      hasBalance,
      hasSnapshot,
      cryptoAmountDisplay: hasSnapshot
        ? formatCryptoAmount(amountValue, coin.displayTicker, intlLocale, rowFractionDigits, rowFractionDigits)
        : `${OVERVIEW_UNAVAILABLE_DISPLAY} ${coin.displayTicker}`,
      fiatValueDisplay:
        fiatValue === null ? OVERVIEW_UNAVAILABLE_DISPLAY : formatUsdAmount(fiatValue, intlLocale),
      unitRateDisplay: usdRate === null ? null : formatUsdAmount(usdRate, intlLocale),
      fiatSortValue: hasBalance && fiatValue !== null ? fiatValue : Number.NEGATIVE_INFINITY
    };
  });

  sortWalletOverviewRows(rows);

  const hasNonZeroRows = rows.some((row) => row.hasBalance);
  const hasAnySnapshot = rows.some((row) => row.hasSnapshot) || hasPrimarySnapshot;
  const hasMissingFiatForHoldings = rows.some(
    (row) => row.hasBalance && row.fiatSortValue === Number.NEGATIVE_INFINITY
  );
  const totalFiat = rows
    .filter((row) => row.hasBalance && row.fiatSortValue !== Number.NEGATIVE_INFINITY)
    .reduce((sum, row) => sum + row.fiatSortValue, 0);
  const heroFiatIsUnavailable = !hasAnySnapshot || hasMissingFiatForHoldings;
  const heroFiatDisplay = heroFiatIsUnavailable
    ? OVERVIEW_UNAVAILABLE_DISPLAY
    : formatUsdAmount(totalFiat, intlLocale);
  const heroFiatParts = heroFiatIsUnavailable
    ? null
    : formatUsdAmountParts(totalFiat, intlLocale);

  return {
    heroFiatDisplay,
    heroFiatSymbolDisplay: heroFiatParts?.symbol ?? '',
    heroFiatValueDisplay: heroFiatParts?.value ?? OVERVIEW_UNAVAILABLE_DISPLAY,
    heroPrimaryCryptoDisplay:
      primaryTotal === null
        ? `${OVERVIEW_UNAVAILABLE_DISPLAY} ${primaryTicker}`
        : formatCryptoAmount(primaryTotal, primaryTicker, intlLocale, 0, 4),
    rows,
    hasUsableLiveData: hasNonZeroRows || hasPrimarySnapshot,
    primaryTicker
  };
}
