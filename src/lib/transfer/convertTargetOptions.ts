import type { BridgeConversionPathQuote } from '$lib/types/wallet';

const POPULAR_CURRENCIES = ['VRSC', 'USDC', 'ETH', 'TBTC', 'DAI'] as const;

const CANONICAL_ASSET_MAP: Record<string, string> = {
  iGBs4DWztRNvNEJBt4mqHszLxfKTNHTkhM: 'DAI',
  DAI: 'DAI',
  iCkKJuJScy4Z6NSDK7Mt42ZAB2NEnAE1o4: 'MKR',
  MKR: 'MKR',
  i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV: 'VRSC',
  VRSC: 'VRSC',
  '0xBc2738BA63882891094C99E59a02141Ca1A1C36a': 'VRSC',
  i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd: 'USDC',
  USDC: 'USDC',
  iS8TfRPfVpKo5FVfSUzfHBQxo9KuzpnqLU: 'TBTC',
  '0x18084fbA666a33d37592fA2633fD49a74DD93a88': 'TBTC',
  iC5TQFrFXSYLQGkiZ8FYmZHFJzaRF5CYgE: 'EURC',
  '0x1aBaEA1f7C830bD89Acc67eC4af516284b1bC33c': 'EURC',
  i9nwxtKuVYX4MSbeULLiK2ttVi6rUEhh4X: 'ETH',
  ETH: 'ETH',
  i3f7tSctFkiPpiedY8QR5Tep9p4qDVebDx: 'BRIDGE',
  '0xE6052Dcc60573561ECef2D9A4C0FEA6d3aC5B9A2': 'BRIDGE',
};

type CanonicalDisplayInfo = { name: string; ticker: string };

const CANONICAL_DISPLAY_MAP: Record<string, CanonicalDisplayInfo> = {
  DAI: { name: 'DAI', ticker: 'DAI' },
  MKR: { name: 'Maker', ticker: 'MKR' },
  VRSC: { name: 'Verus', ticker: 'VRSC' },
  USDC: { name: 'USDC', ticker: 'USDC' },
  TBTC: { name: 'tBTC', ticker: 'TBTC' },
  EURC: { name: 'EURC', ticker: 'EURC' },
  ETH: { name: 'Ethereum', ticker: 'ETH' },
  BRIDGE: { name: 'Bridge.vETH', ticker: 'Bridge.vETH' },
};

type TargetBuildInput = {
  paths: Record<string, BridgeConversionPathQuote[]>;
  sourceCurrencyId: string;
  sourceCurrencyAliases?: string[];
};

type DestinationEntry = {
  destinationId: string;
  label: string;
  ticker: string;
  subtitle?: string;
  fullyqualifiedname: string;
  hasOnChainPath: boolean;
  viaOptions: ViaRouteOption[];
  exportOptions: ExportRouteOption[];
  gateway: boolean;
  isEthDestination: boolean;
  ethDisplayName?: string;
  ethDisplayTicker?: string;
};

function resolveMappingDestination(quote: BridgeConversionPathQuote): string | undefined {
  if (!quote.mapping) return undefined;

  const candidates = [
    quote.destinationDisplayTicker,
    quote.convertToDisplayName,
    quote.destinationDisplayName,
    quote.convertTo,
    quote.destinationId,
    quote.mapTo,
  ];
  let fallback: string | undefined;

  for (const candidate of candidates) {
    if (!candidate) continue;
    const trimmed = candidate.trim();
    if (trimmed.length === 0) continue;
    if (!fallback) fallback = trimmed;
    if (!isLikelyEvmAddress(trimmed)) return trimmed;
  }

  return fallback;
}

function isLikelyEvmAddress(value: string): boolean {
  return /^0x[a-fA-F0-9]{40}$/.test(value.trim());
}

export type ViaRouteOption = {
  id: string;
  key: string;
  receiveKey: string;
  receiveLabel: string;
  receiveSubtitle?: string;
  destinationId: string;
  convertTo?: string | null;
  convertToLabel?: string | null;
  exportTo?: string | null;
  exportToLabel?: string | null;
  via?: string | null;
  viaLabel?: string | null;
  mapTo?: string | null;
  price?: string | null;
  gateway?: boolean;
  mapping?: boolean;
  bounceback?: boolean;
  ethDestination?: boolean;
  prelaunch?: boolean;
};

