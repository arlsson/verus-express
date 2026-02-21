<!--
  Component: AssetDetails
  Purpose: Coin detail view with scope selectors (address × network), scoped actions, and scoped transaction history.
-->

<script lang="ts">
  import SendIcon from '@lucide/svelte/icons/send';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import ArrowLeftRightIcon from '@lucide/svelte/icons/arrow-left-right';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import CheckIcon from '@lucide/svelte/icons/check';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import { Spinner } from '$lib/components/ui/spinner';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import PrivateVerusWordmark from '$lib/components/wallet/PrivateVerusWordmark.svelte';
  import { i18nStore } from '$lib/i18n';
  import { resolveCoinPresentation } from '$lib/coins/presentation.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { balanceStore, getBalance } from '$lib/stores/balances.js';
  import { networkStore } from '$lib/stores/network.js';
  import { ratesStore } from '$lib/stores/rates.js';
  import {
    scopesByCoinId,
    selectedAddressByCoinId,
    selectedSystemByCoinId,
    setCoinScopes,
    setSelectedScopeAddress,
    setSelectedScopeSystem
  } from '$lib/stores/coinScopes.js';
  import { formatUsdAmount } from '$lib/utils/walletOverview.js';
  import { extractWalletErrorMessage, extractWalletErrorType } from '$lib/utils/walletErrors.js';
  import * as walletService from '$lib/services/walletService.js';
  import type { CoinScope, ScopeKind, Transaction, WalletEntryKind } from '$lib/types/wallet.js';
  import type { TransferEntryContext } from './transfer-wizard/types';

  const TRANSACTION_PAGE_SIZE = 50;
  const TRANSACTION_LOAD_MORE_THRESHOLD_PX = 160;
  const initialTransactionSkeletonRows = [0, 1, 2, 3, 4, 5];
  const loadMoreTransactionSkeletonRows = [0, 1];

  type AssetDetailsProps = {
    coinId: string;
    walletEntryKind?: WalletEntryKind;
    scopeFilterMode?: ScopeKind;
    entryDisplayName?: string;
    onNavigateToReceive?: () => void;
    // eslint-disable-next-line no-unused-vars
    onNavigateToSend?: (_context: TransferEntryContext) => void;
    // eslint-disable-next-line no-unused-vars
    onNavigateToConvert?: (_context: TransferEntryContext) => void;
  };

  const noop = () => {};

  /* eslint-disable prefer-const */
  let {
    coinId,
    walletEntryKind = 'coin',
    scopeFilterMode = 'transparent',
    entryDisplayName,
    onNavigateToReceive = noop,
    onNavigateToSend = noop,
    onNavigateToConvert = noop
  }: AssetDetailsProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);
  const coins = $derived($coinsStore);
  const balances = $derived($balanceStore);
  const chainInfo = $derived($networkStore);
  const rates = $derived($ratesStore);
  const allScopesByCoinId = $derived($scopesByCoinId);
  const selectedAddressMap = $derived($selectedAddressByCoinId);
  const selectedSystemMap = $derived($selectedSystemByCoinId);

  type ScopeTransactionPageState = {
    items: Transaction[];
    nextCursor: string | null;
    hasMore: boolean;
    initialLoaded: boolean;
    loadingInitial: boolean;
    loadingMore: boolean;
    error: string;
    loadMoreError: string;
  };

  const createEmptyPageState = (): ScopeTransactionPageState => ({
    items: [],
    nextCursor: null,
    hasMore: false,
    initialLoaded: false,
    loadingInitial: false,
    loadingMore: false,
    error: '',
    loadMoreError: ''
  });

  let scopesLoading = $state(false);
  let scopesError = $state('');
  let loadingSelectedBalance = $state(false);
  let showScopeSheet = $state(false);
  let addressSearchTerm = $state('');
  let copiedAddressKey = $state<string | null>(null);

  let loadedBalanceByChannel = $state<Record<string, boolean>>({});
  let txPagesByScopeKey = $state<Record<string, ScopeTransactionPageState>>({});
  let txScrollElement = $state<HTMLElement | null>(null);
  let canScrollTxDown = $state(false);

  let scopeRequestSequence = 0;
  let selectedBalanceRequestSequence = 0;
  const inFlightBalanceChannels = new Set<string>();
  const inFlightTransactionScopeKeys = new Set<string>();

  const coin = $derived(coins.find((item) => item.id === coinId) ?? null);
  const coinPresentation = $derived((coin ? resolveCoinPresentation(coin) : null));
  const coinScopes = $derived(allScopesByCoinId[coinId] ?? []);
  const allScopes = $derived(
    coinScopes.filter((scope) => scope.scopeKind === scopeFilterMode)
  );

  const selectedAddress = $derived(selectedAddressMap[coinId] ?? '');
  const selectedSystem = $derived(selectedSystemMap[coinId] ?? '');

  const filteredScopeOptions = $derived(
    (() => {
      const query = addressSearchTerm.trim().toLowerCase();
      if (!query) return allScopes;
      return allScopes.filter((scope) => {
        return (
          scope.address.toLowerCase().includes(query) ||
          scope.addressLabel.toLowerCase().includes(query) ||
          networkLabelForScope(scope).toLowerCase().includes(query)
        );
      });
    })()
  );

  const selectedScope = $derived(
    allScopes.find((scope) => scope.address === selectedAddress && scope.systemId === selectedSystem) ??
      null
  );

  const selectedScopeBalance = $derived(
    selectedScope && coin ? getBalance(selectedScope.channelId, coin.id, balances) : undefined
  );

  const selectedScopePageKey = $derived(
    selectedScope && coin ? getScopePageKey(selectedScope.channelId, coin.id) : ''
  );

  const selectedScopeTxPage = $derived(
    selectedScopePageKey ? txPagesByScopeKey[selectedScopePageKey] ?? null : null
  );

  const selectedScopeTransactions = $derived(
    selectedScopeTxPage?.items ?? []
  );

  const sortedSelectedTransactions = $derived(
    [...selectedScopeTransactions].sort((left, right) => {
      const pendingOrder = Number(right.pending) - Number(left.pending);
      if (pendingOrder !== 0) return pendingOrder;
      return (right.timestamp ?? 0) - (left.timestamp ?? 0);
    })
  );

  const loadingSelectedTransactions = $derived(selectedScopeTxPage?.loadingInitial ?? false);
  const loadingMoreTransactions = $derived(selectedScopeTxPage?.loadingMore ?? false);
  const selectedScopeTransactionsError = $derived(selectedScopeTxPage?.error ?? '');
  const selectedScopeLoadMoreError = $derived(selectedScopeTxPage?.loadMoreError ?? '');
  const selectedScopeHasMoreTransactions = $derived(selectedScopeTxPage?.hasMore ?? false);

  const selectedAmountValue = $derived(
    toFiniteNumber(selectedScopeBalance?.total) ?? 0
  );

  const selectedRateMetrics = $derived(
    (() => {
      if (!coin) return null;
      const candidates = [coin.id, coin.currencyId, coin.mappedTo].filter(
        (value): value is string => typeof value === 'string' && value.trim().length > 0
      );
      for (const candidate of candidates) {
        const snapshot = rates[candidate];
        const usd = snapshot?.rates?.USD ?? snapshot?.rates?.usd;
        const usdRate = typeof usd === 'number' && Number.isFinite(usd) ? usd : null;
        const rawChange = snapshot?.usdChange24hPct;
        const change24hPct =
          typeof rawChange === 'number' && Number.isFinite(rawChange) ? rawChange : null;
        if (usdRate !== null || change24hPct !== null) {
          return {
            usdRate,
            change24hPct
          };
        }
      }
      return null;
    })()
  );

  const selectedUsdRate = $derived(selectedRateMetrics?.usdRate ?? null);

  const selectedChange24hPct = $derived(selectedRateMetrics?.change24hPct ?? null);

  const selectedChange24hDirection = $derived(getChangeDirection(selectedChange24hPct));

  const selectedChange24hDisplay = $derived(
    selectedChange24hDirection === 'none' || selectedChange24hPct === null
      ? '—'
      : formatPercentChange(selectedChange24hPct)
  );

  const selectedUnitRateDisplay = $derived(
    selectedUsdRate === null ? '—' : formatUsdAmount(selectedUsdRate, i18n.intlLocale)
  );

  const selectedFiatDisplay = $derived(
    selectedUsdRate === null
      ? '—'
      : formatUsdAmount(selectedAmountValue * selectedUsdRate, i18n.intlLocale)
  );

  const selectedCryptoAmountDisplay = $derived(
    formatCryptoValue(selectedAmountValue, coin?.decimals ?? 8)
  );

  const selectedSyncPercent = $derived(
    selectedScope ? toFiniteNumber(chainInfo[selectedScope.channelId]?.percent) : null
  );
  const isShieldedSyncBlocked = $derived(
    selectedScope?.scopeKind === 'shielded' &&
      selectedSyncPercent !== null &&
      selectedSyncPercent !== 100 &&
      selectedSyncPercent !== -1
  );
  const selectedSyncPercentDisplay = $derived(
    isShieldedSyncBlocked && selectedSyncPercent !== null
      ? `${formatSyncPercent(selectedSyncPercent)}%`
      : ''
  );
  const canSendOrConvert = $derived(
    !!selectedScope && !selectedScope.isReadOnly && !isShieldedSyncBlocked
  );
  const selectedNetworkDisplay = $derived(
    (() => {
      if (selectedScope) return networkLabelForScope(selectedScope);
      if (allScopes.length > 0) return networkLabelForScope(allScopes[0]);
      return '—';
    })()
  );
  const isSingleAddressExternalAsset = $derived(
    (() => {
      if (!coin) return false;
      if (coin.proto !== 'eth' && coin.proto !== 'erc20' && coin.proto !== 'btc') return false;
      const uniqueAddresses = new Set(allScopes.map((scope) => scope.address));
      return uniqueAddresses.size <= 1;
    })()
  );
  const useStaticAddressRow = $derived(
    isSingleAddressExternalAsset || scopeFilterMode === 'shielded'
  );
  const selectedFqnDisplay = $derived(
    (() => {
      const runtimeCoin = coin as (typeof coin & { fullyQualifiedName?: string | null }) | null;
      const candidates = [
        runtimeCoin?.fullyQualifiedName ?? null,
        coinPresentation?.displayTicker ?? null,
        coin?.displayTicker ?? null
      ]
        .map((value) => (typeof value === 'string' ? value.trim() : ''))
        .filter((value) => value.length > 0);
      const dotted = candidates.find((value) => value.includes('.'));
      return dotted ?? candidates[0] ?? '';
    })()
  );
  const headerDisplayName = $derived(
    (entryDisplayName?.trim() || coinPresentation?.displayName || coin?.displayName || '')
  );
  const headerFqnDisplay = $derived(entryDisplayName?.trim() ? '' : selectedFqnDisplay);
  const usePrivateMutedIcon = $derived(scopeFilterMode === 'shielded');
  const usePrivateWordmark = $derived(walletEntryKind === 'private_verus');

  $effect(() => {
    coinId;
    scopeFilterMode;
    addressSearchTerm = '';
    showScopeSheet = false;
    scopesError = '';
    void loadScopes();
  });

  $effect(() => {
    if (!useStaticAddressRow) return;
    showScopeSheet = false;
    addressSearchTerm = '';
  });

  $effect(() => {
    const scopes = allScopes;
    if (scopes.length === 0) return;

    const preferredAddress = selectedAddressMap[coinId];
    let nextAddress =
      preferredAddress && scopes.some((scope) => scope.address === preferredAddress)
        ? preferredAddress
        : '';
    if (!nextAddress) {
      nextAddress = scopes.find((scope) => scope.isPrimaryAddress)?.address ?? scopes[0].address;
      setSelectedScopeAddress(coinId, nextAddress);
    }

    const systemsForAddress = scopes
      .filter((scope) => scope.address === nextAddress)
      .map((scope) => scope.systemId);
    const preferredSystem = selectedSystemMap[coinId];
    let nextSystem =
      preferredSystem && systemsForAddress.includes(preferredSystem) ? preferredSystem : '';
    if (!nextSystem) {
      const rootSystemId = coin?.systemId ?? '';
      nextSystem = systemsForAddress.includes(rootSystemId) ? rootSystemId : (systemsForAddress[0] ?? '');
      if (nextSystem) {
        setSelectedScopeSystem(coinId, nextSystem);
      }
    }
  });

  $effect(() => {
    const activeScope = selectedScope;
    const currentCoin = coin;
    if (!activeScope || !currentCoin) return;

    const requestKey = `${activeScope.channelId}::${currentCoin.id}`;
    selectedBalanceRequestSequence += 1;
    const requestSequence = selectedBalanceRequestSequence;
    loadingSelectedBalance = true;

    void (async () => {
      await fetchBalanceForScope(activeScope, currentCoin.id);
      if (selectedBalanceRequestSequence === requestSequence && requestKey === `${activeScope.channelId}::${currentCoin.id}`) {
        loadingSelectedBalance = false;
      }
    })();
  });

  $effect(() => {
    const currentCoin = coin;
    const activeScope = selectedScope;
    const address = selectedAddress;
    if (!currentCoin || !activeScope || !address) return;

    const siblingScopes = allScopes.filter(
      (scope) => scope.address === address && scope.channelId !== activeScope.channelId
    );
    if (siblingScopes.length === 0) return;
    void fetchSiblingBalances(siblingScopes, currentCoin.id);
  });

  $effect(() => {
    const currentCoin = coin;
    if (!showScopeSheet || !currentCoin || allScopes.length === 0) return;
    void fetchSiblingBalances(allScopes, currentCoin.id);
  });

  $effect(() => {
    const currentCoin = coin;
    const activeScope = selectedScope;
    if (!currentCoin || !activeScope) return;
    void ensureInitialTransactionsForScope(activeScope, currentCoin.id);
  });

  $effect(() => {
    sortedSelectedTransactions.length;
    loadingSelectedTransactions;
    loadingMoreTransactions;
    selectedScopeHasMoreTransactions;
    const element = txScrollElement;
    if (!element) return () => {};

    const resizeObserver = new ResizeObserver(() => {
      updateTxScrollAffordance();
      void maybeLoadMoreTransactions();
    });
    resizeObserver.observe(element);
    const viewportContent = element.querySelector('[data-scroll-area-content]');
    if (viewportContent instanceof HTMLElement) {
      resizeObserver.observe(viewportContent);
    } else {
      const fallbackContent = element.lastElementChild;
      if (fallbackContent instanceof HTMLElement) {
        resizeObserver.observe(fallbackContent);
      }
    }
    const frame = window.requestAnimationFrame(() => {
      updateTxScrollAffordance();
      void maybeLoadMoreTransactions();
    });

    return () => {
      window.cancelAnimationFrame(frame);
      resizeObserver.disconnect();
    };
  });

  async function loadScopes(): Promise<void> {
    const currentCoinId = coinId;
    scopeRequestSequence += 1;
    const requestSequence = scopeRequestSequence;

    scopesLoading = true;
    scopesError = '';
    try {
      const result = await walletService.getCoinScopes(currentCoinId);
      if (requestSequence !== scopeRequestSequence) return;
      setCoinScopes(currentCoinId, result.scopes);
    } catch (error) {
      if (requestSequence !== scopeRequestSequence) return;
      scopesError = mapWalletError(error);
    } finally {
      if (requestSequence === scopeRequestSequence) {
        scopesLoading = false;
      }
    }
  }

  async function fetchBalanceForScope(scope: CoinScope, currentCoinId: string): Promise<void> {
    if (inFlightBalanceChannels.has(scope.channelId)) return;
    inFlightBalanceChannels.add(scope.channelId);

    try {
      const balance = await walletService.getBalances(scope.channelId, currentCoinId);
      balanceStore.update((state) => ({
        ...state,
        [scope.channelId]: {
          ...(state[scope.channelId] ?? {}),
          [currentCoinId]: balance
        }
      }));
      loadedBalanceByChannel = {
        ...loadedBalanceByChannel,
        [scope.channelId]: true
      };
    } catch {
      // Balance refresh is best effort for sibling scopes.
    } finally {
      inFlightBalanceChannels.delete(scope.channelId);
    }
  }

  async function fetchSiblingBalances(scopes: CoinScope[], currentCoinId: string): Promise<void> {
    const pendingScopes = scopes.filter(
      (scope) => !loadedBalanceByChannel[scope.channelId] && !inFlightBalanceChannels.has(scope.channelId)
    );
    if (pendingScopes.length === 0) return;

    const concurrency = Math.min(2, pendingScopes.length);
    let cursor = 0;

    const workers = Array.from({ length: concurrency }, async () => {
      while (cursor < pendingScopes.length) {
        const index = cursor;
        cursor += 1;
        const scope = pendingScopes[index];
        if (!scope) return;
        await fetchBalanceForScope(scope, currentCoinId);
      }
    });

    await Promise.all(workers);
  }

  function getScopePageKey(channelId: string, currentCoinId: string): string {
    return `${channelId}::${currentCoinId}`;
  }

  function updateScopeTxPageState(
    scopePageKey: string,
    // eslint-disable-next-line no-unused-vars
    updater: (_state: ScopeTransactionPageState) => ScopeTransactionPageState
  ): void {
    const previous = txPagesByScopeKey[scopePageKey] ?? createEmptyPageState();
    txPagesByScopeKey = {
      ...txPagesByScopeKey,
      [scopePageKey]: updater(previous)
    };
  }

  function dedupeTransactions(items: Transaction[]): Transaction[] {
    const seen = new Set<string>();
    const out: Transaction[] = [];
    for (const item of items) {
      if (seen.has(item.txid)) continue;
      seen.add(item.txid);
      out.push(item);
    }
    return out;
  }

  async function ensureInitialTransactionsForScope(
    scope: CoinScope,
    currentCoinId: string
  ): Promise<void> {
    const scopePageKey = getScopePageKey(scope.channelId, currentCoinId);
    const existing = txPagesByScopeKey[scopePageKey];
    if (existing?.initialLoaded || existing?.loadingInitial) return;
    if (inFlightTransactionScopeKeys.has(scopePageKey)) return;

    inFlightTransactionScopeKeys.add(scopePageKey);
    updateScopeTxPageState(scopePageKey, (state) => ({
      ...state,
      loadingInitial: true,
      loadingMore: false,
      error: '',
      loadMoreError: ''
    }));

    try {
      const page = await walletService.getTransactionHistoryPage(
        scope.channelId,
        currentCoinId,
        undefined,
        TRANSACTION_PAGE_SIZE
      );
      updateScopeTxPageState(scopePageKey, (state) => ({
        ...state,
        items: dedupeTransactions(page.transactions),
        nextCursor: page.nextCursor ?? null,
        hasMore: page.hasMore,
        initialLoaded: true,
        loadingInitial: false,
        loadingMore: false,
        error: '',
        loadMoreError: ''
      }));
    } catch (error) {
      updateScopeTxPageState(scopePageKey, (state) => ({
        ...state,
        loadingInitial: false,
        initialLoaded: false,
        error: mapWalletError(error)
      }));
    } finally {
      inFlightTransactionScopeKeys.delete(scopePageKey);
      updateTxScrollAffordance();
      void maybeLoadMoreTransactions();
    }
  }

  async function loadMoreTransactionsForScope(scope: CoinScope, currentCoinId: string): Promise<void> {
    const scopePageKey = getScopePageKey(scope.channelId, currentCoinId);
    const state = txPagesByScopeKey[scopePageKey];
    if (!state?.initialLoaded) return;
    if (state.loadingInitial || state.loadingMore) return;
    if (!state.hasMore || !state.nextCursor) return;
    if (inFlightTransactionScopeKeys.has(scopePageKey)) return;

    inFlightTransactionScopeKeys.add(scopePageKey);
    updateScopeTxPageState(scopePageKey, (previous) => ({
      ...previous,
      loadingMore: true,
      loadMoreError: ''
    }));

    try {
      const page = await walletService.getTransactionHistoryPage(
        scope.channelId,
        currentCoinId,
        state.nextCursor,
        TRANSACTION_PAGE_SIZE
      );
      updateScopeTxPageState(scopePageKey, (previous) => ({
        ...previous,
        items: dedupeTransactions([...previous.items, ...page.transactions]),
        nextCursor: page.nextCursor ?? null,
        hasMore: page.hasMore,
        loadingMore: false,
        loadMoreError: ''
      }));
    } catch (error) {
      updateScopeTxPageState(scopePageKey, (previous) => ({
        ...previous,
        loadingMore: false,
        loadMoreError: mapWalletError(error)
      }));
    } finally {
      inFlightTransactionScopeKeys.delete(scopePageKey);
      updateTxScrollAffordance();
    }
  }

  async function maybeLoadMoreTransactions(): Promise<void> {
    if (!selectedScope || !coin || !txScrollElement) return;
    const state = selectedScopeTxPage;
    if (!state) return;
    if (!state.initialLoaded || state.loadingInitial || state.loadingMore) return;
    if (!state.hasMore || !state.nextCursor) return;

    const remainingScrollDistance =
      txScrollElement.scrollHeight - txScrollElement.scrollTop - txScrollElement.clientHeight;
    if (remainingScrollDistance > TRANSACTION_LOAD_MORE_THRESHOLD_PX) return;

    await loadMoreTransactionsForScope(selectedScope, coin.id);
  }

  async function retryTransactions(): Promise<void> {
    const activeScope = selectedScope;
    const currentCoin = coin;
    if (!activeScope || !currentCoin) return;
    const scopePageKey = getScopePageKey(activeScope.channelId, currentCoin.id);
    txPagesByScopeKey = {
      ...txPagesByScopeKey,
      [scopePageKey]: createEmptyPageState()
    };
    await ensureInitialTransactionsForScope(activeScope, currentCoin.id);
  }

  async function retryLoadMoreTransactions(): Promise<void> {
    const activeScope = selectedScope;
    const currentCoin = coin;
    if (!activeScope || !currentCoin) return;
    await loadMoreTransactionsForScope(activeScope, currentCoin.id);
  }

  function selectScope(scope: CoinScope): void {
    setSelectedScopeAddress(coinId, scope.address);
    setSelectedScopeSystem(coinId, scope.systemId);
    showScopeSheet = false;
    addressSearchTerm = '';
  }

  async function copyAddress(address: string, key: string): Promise<void> {
    if (!address) return;
    try {
      await globalThis.navigator.clipboard.writeText(address);
      copiedAddressKey = key;
      setTimeout(() => {
        if (copiedAddressKey === key) copiedAddressKey = null;
      }, 1800);
    } catch {
      copiedAddressKey = null;
    }
  }

  function updateTxScrollAffordance(): void {
    if (!txScrollElement) {
      canScrollTxDown = false;
      return;
    }
    const maxScrollTop = Math.max(0, txScrollElement.scrollHeight - txScrollElement.clientHeight);
    canScrollTxDown = maxScrollTop > 1 && txScrollElement.scrollTop < maxScrollTop - 1;
  }

  function onTxScroll(event: Event): void {
    const target = event.currentTarget;
    if (!(target instanceof HTMLElement)) return;
    const maxScrollTop = Math.max(0, target.scrollHeight - target.clientHeight);
    canScrollTxDown = maxScrollTop > 1 && target.scrollTop < maxScrollTop - 1;
    void maybeLoadMoreTransactions();
  }

  function toTransferContext(scope: CoinScope): TransferEntryContext {
    return {
      coinId,
      channelId: scope.channelId,
      readOnly: scope.isReadOnly,
      scopeKind: scope.scopeKind
    };
  }

  function formatCryptoAmount(value: number, ticker: string, decimals: number): string {
    const maxFractionDigits = Math.max(2, Math.min(8, decimals));
    const minimumFractionDigits = value === 0 ? 0 : Math.min(2, maxFractionDigits);
    const formatted = i18n.formatNumber(value, {
      minimumFractionDigits,
      maximumFractionDigits: maxFractionDigits
    });
    return `${formatted} ${ticker}`;
  }

  function formatCryptoValue(value: number, decimals: number): string {
    const maxFractionDigits = Math.max(2, Math.min(8, decimals));
    const minimumFractionDigits = value === 0 ? 0 : Math.min(2, maxFractionDigits);
    return i18n.formatNumber(value, {
      minimumFractionDigits,
      maximumFractionDigits: maxFractionDigits
    });
  }

  function getChangeDirection(changePct: number | null): 'up' | 'down' | 'flat' | 'none' {
    if (changePct === null) return 'none';
    if (Math.abs(changePct) < 0.01) return 'flat';
    if (changePct > 0) return 'up';
    return 'down';
  }

  function formatPercentChange(changePct: number): string {
    const formatted = i18n.formatNumber(Math.abs(changePct), {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    });
    if (changePct > 0) return `+${formatted}%`;
    if (changePct < 0) return `-${formatted}%`;
    return `${formatted}%`;
  }

  function formatSyncPercent(percent: number): string {
    const clamped = Math.max(0, Math.min(percent, 100));
    if (clamped > 0 && clamped < 1) return '<1';
    const floored = Math.floor(clamped * 10) / 10;
    return i18n.formatNumber(floored, {
      minimumFractionDigits: 0,
      maximumFractionDigits: 1
    });
  }

  function toFiniteNumber(value: unknown): number | null {
    if (typeof value === 'number') {
      return Number.isFinite(value) ? value : null;
    }

    if (typeof value === 'string') {
      const parsed = Number(value.trim());
      return Number.isFinite(parsed) ? parsed : null;
    }

    return null;
  }

  function transactionDirection(transaction: Transaction): 'in' | 'out' {
    const amount = toFiniteNumber(transaction.amount) ?? 0;
    return amount >= 0 ? 'in' : 'out';
  }

  function transactionAmountDisplay(transaction: Transaction): string {
    const amount = Math.abs(toFiniteNumber(transaction.amount) ?? 0);
    const ticker = coinPresentation?.displayTicker ?? coin?.displayTicker ?? coinId;
    return `${transactionDirection(transaction) === 'in' ? '+' : '-'}${formatCryptoAmount(amount, ticker, coin?.decimals ?? 8)}`;
  }

  function transactionCounterpartyRaw(transaction: Transaction): string {
    const direction = transactionDirection(transaction);
    return direction === 'in' ? transaction.fromAddress.trim() : transaction.toAddress.trim();
  }

  function transactionCounterparty(transaction: Transaction): string {
    const direction = transactionDirection(transaction);
    const counterparty = transactionCounterpartyRaw(transaction);
    if (!counterparty) {
      return direction === 'in'
        ? i18n.t('wallet.assetDetails.receivedFallback')
        : i18n.t('wallet.assetDetails.sentFallback');
    }

    return truncateMiddle(counterparty, 10, 10);
  }

  function transactionCounterpartyIsAddress(transaction: Transaction): boolean {
    return transactionCounterpartyRaw(transaction).length > 0;
  }

  function truncateMiddle(value: string, start = 8, end = 8): string {
    if (value.length <= start + end + 3) return value;
    return `${value.slice(0, start)}...${value.slice(-end)}`;
  }

  function formatTimestamp(transaction: Transaction): string {
    if (transaction.pending) return i18n.t('wallet.assetDetails.pendingLabel');

    const timestamp = transaction.timestamp;
    if (!timestamp || timestamp <= 0) return '—';

    return i18n.formatDate(timestamp * 1000, {
      day: '2-digit',
      month: 'short',
      hour: '2-digit',
      minute: '2-digit'
    });
  }

  function networkLabelForScope(scope: CoinScope): string {
    const ticker = scope.systemTicker.trim();
    if (ticker) return ticker;
    const displayName = scope.systemDisplayName.trim();
    return displayName || '—';
  }

  function scopeAmountValue(scope: CoinScope): number | null {
    if (!coin) return null;
    return toFiniteNumber(getBalance(scope.channelId, coin.id, balances)?.total);
  }

  function scopeCryptoAmountDisplay(scope: CoinScope): string {
    const value = scopeAmountValue(scope);
    if (value === null) return '—';
    const ticker = coinPresentation?.displayTicker ?? coin?.displayTicker ?? coinId;
    return formatCryptoAmount(value, ticker, coin?.decimals ?? 8);
  }

  function scopeFiatAmountDisplay(scope: CoinScope): string {
    const value = scopeAmountValue(scope);
    if (value === null || selectedUsdRate === null) return '—';
    return formatUsdAmount(value * selectedUsdRate, i18n.intlLocale);
  }

  function mapWalletError(error: unknown): string {
    const errorType = extractWalletErrorType(error);
    const rawMessage = extractWalletErrorMessage(error);

    if (errorType === 'OperationFailed') {
      if (rawMessage && rawMessage.toLowerCase() !== 'operation failed') return rawMessage;
    }

    if (rawMessage) return rawMessage;

    return i18n.t('common.unknownError');
  }

