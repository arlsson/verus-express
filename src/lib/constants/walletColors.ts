export const WALLET_COLOR_OPTIONS = [
  { name: 'blue', hex: '#2563EB' },
  { name: 'indigo', hex: '#4F46E5' },
  { name: 'violet', hex: '#7C3AED' },
  { name: 'purple', hex: '#A21CAF' },
  { name: 'pink', hex: '#DB2777' },
  { name: 'red', hex: '#DC2626' },
  { name: 'orange', hex: '#EA580C' },
  { name: 'amber', hex: '#CA8A04' },
  { name: 'lime', hex: '#65A30D' },
  { name: 'green', hex: '#16A34A' },
  { name: 'teal', hex: '#0F766E' },
  { name: 'cyan', hex: '#0891B2' },
  { name: 'slate', hex: '#475569' },
  { name: 'stone', hex: '#78716C' }
] as const;

export type WalletColorOption = (typeof WALLET_COLOR_OPTIONS)[number];
export type WalletColorName = WalletColorOption['name'];

export const DEFAULT_WALLET_COLOR: WalletColorName = 'blue';

const LEGACY_WALLET_COLOR_ALIASES: Record<string, WalletColorName> = {
  sky: 'cyan',
  emerald: 'green',
  yellow: 'amber',
  rose: 'pink',
  gray: 'slate',
  zinc: 'stone'
};

const walletColorMap = WALLET_COLOR_OPTIONS.reduce(
  (map, color) => {
    map[color.name] = color;
    return map;
  },
  {} as Record<WalletColorName, WalletColorOption>
);

export function normalizeWalletColor(color?: string | null): WalletColorName {
  if (!color) return DEFAULT_WALLET_COLOR;
  const normalized = color.trim().toLowerCase();
  if (normalized in walletColorMap) {
    return normalized as WalletColorName;
  }
  return LEGACY_WALLET_COLOR_ALIASES[normalized] ?? DEFAULT_WALLET_COLOR;
}

export function getWalletColor(color?: string | null): WalletColorOption {
  return walletColorMap[normalizeWalletColor(color)];
}

export function getWalletColorHex(color?: string | null): string {
  return getWalletColor(color).hex;
}