export type ExportRouteOption = {
  exportTo: string;
  exportToName: string;
  gateway: boolean;
  via?: string | null;
  price?: string | null;
  mappingDestination?: string;
};

export type ReceiveAssetOption = {
  id: string;
  key: string;
  label: string;
  ticker: string;
  subtitle?: string;
  fullyqualifiedname: string;
  destinationId: string;
  canonicalKey: string;
  hasOnChainPath: boolean;
  isCrossChain: boolean;
  viaOptions: ViaRouteOption[];
  exportOptions: ExportRouteOption[];
  gateway: boolean;
  isEthDestination: boolean;
  ethDisplayName?: string;
  ethDisplayTicker?: string;
  isGrouped?: boolean;
  networkOptions?: ReceiveAssetOption[];
};

export type ReceiveAssetSections = {
  allOptions: ReceiveAssetOption[];
  popularOptions: ReceiveAssetOption[];
  otherOptions: ReceiveAssetOption[];
};

export function buildReceiveAssetSections(input: TargetBuildInput): ReceiveAssetSections {
  const sourceAliases = buildSourceAliasSet(input);
  const sourceIsEvmLike = input.sourceCurrencyId.trim().toLowerCase().startsWith('0x');
  const destinationMap = new Map<string, DestinationEntry>();
  const debugStats = {
    inputQuoteCount: 0,
    postPrelaunchCount: 0,
    postSuppressionCount: 0,
  };

  for (const pathList of Object.values(input.paths)) {
    if (!Array.isArray(pathList) || pathList.length === 0) continue;

    for (const quote of pathList) {
      debugStats.inputQuoteCount += 1;
      if (quote.prelaunch) continue;
      debugStats.postPrelaunchCount += 1;

      const destinationId = quote.destinationId || quote.convertTo || '';
      if (!destinationId) continue;
      const convertTo = quote.convertTo || destinationId;

      const sameCurrencyRoute = isSameCurrencyRoute(destinationId, convertTo, sourceAliases);
      if (sameCurrencyRoute && !quote.exportTo) {
        continue;
      }

      if (quote.mapping && quote.exportTo && !sameCurrencyRoute && !sourceIsEvmLike) {
        continue;
      }
      debugStats.postSuppressionCount += 1;

      let entry = destinationMap.get(destinationId);
      if (!entry) {
        const label = quote.destinationDisplayTicker || quote.destinationDisplayName || destinationId;
        const ticker = quote.destinationDisplayTicker || label;
        entry = {
          destinationId,
          label,
          ticker,
          subtitle:
            quote.destinationDisplayName && quote.destinationDisplayName !== label
              ? quote.destinationDisplayName
              : undefined,
          fullyqualifiedname: quote.destinationDisplayName || quote.convertTo || label,
          hasOnChainPath: false,
          viaOptions: [],
          exportOptions: [],
          gateway: quote.gateway,
          isEthDestination: quote.ethDestination,
          ethDisplayName: quote.ethDestination ? (quote.destinationDisplayName ?? undefined) : undefined,
          ethDisplayTicker: quote.ethDestination
            ? (quote.destinationDisplayTicker || quote.destinationDisplayName || undefined)
            : undefined,
        };
        destinationMap.set(destinationId, entry);
      }

      const exportTo = quote.exportTo ?? null;
      if (!exportTo) {
        entry.hasOnChainPath = true;
      }

      const viaId = quote.via ?? null;
      const viaKey = `${viaId ?? 'direct'}:${exportTo ?? 'onchain'}`.toLowerCase();
      if (!entry.viaOptions.some((option) => option.key === viaKey)) {
        const receiveKey = normalizeCase(convertTo);
        entry.viaOptions.push({
          id: [
            'path',
            normalizeCase(destinationId),
            viaId ? normalizeCase(viaId) : 'direct',
            exportTo ? normalizeCase(exportTo) : 'onchain',
          ].join('|'),
          key: viaKey,
          receiveKey,
          receiveLabel: entry.ticker,
          receiveSubtitle: entry.label !== entry.ticker ? entry.label : undefined,
          destinationId,
          convertTo,
          convertToLabel: quote.convertToDisplayName ?? quote.destinationDisplayName ?? convertTo,
          exportTo,
          exportToLabel: quote.exportToDisplayName ?? exportTo,
          via: viaId,
          viaLabel: quote.viaDisplayName ?? viaId,
          mapTo: quote.mapTo ?? null,
          price: quote.price ?? null,
          gateway: quote.gateway,
          mapping: quote.mapping,
          bounceback: quote.bounceback,
          ethDestination: quote.ethDestination,
          prelaunch: quote.prelaunch,
        });
      }

      if (exportTo) {
        const mappingDestination = resolveMappingDestination(quote);
        const existingOption = entry.exportOptions.find(
          (option) => normalizeCase(option.exportTo) === normalizeCase(exportTo)
        );

        if (!existingOption) {
          entry.exportOptions.push({
            exportTo,
            exportToName: quote.exportToDisplayName ?? exportTo,
            gateway: quote.gateway,
            via: quote.via ?? null,
            price: quote.price ?? null,
            mappingDestination,
          });
        } else if (!existingOption.mappingDestination && mappingDestination) {
          existingOption.mappingDestination = mappingDestination;
        }
      }
    }
  }

  const ungroupedOptions = Array.from(destinationMap.values()).map((entry) =>
    toReceiveAssetOption(entry)
  );

  const groupedByCanonicalKey = new Map<string, ReceiveAssetOption[]>();
  for (const option of ungroupedOptions) {
    const canonicalKey = getCanonicalAssetKey(
      option.destinationId,
      option.ticker,
      option.fullyqualifiedname || option.label
    );
    option.canonicalKey = canonicalKey;
    const current = groupedByCanonicalKey.get(canonicalKey) ?? [];
    current.push(option);
    groupedByCanonicalKey.set(canonicalKey, current);
  }

  const mergedOptions: ReceiveAssetOption[] = [];
  for (const [canonicalKey, options] of groupedByCanonicalKey) {
    if (options.length === 1) {
      mergedOptions.push(options[0]);
      continue;
    }

    const displayInfo = getCanonicalAssetDisplayInfo(canonicalKey);
    const sortedNetworkOptions = [...options].sort((a, b) => {
      if (a.hasOnChainPath && !b.hasOnChainPath) return -1;
      if (!a.hasOnChainPath && b.hasOnChainPath) return 1;
      return a.label.localeCompare(b.label);
    });

    const groupedVia = dedupeViaOptions(
      sortedNetworkOptions.flatMap((option) => option.viaOptions)
    );
    const groupedExport = dedupeExportOptions(
      sortedNetworkOptions.flatMap((option) => option.exportOptions)
    );
    const iconId =
      displayInfo.ticker.trim() ||
      sortedNetworkOptions.find((option) => !option.destinationId.startsWith('0x'))?.destinationId ||
      sortedNetworkOptions[0].destinationId;
    const groupedFqn = displayInfo.ticker.trim() || displayInfo.name;

    mergedOptions.push({
      id: canonicalKey,
      key: canonicalKey,
      label: displayInfo.name,
      ticker: displayInfo.ticker,
      subtitle: undefined,
      fullyqualifiedname: groupedFqn,
      destinationId: iconId,
      canonicalKey,
      hasOnChainPath: sortedNetworkOptions.some((option) => option.hasOnChainPath),
      isCrossChain: groupedExport.length > 0,
      viaOptions: groupedVia,
      exportOptions: groupedExport,
      gateway: sortedNetworkOptions.some((option) => option.gateway),
      isEthDestination: sortedNetworkOptions.some((option) => option.isEthDestination),
      isGrouped: true,
      networkOptions: sortedNetworkOptions,
    });
  }

  const popularOptions: ReceiveAssetOption[] = [];
  const otherOptions: ReceiveAssetOption[] = [];
  for (const option of mergedOptions) {
    if (isPopularConversion(option)) {
      popularOptions.push(option);
    } else {
      otherOptions.push(option);
    }
  }

  popularOptions.sort((a, b) => popularIndex(a) - popularIndex(b));
  otherOptions.sort((a, b) => a.label.localeCompare(b.label));

  if (parityDebugEnabled()) {
    // eslint-disable-next-line no-console
    console.debug('[BRIDGE][PARITY][TARGETS]', {
      inputQuoteCount: debugStats.inputQuoteCount,
      postPrelaunchCount: debugStats.postPrelaunchCount,
      postSuppressionCount: debugStats.postSuppressionCount,
      finalGroupedOptionCount: popularOptions.length + otherOptions.length,
    });
  }

  return {
    allOptions: [...popularOptions, ...otherOptions],
    popularOptions,
    otherOptions,
  };
}