</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-6xl flex-col px-6 pb-6 pt-0 sm:px-8">
  <section class="flex min-h-0 flex-1 flex-col overflow-hidden">
    {#if scopesLoading}
      <div class="space-y-4 pt-3">
        <Skeleton class="h-12 w-56 rounded-lg" />
        <Skeleton class="h-9 w-32 rounded-lg" />
        <Skeleton class="h-10 w-full rounded-lg" />
        <Skeleton class="h-10 w-full rounded-lg" />
        <Skeleton class="h-48 w-full rounded-xl" />
      </div>
    {:else if scopesError}
      <div class="mt-4 rounded-lg bg-destructive/10 px-4 py-3 text-sm text-destructive">
        <p>{i18n.t('wallet.assetDetails.errorLoadScopes')}</p>
        <p class="mt-1 text-xs">{scopesError}</p>
        <Button variant="secondary" size="sm" class="mt-3" onclick={loadScopes}>
          {i18n.t('common.retry')}
        </Button>
      </div>
    {:else if !coin || !selectedScope}
      <div class="mt-8 rounded-lg bg-muted/45 px-4 py-5 text-sm text-muted-foreground">
        {i18n.t('wallet.assetDetails.scopeUnavailable')}
      </div>
    {:else}
      <div class="min-h-0 flex flex-1 flex-col gap-4 pt-0">
        <div class="px-0 py-1">
          <div class="flex items-start justify-between gap-4">
            <div class="min-w-0 flex items-center gap-3">
              <CoinIcon
                coinId={coin.id}
                coinName={coinPresentation?.displayName}
                proto={coin.proto}
                size={44}
                showBadge
                privateMuted={usePrivateMutedIcon}
                decorative
              />
              <div class="min-w-0">
                <p class="truncate text-lg font-semibold">
                  {#if usePrivateWordmark}
                    <PrivateVerusWordmark label={headerDisplayName} />
                  {:else}
                    {headerDisplayName}
                  {/if}
                  {#if headerFqnDisplay}
                    <span class="text-muted-foreground ml-2 text-sm font-medium">{headerFqnDisplay}</span>
                  {/if}
                </p>
                <div class="mt-0.5 flex items-center gap-2">
                  <p class="text-muted-foreground text-sm">{selectedUnitRateDisplay}</p>
                  <p
                    class={`text-sm font-semibold ${
                      selectedChange24hDirection === 'up'
                        ? 'text-emerald-700 dark:text-emerald-300'
                        : selectedChange24hDirection === 'down'
                          ? 'text-destructive'
                          : 'text-muted-foreground'
                    }`}
                  >
                    {selectedChange24hDisplay}
                  </p>
                </div>
              </div>
            </div>

            <div class="shrink-0 text-right">
              {#if selectedSyncPercentDisplay}
                <p class="text-foreground text-2xl leading-tight font-semibold tracking-tight">
                  <span class="inline-flex items-center justify-end gap-1.5">
                    <Spinner class="size-4" />
                    <span>{selectedSyncPercentDisplay}</span>
                  </span>
                </p>
                <p class="text-muted-foreground mt-1 text-[11px]">
                  {i18n.t('wallet.assetDetails.privateSyncInlineHelper')}
                </p>
              {:else}
                <p class="text-foreground text-2xl leading-tight font-semibold tracking-tight">
                  {selectedCryptoAmountDisplay}
                </p>
                <p class="text-muted-foreground mt-1 text-sm leading-tight">{selectedFiatDisplay}</p>
              {/if}
              {#if loadingSelectedBalance}
                <p class="text-muted-foreground mt-1 text-[11px]">{i18n.t('common.loading')}</p>
              {/if}
            </div>
          </div>

          <div class="mt-8 flex items-center gap-2">
            {#if useStaticAddressRow}
              <div class="flex h-[52px] min-w-0 flex-1 items-center justify-between gap-2 rounded-md bg-muted/55 pl-3 pr-1.5">
                <p class="identifier-text truncate text-sm font-medium text-foreground">
                  {truncateMiddle(selectedAddress || '—', 10, 10)}
                </p>
                <button
                  type="button"
                  class="text-muted-foreground hover:text-foreground focus-visible:ring-ring/50 -mr-0.5 h-8 w-8 shrink-0 rounded-sm transition-colors focus-visible:outline-none focus-visible:ring-2"
                  onclick={() => copyAddress(selectedAddress, 'selected-static')}
                  title={i18n.t('wallet.receive.copy')}
                  aria-label={i18n.t('wallet.receive.copy')}
                >
                  {#if copiedAddressKey === 'selected-static'}
                    <CheckIcon class="size-4 text-emerald-600 dark:text-emerald-400" />
                  {:else}
                    <CopyIcon class="size-4" />
                  {/if}
                </button>
              </div>
            {:else}
              <div class="bg-primary flex h-[52px] min-w-0 flex-1 items-center gap-1 rounded-md pl-1.5 pr-1">
                <button
                  type="button"
                  class="hover:bg-primary/90 focus-visible:ring-primary-foreground/60 flex min-w-0 flex-1 items-center justify-between gap-2 rounded-md px-2 py-1 text-left transition-colors focus-visible:outline-none focus-visible:ring-2"
                  aria-label={i18n.t('wallet.assetDetails.scopePicker')}
                  title={i18n.t('wallet.assetDetails.scopePicker')}
                  onclick={() => (showScopeSheet = true)}
                >
                  <div class="min-w-0 flex-1 text-left">
                    <p class="identifier-text truncate text-sm font-medium text-primary-foreground">
                      {truncateMiddle(selectedAddress || '—', 10, 10)}
                      <span class="ml-1.5 font-normal text-primary-foreground/80">• {selectedNetworkDisplay}</span>
                    </p>
                    <p class="mt-0.5 truncate text-xs text-primary-foreground/80">
                      {selectedCryptoAmountDisplay}
                      <span class="mx-1.5">•</span>
                      {selectedFiatDisplay}
                    </p>
                  </div>
                  <ChevronDownIcon class="h-4 w-4 shrink-0 text-primary-foreground/80" />
                </button>
                <button
                  type="button"
                  class="text-primary-foreground/75 hover:text-primary-foreground focus-visible:ring-primary-foreground/60 -mr-0.5 h-8 w-8 shrink-0 rounded-sm transition-colors focus-visible:outline-none focus-visible:ring-2"
                  onclick={() => copyAddress(selectedAddress, 'selected-interactive')}
                  title={i18n.t('wallet.receive.copy')}
                  aria-label={i18n.t('wallet.receive.copy')}
                >
                  {#if copiedAddressKey === 'selected-interactive'}
                    <CheckIcon class="size-4 text-emerald-300 dark:text-emerald-200" />
                  {:else}
                    <CopyIcon class="size-4" />
                  {/if}
                </button>
              </div>
            {/if}

            <div class="flex items-center gap-2">
              <Button
                variant="secondary"
                size="icon-lg"
                class="size-[52px] rounded-md"
                aria-label={i18n.t('wallet.overview.receive')}
                title={i18n.t('wallet.overview.receive')}
                onclick={onNavigateToReceive}
              >
                <DownloadIcon class="h-[18px] w-[18px]" />
              </Button>
              <Button
                variant="secondary"
                size="icon-lg"
                class="size-[52px] rounded-md"
                disabled={!canSendOrConvert}
                aria-label={i18n.t('wallet.overview.send')}
                title={i18n.t('wallet.overview.send')}
                onclick={() => {
                  if (!selectedScope || !canSendOrConvert) return;
                  onNavigateToSend(toTransferContext(selectedScope));
                }}
              >
                <SendIcon class="h-[18px] w-[18px]" />
              </Button>
              <Button
                variant="secondary"
                size="icon-lg"
                class="size-[52px] rounded-md"
                disabled={!canSendOrConvert}
                aria-label={i18n.t('wallet.overview.convert')}
                title={i18n.t('wallet.overview.convert')}
                onclick={() => {
                  if (!selectedScope || !canSendOrConvert) return;
                  onNavigateToConvert(toTransferContext(selectedScope));
                }}
              >
                <ArrowLeftRightIcon class="h-[18px] w-[18px]" />
              </Button>
            </div>
          </div>

          {#if !canSendOrConvert && !isShieldedSyncBlocked}
            <p class="text-muted-foreground mt-2 text-right text-[11px] leading-snug">
              {i18n.t('wallet.assetDetails.readOnlyHelper')}
            </p>
          {/if}
        </div>

        <div class="relative min-h-0 flex flex-1 flex-col">
          <div class="px-0 pb-2 pt-2">
            <p class="text-sm font-medium">{i18n.t('wallet.assetDetails.transactions')}</p>
          </div>

          <ScrollArea.Root class="h-full min-h-0 flex-1" type="scroll">
            <ScrollArea.Viewport
              class="asset-tx-scroll h-full overscroll-contain pr-5"
              bind:ref={txScrollElement}
              onscroll={onTxScroll}
            >
              {#if loadingSelectedTransactions && sortedSelectedTransactions.length === 0}
                <ul class="space-y-2 py-2 pr-1">
                  {#each initialTransactionSkeletonRows as skeletonRow (skeletonRow)}
                    <li class="flex items-center justify-between rounded-md py-2">
                      <div class="min-w-0 flex-1">
                        <Skeleton class="h-4 w-40 rounded-sm" />
                        <Skeleton class="mt-2 h-3 w-28 rounded-sm" />
                      </div>
                      <div class="ml-4 min-w-[9rem] text-right">
                        <Skeleton class="ml-auto h-4 w-24 rounded-sm" />
                        <Skeleton class="mt-2 ml-auto h-3 w-20 rounded-sm" />
                      </div>
                    </li>
                  {/each}
                </ul>
              {:else if selectedScopeTransactionsError}
                <div class="py-5">
                  <p class="text-sm text-destructive">{i18n.t('wallet.assetDetails.errorLoadTransactions')}</p>
                  <p class="text-muted-foreground mt-1 text-xs">{selectedScopeTransactionsError}</p>
                  <Button variant="secondary" size="sm" class="mt-3" onclick={retryTransactions}>
                    {i18n.t('common.retry')}
                  </Button>
                </div>
              {:else if sortedSelectedTransactions.length === 0}
                <p class="text-muted-foreground py-6 text-sm">{i18n.t('wallet.assetDetails.noTransactionsForScope')}</p>
              {:else}
                <ul class="space-y-1.5 py-2 pr-1">
                  {#each sortedSelectedTransactions as transaction (transaction.txid)}
                    <li class="flex items-center justify-between rounded-md px-0 py-2 hover:bg-muted/45">
                      <div class="min-w-0">
                        <p
                          class={`truncate text-sm font-medium ${transactionCounterpartyIsAddress(transaction) ? 'identifier-text' : ''}`}
                        >
                          {transactionCounterparty(transaction)}
                        </p>
                        <p class="text-muted-foreground mt-0.5 text-xs">
                          {formatTimestamp(transaction)}
                        </p>
                      </div>
                      <div class="text-right">
                        <p class={`text-sm font-semibold ${transactionDirection(transaction) === 'in' ? 'text-emerald-700 dark:text-emerald-300' : 'text-foreground'}`}>
                          {transactionAmountDisplay(transaction)}
                        </p>
                        <p class="text-muted-foreground identifier-text mt-0.5 text-[11px]">
                          {truncateMiddle(transaction.txid, 8, 8)}
                        </p>
                      </div>
                    </li>
                  {/each}
                </ul>

                {#if loadingMoreTransactions}
                  <ul class="space-y-2 pb-2 pr-1">
                    {#each loadMoreTransactionSkeletonRows as skeletonRow (skeletonRow)}
                      <li class="flex items-center justify-between rounded-md py-2">
                        <div class="min-w-0 flex-1">
                          <Skeleton class="h-4 w-36 rounded-sm" />
                          <Skeleton class="mt-2 h-3 w-24 rounded-sm" />
                        </div>
                        <div class="ml-4 min-w-[8.5rem] text-right">
                          <Skeleton class="ml-auto h-4 w-20 rounded-sm" />
                          <Skeleton class="mt-2 ml-auto h-3 w-16 rounded-sm" />
                        </div>
                      </li>
                    {/each}
                  </ul>
                {/if}

                {#if selectedScopeLoadMoreError}
                  <div class="pb-3 pr-1">
                    <p class="text-xs text-destructive">{i18n.t('wallet.assetDetails.errorLoadMoreTransactions')}</p>
                    <p class="text-muted-foreground mt-1 text-xs">{selectedScopeLoadMoreError}</p>
                    <Button variant="secondary" size="sm" class="mt-2" onclick={retryLoadMoreTransactions}>
                      {i18n.t('common.retry')}
                    </Button>
                  </div>
                {/if}
              {/if}
            </ScrollArea.Viewport>
            <ScrollArea.Scrollbar orientation="vertical" />
          </ScrollArea.Root>

          {#if canScrollTxDown}
            <div
              class="pointer-events-none absolute inset-x-0 bottom-0 h-14 bg-gradient-to-t from-background to-transparent dark:from-[#111111]"
            ></div>
          {/if}
        </div>
      </div>
    {/if}
  </section>
</div>

<StandardRightSheet bind:isOpen={showScopeSheet} title={i18n.t('wallet.assetDetails.scopeSheetTitle')}>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <SearchInput
      bind:value={addressSearchTerm}
      placeholder={i18n.t('wallet.assetDetails.scopeSearchPlaceholder')}
    />
    <ScrollArea.Root class="min-h-0 flex-1" type="scroll">
      <ScrollArea.Viewport class="h-full pr-1">
        {#if filteredScopeOptions.length === 0}
          <p class="text-muted-foreground px-1 py-4 text-sm">{i18n.t('wallet.assetDetails.noScopeMatches')}</p>
        {:else}
          <ul class="space-y-1.5 pb-2">
            {#each filteredScopeOptions as scopeOption (scopeOption.channelId)}
              <li>
                <div class="hover:bg-muted/50 flex items-center gap-1 rounded-md pr-0.5">
                  <button
                    type="button"
                    class="focus-visible:ring-ring/60 flex min-w-0 flex-1 items-center justify-between rounded-md px-3 py-2.5 text-left outline-none focus-visible:ring-2"
                    onclick={() => selectScope(scopeOption)}
                  >
                    <div class="min-w-0">
                      <p class="identifier-text truncate text-sm">{truncateMiddle(scopeOption.addressLabel, 10, 10)}</p>
                      <p class="text-muted-foreground mt-0.5 text-xs">
                        {networkLabelForScope(scopeOption)}
                        <span class="mx-1.5">•</span>
                        {scopeCryptoAmountDisplay(scopeOption)}
                        <span class="mx-1.5">•</span>
                        {scopeFiatAmountDisplay(scopeOption)}
                      </p>
                    </div>

                    <div class="flex items-center gap-1.5">
                      {#if scopeOption.address === selectedAddress && scopeOption.systemId === selectedSystem}
                        <CheckIcon class="text-primary h-4 w-4" />
                      {/if}
                    </div>
                  </button>
                  <button
                    type="button"
                    class="text-muted-foreground hover:text-foreground focus-visible:ring-ring/50 -mr-0.5 h-8 w-8 shrink-0 rounded-sm transition-colors focus-visible:outline-none focus-visible:ring-2"
                    onclick={() => copyAddress(scopeOption.address, `scope:${scopeOption.channelId}`)}
                    title={i18n.t('wallet.receive.copy')}
                    aria-label={i18n.t('wallet.receive.copy')}
                  >
                    {#if copiedAddressKey === `scope:${scopeOption.channelId}`}
                      <CheckIcon class="size-4 text-emerald-600 dark:text-emerald-400" />
                    {:else}
                      <CopyIcon class="size-4" />
                    {/if}
                  </button>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </ScrollArea.Viewport>
      <ScrollArea.Scrollbar orientation="vertical" />
    </ScrollArea.Root>
  </div>
</StandardRightSheet>

<style>
  .asset-tx-scroll {
    scrollbar-gutter: stable;
  }
</style>
