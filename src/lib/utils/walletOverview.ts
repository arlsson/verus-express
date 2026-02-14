import type { WalletChannelsState } from '$lib/stores/walletChannels.js';
import type { CoinRatesSnapshot } from '$lib/stores/rates.js';
import type { BalanceResult, CoinDefinition, WalletNetwork } from '$lib/types/wallet.js';
import { resolveCoinPresentation } from '$lib/coins/presentation.js';

export const OVERVIEW_UNAVAILABLE_DISPLAY = '—';
export const OVERVIEW_FIAT_CODE = 'USD';

export interface WalletOverviewRowViewModel {
  key: string;
  coinId: string;
  proto: CoinDefinition['proto'];
  ticker: string;
  name: string;
  hasBalance: boolean;
  hasSnapshot: boolean;
  cryptoAmountDisplay: string;
  fiatValueDisplay: string;
  marketPriceDisplay: string;
  change24hDisplay: string;
  change24hDirection: 'up' | 'down' | 'flat' | 'none';
  unitRateDisplay: string | null;
  fiatSortValue: number;
}

export interface WalletOverviewViewModel {
  heroFiatDisplay: string;
  heroFiatSymbolDisplay: string;
  heroFiatValueDisplay: string;
  heroPrimaryCryptoDisplay: string;
  assetCount: number;
  identityCount: number;
  rows: WalletOverviewRowViewModel[];
  hasUsableLiveData: boolean;
  primaryTicker: string;
}

export interface BuildWalletOverviewParams {
  coins: CoinDefinition[];
  walletChannels: WalletChannelsState;
  balances: Record<string, Record<string, BalanceResult>>;
  rates: Record<string, CoinRatesSnapshot>;
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

function getUsdRate(rateMap?: Record<string, number>): number | null {
  if (!rateMap) return null;
  const candidate = rateMap.USD ?? rateMap.usd;
  if (typeof candidate !== 'number' || !Number.isFinite(candidate)) {
    return null;
  }
  return candidate;
}

function getChangeDirection(changePct: number | null): WalletOverviewRowViewModel['change24hDirection'] {
  if (changePct === null) return 'none';
  if (Math.abs(changePct) < 0.01) return 'flat';
  if (changePct > 0) return 'up';
  return 'down';
}

function formatPercentChange(changePct: number, intlLocale: string): string {
  const formatter = new Intl.NumberFormat(intlLocale, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
  const absDisplay = formatter.format(Math.abs(changePct));

  if (changePct > 0) return `+${absDisplay}%`;
  if (changePct < 0) return `-${absDisplay}%`;
  return `${absDisplay}%`;
}

function resolvePrimaryCoin(
  coins: CoinDefinition[],
  walletChannels: WalletChannelsState,
  network?: WalletNetwork
): CoinDefinition | null {
  const primaryChannelId = walletChannels.primaryChannelId;
  const matchingPrimaryCoinIds = primaryChannelId
    ? Object.entries(walletChannels.byCoinId)
        .filter(([, channel]) => channel === primaryChannelId)
        .map(([coinId]) => coinId)
    : [];
  const primaryCoinIdFromChannel =
    matchingPrimaryCoinIds.length === 1 ? matchingPrimaryCoinIds[0] : null;

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
  const primaryPresentation = primaryCoin ? resolveCoinPresentation(primaryCoin) : null;
  const fallbackPrimaryTicker = network === 'testnet' ? 'VRSCTEST' : 'VRSC';
  const primaryTicker = primaryPresentation?.displayTicker ?? fallbackPrimaryTicker;
  const primaryChannelId =
    (primaryCoin ? walletChannels.byCoinId[primaryCoin.id] : null) ?? walletChannels.primaryChannelId;
  const primarySnapshot =
    primaryCoin && primaryChannelId ? balances[primaryChannelId]?.[primaryCoin.id] : undefined;
  const hasPrimarySnapshot = primarySnapshot !== undefined;
  const primaryTotal = hasPrimarySnapshot ? toFiniteNumber(primarySnapshot?.total) : null;

  const rows = coins.map<WalletOverviewRowViewModel>((coin) => {
    const coinPresentation = resolveCoinPresentation(coin);
    const displayTicker = coinPresentation.displayTicker;
    const displayName = coinPresentation.displayName;
    const channelId = walletChannels.byCoinId[coin.id];
    const balanceSnapshot = channelId ? balances[channelId]?.[coin.id] : undefined;
    const hasSnapshot = balanceSnapshot !== undefined;
    const totalAmount = hasSnapshot ? toFiniteNumber(balanceSnapshot?.total) : null;
    const amountValue = totalAmount ?? 0;
    const hasBalance = hasSnapshot && amountValue > 0;
    const rateSnapshot = rates[coin.id];
    const usdRate = getUsdRate(rateSnapshot?.rates);
    const rawChange = rateSnapshot?.usdChange24hPct;
    const change24hPct =
      typeof rawChange === 'number' && Number.isFinite(rawChange) ? rawChange : null;
    const change24hDirection = getChangeDirection(change24hPct);
    const fiatValue = hasSnapshot && usdRate !== null ? amountValue * usdRate : null;
    const rowFractionDigits = Math.max(0, Math.min(4, coin.decimals));
    const marketPriceDisplay =
      usdRate === null ? OVERVIEW_UNAVAILABLE_DISPLAY : formatUsdAmount(usdRate, intlLocale);
    const change24hDisplay =
      change24hDirection === 'none' || change24hPct === null
        ? OVERVIEW_UNAVAILABLE_DISPLAY
        : formatPercentChange(change24hPct, intlLocale);

    return {
      key: coin.id,
      coinId: coin.id,
      proto: coin.proto,
      ticker: displayTicker,
      name: displayName,
      hasBalance,
      hasSnapshot,
      cryptoAmountDisplay: hasSnapshot
        ? formatCryptoAmount(amountValue, displayTicker, intlLocale, rowFractionDigits, rowFractionDigits)
        : `${OVERVIEW_UNAVAILABLE_DISPLAY} ${displayTicker}`,
      fiatValueDisplay:
        fiatValue === null ? OVERVIEW_UNAVAILABLE_DISPLAY : formatUsdAmount(fiatValue, intlLocale),
      marketPriceDisplay,
      change24hDisplay,
      change24hDirection,
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
    assetCount: rows.length,
    identityCount: 0,
    rows,
    hasUsableLiveData: hasNonZeroRows || hasPrimarySnapshot,
    primaryTicker
  };
}