export function filterReceiveAssetSectionsByQuery(
  sections: ReceiveAssetSections,
  query: string
): ReceiveAssetSections {
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return sections;
  }

  const filterOption = (option: ReceiveAssetOption): ReceiveAssetOption | null => {
    if (optionMatchesQuery(option, normalized)) {
      return option;
    }

    if (!option.isGrouped || !option.networkOptions) {
      return null;
    }

    const matchedNetworkOptions = option.networkOptions.filter((networkOption) =>
      optionMatchesQuery(networkOption, normalized)
    );
    if (matchedNetworkOptions.length === 0) {
      return null;
    }

    return {
      ...option,
      networkOptions: matchedNetworkOptions,
    };
  };

  const filteredPopular = sections.popularOptions
    .map(filterOption)
    .filter((option): option is ReceiveAssetOption => option !== null);
  const filteredOther = sections.otherOptions
    .map(filterOption)
    .filter((option): option is ReceiveAssetOption => option !== null);

  return {
    allOptions: [...filteredPopular, ...filteredOther],
    popularOptions: filteredPopular,
    otherOptions: filteredOther,
  };
}

function toReceiveAssetOption(entry: DestinationEntry): ReceiveAssetOption {
  const canonicalKey = getCanonicalAssetKey(
    entry.destinationId,
    entry.ticker,
    entry.fullyqualifiedname || entry.label
  );
  return {
    id: entry.destinationId,
    key: normalizeCase(entry.destinationId),
    label: entry.label,
    ticker: entry.ticker,
    subtitle: entry.subtitle,
    fullyqualifiedname: entry.fullyqualifiedname,
    destinationId: entry.destinationId,
    canonicalKey,
    hasOnChainPath: entry.hasOnChainPath,
    isCrossChain: entry.exportOptions.length > 0,
    viaOptions: entry.viaOptions,
    exportOptions: entry.exportOptions,
    gateway: entry.gateway,
    isEthDestination: entry.isEthDestination,
    ethDisplayName: entry.ethDisplayName,
    ethDisplayTicker: entry.ethDisplayTicker,
  };
}

