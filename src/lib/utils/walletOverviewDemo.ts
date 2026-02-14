import type { WalletNetwork } from '$lib/types/wallet.js';
import {
  formatCryptoAmount,
  formatUsdAmount,
  formatUsdAmountParts,
  sortWalletOverviewRows,
  type WalletOverviewRowViewModel,
  type WalletOverviewViewModel
} from '$lib/utils/walletOverview.js';
import { resolveCoinPresentationById } from '$lib/coins/presentation.js';

interface DemoRowSeed {
  coinId: string;
  proto: WalletOverviewRowViewModel['proto'];
  fallbackTicker: string;
  fallbackName: string;
  amount: number;
  fiatValue: number;
  unitRate: number;
}

const MAINNET_ROWS: DemoRowSeed[] = [
  {
    coinId: 'VRSC',
    proto: 'vrsc',
    fallbackTicker: 'VRSC',
    fallbackName: 'Verus',
    amount: 2784.2191,
    fiatValue: 6431.55,
    unitRate: 2.31
  },
  {
    coinId: 'ETH',
    proto: 'eth',
    fallbackTicker: 'ETH',
    fallbackName: 'Ethereum',
    amount: 1.2408,
    fiatValue: 4131.86,
    unitRate: 3330
  },
  {
    coinId: 'BTC',
    proto: 'btc',
    fallbackTicker: 'BTC',
    fallbackName: 'Bitcoin',
    amount: 0.0314,
    fiatValue: 2006.46,
    unitRate: 63900
  },
  {
    coinId: 'USDC',
    proto: 'erc20',
    fallbackTicker: 'USDC',
    fallbackName: 'USD Coin',
    amount: 0,
    fiatValue: 0,
    unitRate: 1
  }
];

const TESTNET_ROWS: DemoRowSeed[] = [
  {
    coinId: 'VRSCTEST',
    proto: 'vrsc',
    fallbackTicker: 'VRSCTEST',
    fallbackName: 'Verus Testnet',
    amount: 18250.4421,
    fiatValue: 730.02,
    unitRate: 0.04
  },
  {
    coinId: 'ETH',
    proto: 'eth',
    fallbackTicker: 'ETH',
    fallbackName: 'Ethereum',
    amount: 0.5204,
    fiatValue: 1732.93,
    unitRate: 3330
  },
  {
    coinId: 'BTCTEST',
    proto: 'btc',
    fallbackTicker: 'BTCTEST',
    fallbackName: 'Bitcoin Testnet',
    amount: 0.0128,
    fiatValue: 817.92,
    unitRate: 63900
  },
  {
    coinId: 'USDC',
    proto: 'erc20',
    fallbackTicker: 'USDC',
    fallbackName: 'USD Coin',
    amount: 0,
    fiatValue: 0,
    unitRate: 1
  }
];

function toViewRow(seed: DemoRowSeed, intlLocale: string): WalletOverviewRowViewModel {
  const presentation = resolveCoinPresentationById(seed.coinId, seed.proto);
  const displayTicker = presentation?.displayTicker ?? seed.fallbackTicker;
  const displayName = presentation?.displayName ?? seed.fallbackName;
  const hasBalance = seed.amount > 0;
  return {
    key: seed.coinId,
    coinId: seed.coinId,
    proto: seed.proto,
    ticker: displayTicker,
    name: displayName,
    hasBalance,
    hasSnapshot: true,
    cryptoAmountDisplay: formatCryptoAmount(seed.amount, displayTicker, intlLocale, 4, 4),
    fiatValueDisplay: formatUsdAmount(seed.fiatValue, intlLocale),
    unitRateDisplay: formatUsdAmount(seed.unitRate, intlLocale),
    fiatSortValue: hasBalance ? seed.fiatValue : Number.NEGATIVE_INFINITY
  };
}

export function getWalletOverviewDemoSnapshot(
  network: WalletNetwork | undefined,
  intlLocale: string
): WalletOverviewViewModel {
  const seedRows = network === 'testnet' ? TESTNET_ROWS : MAINNET_ROWS;
  const rows = seedRows.map((row) => toViewRow(row, intlLocale));
  sortWalletOverviewRows(rows);

  const heroFiatTotal = seedRows
    .filter((row) => row.amount > 0)
    .reduce((sum, row) => sum + row.fiatValue, 0);
  const heroFiatDisplay = formatUsdAmount(heroFiatTotal, intlLocale);
  const heroFiatParts = formatUsdAmountParts(heroFiatTotal, intlLocale);
  const primaryTicker = network === 'testnet' ? 'VRSCTEST' : 'VRSC';
  const primarySeed = seedRows.find((row) => row.coinId === primaryTicker) ?? seedRows[0] ?? null;
  const primaryPresentation = primarySeed ? resolveCoinPresentationById(primarySeed.coinId) : null;
  const primaryDisplayTicker = primaryPresentation?.displayTicker ?? primarySeed?.fallbackTicker ?? primaryTicker;

  return {
    heroFiatDisplay,
    heroFiatSymbolDisplay: heroFiatParts.symbol,
    heroFiatValueDisplay: heroFiatParts.value,
    heroPrimaryCryptoDisplay: primarySeed
      ? formatCryptoAmount(primarySeed.amount, primaryDisplayTicker, intlLocale, 0, 4)
      : '—',
    assetCount: rows.length,
    identityCount: network === 'testnet' ? 1 : 2,
    rows,
    hasUsableLiveData: true,
    primaryTicker
  };
}
