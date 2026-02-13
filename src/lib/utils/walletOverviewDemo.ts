import type { WalletNetwork } from '$lib/types/wallet.js';
import {
  formatCryptoAmount,
  formatUsdAmount,
  formatUsdAmountParts,
  sortWalletOverviewRows,
  type WalletOverviewRowViewModel,
  type WalletOverviewViewModel
} from '$lib/utils/walletOverview.js';

interface DemoRowSeed {
  coinId: string;
  ticker: string;
  name: string;
  amount: number;
  fiatValue: number;
  unitRate: number;
}

const MAINNET_ROWS: DemoRowSeed[] = [
  {
    coinId: 'VRSC',
    ticker: 'VRSC',
    name: 'Verus',
    amount: 2784.2191,
    fiatValue: 6431.55,
    unitRate: 2.31
  },
  {
    coinId: 'ETH',
    ticker: 'ETH',
    name: 'Ethereum',
    amount: 1.2408,
    fiatValue: 4131.86,
    unitRate: 3330
  },
  {
    coinId: 'BTC',
    ticker: 'BTC',
    name: 'Bitcoin',
    amount: 0.0314,
    fiatValue: 2006.46,
    unitRate: 63900
  },
  {
    coinId: 'USDC',
    ticker: 'USDC',
    name: 'USD Coin',
    amount: 0,
    fiatValue: 0,
    unitRate: 1
  }
];

const TESTNET_ROWS: DemoRowSeed[] = [
  {
    coinId: 'VRSCTEST',
    ticker: 'VRSCTEST',
    name: 'Verus Testnet',
    amount: 18250.4421,
    fiatValue: 730.02,
    unitRate: 0.04
  },
  {
    coinId: 'ETH',
    ticker: 'ETH',
    name: 'Ethereum',
    amount: 0.5204,
    fiatValue: 1732.93,
    unitRate: 3330
  },
  {
    coinId: 'BTCTEST',
    ticker: 'BTCTEST',
    name: 'Bitcoin Testnet',
    amount: 0.0128,
    fiatValue: 817.92,
    unitRate: 63900
  },
  {
    coinId: 'USDC',
    ticker: 'USDC',
    name: 'USD Coin',
    amount: 0,
    fiatValue: 0,
    unitRate: 1
  }
];

function toViewRow(seed: DemoRowSeed, intlLocale: string): WalletOverviewRowViewModel {
  const hasBalance = seed.amount > 0;
  return {
    key: seed.coinId,
    coinId: seed.coinId,
    ticker: seed.ticker,
    name: seed.name,
    hasBalance,
    hasSnapshot: true,
    cryptoAmountDisplay: formatCryptoAmount(seed.amount, seed.ticker, intlLocale, 4, 4),
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
  const primarySeed = seedRows.find((row) => row.ticker === primaryTicker) ?? seedRows[0] ?? null;

  return {
    heroFiatDisplay,
    heroFiatSymbolDisplay: heroFiatParts.symbol,
    heroFiatValueDisplay: heroFiatParts.value,
    heroPrimaryCryptoDisplay: primarySeed
      ? formatCryptoAmount(primarySeed.amount, primarySeed.ticker, intlLocale, 0, 4)
      : '—',
    rows,
    hasUsableLiveData: true,
    primaryTicker
  };
}