function dedupeViaOptions(options: ViaRouteOption[]): ViaRouteOption[] {
  const dedupe = new Map<string, ViaRouteOption>();
  for (const option of options) {
    if (!dedupe.has(option.key)) {
      dedupe.set(option.key, option);
    }
  }
  return Array.from(dedupe.values());
}

function dedupeExportOptions(options: ExportRouteOption[]): ExportRouteOption[] {
  const dedupe = new Map<string, ExportRouteOption>();
  for (const option of options) {
    const key = normalizeCase(option.exportTo);
    const existing = dedupe.get(key);
    if (!existing) {
      dedupe.set(key, option);
      continue;
    }

    if (!existing.mappingDestination && option.mappingDestination) {
      dedupe.set(key, {
        ...existing,
        mappingDestination: option.mappingDestination,
      });
    }
  }
  return Array.from(dedupe.values());
}

function buildSourceAliasSet(input: TargetBuildInput): Set<string> {
  const aliases = new Set<string>();
  addAlias(aliases, input.sourceCurrencyId);
  for (const alias of input.sourceCurrencyAliases ?? []) {
    addAlias(aliases, alias);
  }
  return aliases;
}

function addAlias(target: Set<string>, value?: string | null): void {
  if (!value) return;
  const normalized = normalizeCase(value);
  if (!normalized) return;
  target.add(normalized);
}

