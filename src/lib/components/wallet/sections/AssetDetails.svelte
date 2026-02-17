<!--
  Component: AssetDetails
  Purpose: Coin detail view with scope selectors (address × network), scoped actions, and scoped transaction history.
-->

<script lang="ts">
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import SendIcon from '@lucide/svelte/icons/send';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import ArrowLeftRightIcon from '@lucide/svelte/icons/arrow-left-right';
  import CheckIcon from '@lucide/svelte/icons/check';
  import Link2Icon from '@lucide/svelte/icons/link-2';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { Skeleton } from '$lib/components/ui/skeleton/index.js';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import { i18nStore } from '$lib/i18n';
  import { resolveCoinPresentation } from '$lib/coins/presentation.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { balanceStore, getBalance } from '$lib/stores/balances.js';
  import { ratesStore } from '$lib/stores/rates.js';
  import { getTransactions, transactionStore } from '$lib/stores/transactions.js';
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
  import type { CoinScope, Transaction } from '$lib/types/wallet.js';
  import type { TransferEntryContext } from './transfer-wizard/types';

  type AssetDetailsProps = {
    coinId: string;
    onBack?: () => void;
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
    onBack = noop,
    onNavigateToReceive = noop,
    onNavigateToSend = noop,
    onNavigateToConvert = noop
  }: AssetDetailsProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);
  const coins = $derived($coinsStore);
  const balances = $derived($balanceStore);
  const rates = $derived($ratesStore);
  const transactionsByChannel = $derived($transactionStore);
  const allScopesByCoinId = $derived($scopesByCoinId);
  const selectedAddressMap = $derived($selectedAddressByCoinId);
  const selectedSystemMap = $derived($selectedSystemByCoinId);

  let scopesLoading = $state(false);
  let scopesError = $state('');
  let transactionsError = $state('');
  let loadingSelectedBalance = $state(false);
  let loadingSelectedTransactions = $state(false);
  let showAddressSheet = $state(false);
  let addressSearchTerm = $state('');

  let loadedBalanceByChannel = $state<Record<string, boolean>>({});
  let loadedTransactionsByChannel = $state<Record<string, boolean>>({});

  let scopeRequestSequence = 0;
  let selectedBalanceRequestSequence = 0;
  const inFlightBalanceChannels = new Set<string>();
  const inFlightTransactionChannels = new Set<string>();

  const coin = $derived(coins.find((item) => item.id === coinId) ?? null);
  const coinPresentation = $derived((coin ? resolveCoinPresentation(coin) : null));
  const allScopes = $derived(allScopesByCoinId[coinId] ?? []);

  const selectedAddress = $derived(selectedAddressMap[coinId] ?? '');
  const selectedSystem = $derived(selectedSystemMap[coinId] ?? '');

  const addressOptions = $derived(
    (() => {
      const byAddress = new Map<string, CoinScope>();
      for (const scope of allScopes) {
        if (!byAddress.has(scope.address)) {
          byAddress.set(scope.address, scope);
        }
      }

      return [...byAddress.values()];
    })()
  );

  const filteredAddressOptions = $derived(
    (() => {
      const query = addressSearchTerm.trim().toLowerCase();
      if (!query) return addressOptions;
      return addressOptions.filter((scope) => {
        return (
          scope.address.toLowerCase().includes(query) ||
          scope.addressLabel.toLowerCase().includes(query)
        );
      });
    })()
  );

  const chainOptionsForSelectedAddress = $derived(
    allScopes.filter((scope) => scope.address === selectedAddress)
  );

  const selectedScope = $derived(
    allScopes.find((scope) => scope.address === selectedAddress && scope.systemId === selectedSystem) ??
      null
  );

  const selectedScopeBalance = $derived(
    selectedScope && coin ? getBalance(selectedScope.channelId, coin.id, balances) : undefined
  );

  const selectedScopeTransactions = $derived(
    selectedScope && coin ? getTransactions(selectedScope.channelId, coin.id, transactionsByChannel) : []
  );

  const sortedSelectedTransactions = $derived(
    [...selectedScopeTransactions].sort((left, right) => {
      const pendingOrder = Number(right.pending) - Number(left.pending);
      if (pendingOrder !== 0) return pendingOrder;
      return (right.timestamp ?? 0) - (left.timestamp ?? 0);
    })
  );

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

  const canSendOrConvert = $derived(!!selectedScope && !selectedScope.isReadOnly);
  const showChainSelector = $derived(chainOptionsForSelectedAddress.length > 1);
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

  $effect(() => {
    coinId;
    addressSearchTerm = '';
    showAddressSheet = false;
    scopesError = '';
    transactionsError = '';
    void loadScopes();
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
    const activeScope = selectedScope;
    if (!currentCoin || !activeScope) return;

    const hasExistingTransactions =
      transactionsByChannel[activeScope.channelId]?.[currentCoin.id] !== undefined;
    if (hasExistingTransactions || loadedTransactionsByChannel[activeScope.channelId]) {
      if (hasExistingTransactions && !loadedTransactionsByChannel[activeScope.channelId]) {
        loadedTransactionsByChannel = {
          ...loadedTransactionsByChannel,
          [activeScope.channelId]: true
        };
      }
      loadingSelectedTransactions = false;
      transactionsError = '';
      return;
    }

    loadingSelectedTransactions = true;
    transactionsError = '';
    void fetchTransactionsForScope(activeScope, currentCoin.id);
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

  async function fetchTransactionsForScope(scope: CoinScope, currentCoinId: string): Promise<void> {
    if (inFlightTransactionChannels.has(scope.channelId)) return;
    inFlightTransactionChannels.add(scope.channelId);

    try {
      const transactions = await walletService.getTransactionHistory(scope.channelId, currentCoinId);
      transactionStore.update((state) => ({
        ...state,
        [scope.channelId]: {
          ...(state[scope.channelId] ?? {}),
          [currentCoinId]: transactions
        }
      }));
      loadedTransactionsByChannel = {
        ...loadedTransactionsByChannel,
        [scope.channelId]: true
      };
      transactionsError = '';
    } catch (error) {
      transactionsError = mapWalletError(error);
    } finally {
      loadingSelectedTransactions = false;
      inFlightTransactionChannels.delete(scope.channelId);
    }
  }

  async function retryTransactions(): Promise<void> {
    const activeScope = selectedScope;
    const currentCoin = coin;
    if (!activeScope || !currentCoin) return;
    loadingSelectedTransactions = true;
    transactionsError = '';
    await fetchTransactionsForScope(activeScope, currentCoin.id);
  }

  function selectAddress(address: string): void {
    if (!address) return;
    setSelectedScopeAddress(coinId, address);

    const firstScope = allScopes.find((scope) => scope.address === address);
    if (firstScope) {
      const rootSystemId = coin?.systemId ?? '';
      const matchingRoot = allScopes.find(
        (scope) => scope.address === address && scope.systemId === rootSystemId
      );
      setSelectedScopeSystem(coinId, matchingRoot?.systemId ?? firstScope.systemId);
    }

    showAddressSheet = false;
    addressSearchTerm = '';
  }

  function selectSystem(systemId: string): void {
    if (!systemId) return;
    setSelectedScopeSystem(coinId, systemId);
  }

  function toTransferContext(scope: CoinScope): TransferEntryContext {
    return {
      coinId,
      channelId: scope.channelId,
      readOnly: scope.isReadOnly
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

  function transactionCounterparty(transaction: Transaction): string {
    const direction = transactionDirection(transaction);
    const counterparty =
      direction === 'in' ? transaction.fromAddress.trim() : transaction.toAddress.trim();
    if (!counterparty) {
      return direction === 'in'
        ? i18n.t('wallet.assetDetails.receivedFallback')
        : i18n.t('wallet.assetDetails.sentFallback');
    }

    return truncateMiddle(counterparty, 10, 10);
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

  function mapWalletError(error: unknown): string {
    const errorType = extractWalletErrorType(error);
    const rawMessage = extractWalletErrorMessage(error);

    if (errorType === 'OperationFailed') {
      if (rawMessage && rawMessage.toLowerCase() !== 'operation failed') return rawMessage;
    }

    if (rawMessage) return rawMessage;

    return i18n.t('common.unknownError');
  }

  const selectedAddressScope = $derived(
    addressOptions.find((scope) => scope.address === selectedAddress) ?? null
  );
</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-6xl flex-col px-6 pb-6 pt-0 sm:px-8">
  <section class="flex min-h-0 flex-1 flex-col overflow-hidden">
    <div class="flex items-center justify-between gap-3 pb-2 pt-0">
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground inline-flex items-center gap-1.5 text-sm transition-colors"
        onclick={onBack}
      >
        <ArrowLeftIcon class="size-4" />
        {i18n.t('wallet.assetDetails.back')}
      </button>

      {#if !scopesLoading && !scopesError && coin && selectedScope}
        <div class="flex items-center gap-2">
          <Button
            variant="secondary"
            size="icon-lg"
            class="size-10 rounded-md"
            aria-label={i18n.t('wallet.overview.receive')}
            title={i18n.t('wallet.overview.receive')}
            onclick={onNavigateToReceive}
          >
            <DownloadIcon class="h-[18px] w-[18px]" />
          </Button>
          <Button
            variant="secondary"
            size="icon-lg"
            class="size-10 rounded-md"
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
            class="size-10 rounded-md"
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
      {/if}
    </div>

    {#if !canSendOrConvert && !scopesLoading && !scopesError && coin && selectedScope}
      <p class="text-muted-foreground mb-1 self-end text-right text-[11px] leading-snug">
        {i18n.t('wallet.assetDetails.readOnlyHelper')}
      </p>
    {/if}

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
      <div class="min-h-0 flex flex-1 flex-col gap-4 pt-1">
        <div class="px-0 py-1">
          <div class="flex items-start justify-between gap-4">
            <div class="min-w-0 flex items-center gap-3">
              <CoinIcon
                coinId={coin.id}
                coinName={coinPresentation?.displayName}
                proto={coin.proto}
                size={44}
                showBadge
                decorative
              />
              <div class="min-w-0">
                <p class="truncate text-lg font-semibold">
                  {coinPresentation?.displayName ?? coin.displayName}
                  {#if selectedFqnDisplay}
                    <span class="text-muted-foreground ml-2 text-sm font-medium">{selectedFqnDisplay}</span>
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

            <div class="text-right">
              <p class="text-foreground text-2xl leading-tight font-semibold tracking-tight">{selectedCryptoAmountDisplay}</p>
              <p class="text-muted-foreground mt-1 text-sm leading-tight">{selectedFiatDisplay}</p>
              {#if loadingSelectedBalance}
                <p class="text-muted-foreground mt-1 text-[11px]">{i18n.t('common.loading')}</p>
              {/if}
            </div>
          </div>
        </div>

        <div class="space-y-3 px-0 py-1">
          <div class="flex flex-wrap items-center gap-2">
            <p class="text-muted-foreground text-xs font-medium">{i18n.t('wallet.assetDetails.address')}</p>
            {#if selectedAddressScope?.isPrimaryAddress}
              <span class="rounded-full bg-muted px-2 py-0.5 text-[11px] font-medium">
                {i18n.t('wallet.assetDetails.primaryAddress')}
              </span>
            {:else}
              <span class="rounded-full bg-amber-100 px-2 py-0.5 text-[11px] font-medium text-amber-900 dark:bg-amber-500/20 dark:text-amber-200">
                {i18n.t('wallet.assetDetails.readOnly')}
              </span>
            {/if}
          </div>
          <Button
            variant="secondary"
            class="h-10 w-full justify-between rounded-md px-3 font-mono text-xs sm:text-sm"
            onclick={() => (showAddressSheet = true)}
          >
            <span class="truncate">{truncateMiddle(selectedAddress, 12, 12)}</span>
            <span class="text-muted-foreground text-xs">{i18n.t('wallet.assetDetails.changeAddress')}</span>
          </Button>

          {#if showChainSelector}
            <div class="space-y-2">
              <p class="text-muted-foreground text-xs font-medium">{i18n.t('wallet.assetDetails.chain')}</p>
              <div class="flex flex-wrap gap-2">
                {#each chainOptionsForSelectedAddress as scopeOption (scopeOption.channelId)}
                  <Button
                    size="sm"
                    variant={scopeOption.systemId === selectedSystem ? 'default' : 'secondary'}
                    class="h-8 rounded-full px-3 text-xs"
                    onclick={() => selectSystem(scopeOption.systemId)}
                  >
                    {scopeOption.systemDisplayName}
                  </Button>
                {/each}
              </div>
            </div>
          {/if}
        </div>

        <div class="relative min-h-0 flex-1">
          <div class="px-0 pb-2 pt-2">
            <p class="text-sm font-medium">{i18n.t('wallet.assetDetails.transactions')}</p>
            <div class="text-muted-foreground mt-1 flex items-center gap-1.5 text-[11px]">
              <Link2Icon class="size-3.5" />
              <span>
                {i18n.t('wallet.assetDetails.connectedToScope', {
                  address: truncateMiddle(selectedScope.address, 8, 8),
                  network: selectedScope.systemDisplayName
                })}
              </span>
            </div>
          </div>

          <ScrollArea.Root class="h-full" type="scroll">
            <ScrollArea.Viewport class="h-full max-h-[40vh]">
              {#if loadingSelectedTransactions && sortedSelectedTransactions.length === 0}
                <p class="text-muted-foreground py-6 text-sm">{i18n.t('wallet.assetDetails.loadingTransactions')}</p>
              {:else if transactionsError}
                <div class="py-5">
                  <p class="text-sm text-destructive">{i18n.t('wallet.assetDetails.errorLoadTransactions')}</p>
                  <p class="text-muted-foreground mt-1 text-xs">{transactionsError}</p>
                  <Button variant="secondary" size="sm" class="mt-3" onclick={retryTransactions}>
                    {i18n.t('common.retry')}
                  </Button>
                </div>
              {:else if sortedSelectedTransactions.length === 0}
                <p class="text-muted-foreground py-6 text-sm">{i18n.t('wallet.assetDetails.noTransactionsForScope')}</p>
              {:else}
                <ul class="space-y-1.5 py-2">
                  {#each sortedSelectedTransactions as transaction (transaction.txid)}
                    <li class="flex items-center justify-between rounded-md px-0 py-2 hover:bg-muted/45">
                      <div class="min-w-0">
                        <p class="truncate text-sm font-medium">{transactionCounterparty(transaction)}</p>
                        <p class="text-muted-foreground mt-0.5 text-xs">
                          {formatTimestamp(transaction)}
                        </p>
                      </div>
                      <div class="text-right">
                        <p class={`text-sm font-semibold ${transactionDirection(transaction) === 'in' ? 'text-emerald-700 dark:text-emerald-300' : 'text-foreground'}`}>
                          {transactionAmountDisplay(transaction)}
                        </p>
                        <p class="text-muted-foreground mt-0.5 text-[11px] font-mono">
                          {truncateMiddle(transaction.txid, 8, 8)}
                        </p>
                      </div>
                    </li>
                  {/each}
                </ul>
              {/if}
            </ScrollArea.Viewport>
            <ScrollArea.Scrollbar orientation="vertical" />
          </ScrollArea.Root>
        </div>
      </div>
    {/if}
  </section>
</div>

<StandardRightSheet bind:isOpen={showAddressSheet} title={i18n.t('wallet.assetDetails.addressSheetTitle')}>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <SearchInput
      bind:value={addressSearchTerm}
      placeholder={i18n.t('wallet.assetDetails.addressSearchPlaceholder')}
    />
    <ScrollArea.Root class="min-h-0 flex-1" type="scroll">
      <ScrollArea.Viewport class="h-full pr-1">
        {#if filteredAddressOptions.length === 0}
          <p class="text-muted-foreground px-1 py-4 text-sm">{i18n.t('wallet.assetDetails.noAddresses')}</p>
        {:else}
          <ul class="space-y-1.5 pb-2">
            {#each filteredAddressOptions as scopeOption (scopeOption.address)}
              <li>
                <button
                  type="button"
                  class="hover:bg-muted/50 focus-visible:ring-ring/60 flex w-full items-center justify-between rounded-md px-3 py-2.5 text-left outline-none focus-visible:ring-2"
                  onclick={() => selectAddress(scopeOption.address)}
                >
                  <div class="min-w-0">
                    <p class="truncate font-mono text-sm">{truncateMiddle(scopeOption.addressLabel, 12, 12)}</p>
                    <p class="text-muted-foreground mt-0.5 text-xs">
                      {scopeOption.isPrimaryAddress
                        ? i18n.t('wallet.assetDetails.primaryAddress')
                        : i18n.t('wallet.assetDetails.readOnlyAddress')}
                    </p>
                  </div>

                  <div class="flex items-center gap-1.5">
                    {#if scopeOption.address === selectedAddress}
                      <CheckIcon class="text-primary h-4 w-4" />
                    {/if}
                  </div>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </ScrollArea.Viewport>
      <ScrollArea.Scrollbar orientation="vertical" />
    </ScrollArea.Root>
  </div>
</StandardRightSheet>