function isSameCurrencyRoute(destinationId: string, convertTo: string, sourceAliases: Set<string>): boolean {
  if (sourceAliases.size === 0) return false;
  return sourceAliases.has(normalizeCase(destinationId)) || sourceAliases.has(normalizeCase(convertTo));
}

function getCanonicalAssetKey(currencyId: string, ticker = '', name = ''): string {
  if (CANONICAL_ASSET_MAP[currencyId]) {
    return CANONICAL_ASSET_MAP[currencyId];
  }

  const tickerUpper = ticker.trim().toUpperCase();
  if (tickerUpper.endsWith('.VETH')) {
    return tickerUpper.replace('.VETH', '');
  }
  if (tickerUpper.startsWith('V') && tickerUpper.includes('.VETH')) {
    return tickerUpper.substring(1).replace('.VETH', '');
  }
  if (tickerUpper.includes('[ERC20]')) {
    return tickerUpper.replace(/\s*\[ERC20\]\s*/gi, '').trim();
  }

  const nameUpper = name.trim().toUpperCase();
  const onVerusMatch = nameUpper.match(/^(.+?)\s+ON\s+VERUS$/);
  if (onVerusMatch) return onVerusMatch[1].trim().toUpperCase();
  const onEthMatch = nameUpper.match(/^(.+?)\s+ON\s+ETHEREUM$/);
  if (onEthMatch) return onEthMatch[1].trim().toUpperCase();

  // Distinguish bridge currencies that resolve to generic "Bridge" tickers
  // so unrelated Bridge.* assets do not collapse into one grouped entry.
  if (tickerUpper === 'BRIDGE' && nameUpper.startsWith('BRIDGE.')) {
    return nameUpper === 'BRIDGE.VETH' ? 'BRIDGE' : nameUpper;
  }

  return tickerUpper || currencyId.toUpperCase();
}

function getCanonicalAssetDisplayInfo(canonicalKey: string): CanonicalDisplayInfo {
  return CANONICAL_DISPLAY_MAP[canonicalKey] ?? { name: canonicalKey, ticker: canonicalKey };
}

function isPopularConversion(option: ReceiveAssetOption): boolean {
  const canonicalKey = option.canonicalKey.toUpperCase();
  const nameUpper = option.label.toUpperCase();
  const tickerUpper = option.ticker.toUpperCase();

  return POPULAR_CURRENCIES.some(
    (popular) =>
      canonicalKey === popular ||
      nameUpper === popular ||
      tickerUpper === popular ||
      nameUpper.startsWith(`${popular}.`) ||
      tickerUpper.startsWith(`${popular}.`) ||
      nameUpper.endsWith(`.${popular}`) ||
      tickerUpper.endsWith(`.${popular}`)
  );
}

function popularIndex(option: ReceiveAssetOption): number {
  const canonicalKey = option.canonicalKey.toUpperCase();
  const nameUpper = option.label.toUpperCase();
  const tickerUpper = option.ticker.toUpperCase();
  const index = POPULAR_CURRENCIES.findIndex(
    (popular) =>
      canonicalKey === popular ||
      nameUpper === popular ||
      tickerUpper === popular ||
      nameUpper.startsWith(`${popular}.`) ||
      tickerUpper.startsWith(`${popular}.`) ||
      nameUpper.endsWith(`.${popular}`) ||
      tickerUpper.endsWith(`.${popular}`)
  );
  return index === -1 ? Number.MAX_SAFE_INTEGER : index;
}

function optionMatchesQuery(option: ReceiveAssetOption, query: string): boolean {
  const values = [
    option.label,
    option.ticker,
    option.id,
    option.fullyqualifiedname,
    option.canonicalKey,
    option.destinationId,
  ];
  return values.some((value) => value.toLowerCase().includes(query));
}

function normalizeCase(value: string): string {
  return value.trim().toLowerCase();
}

function parityDebugEnabled(): boolean {
  if (!import.meta.env.DEV) return false;
  if (typeof window === 'undefined') return false;
  return window.localStorage.getItem('BRIDGE_PATH_PARITY_DEBUG') === '1';
}
