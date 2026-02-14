<script lang="ts">
  import SearchIcon from '@lucide/svelte/icons/search';
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import AddAssetRow from '$lib/components/wallet/AddAssetRow.svelte';
  import { Input } from '$lib/components/ui/input';
  import { Button } from '$lib/components/ui/button';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import { i18nStore } from '$lib/i18n';
  import { isWalletSupportedAsset } from '$lib/coins/supportedAssets.js';
  import {
    assetVisibilityKey,
    filterVisibleAssets,
    hideAssetByKey,
    isAssetHiddenByKey,
    showAssetByKey
  } from '$lib/stores/assetVisibility.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { buildWalletChannels, walletChannelsStore } from '$lib/stores/walletChannels.js';
  import * as coinsService from '$lib/services/coinsService.js';
  import {
    applyCatalogMetadataToCoinDefinition,
    type AddAssetEntry,
    buildAddAssetCatalogView,
    catalogEntryToCoinDefinition,
    erc20ContractValue,
    pbaasLookupValue
  } from '$lib/stores/addAssetCatalog.js';
  import type { CoinDefinition, PbaasCandidate, WalletNetwork } from '$lib/types/wallet.js';

  /* eslint-disable prefer-const */
  let { isOpen = $bindable(false), network }: { isOpen?: boolean; network: WalletNetwork } = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);
  const coins = $derived($coinsStore);
  const walletChannels = $derived($walletChannelsStore);

  let searchInput = $state('');
  let debouncedSearch = $state('');

  let actionError = $state('');
  let actionSuccess = $state('');
  let activeRowKey = $state<string | null>(null);

  let pbaasQuery = $state('');
  let pbaasResolving = $state(false);
  let pbaasAdding = $state(false);
  let pbaasError = $state('');
  let pbaasResolvedCoin = $state<CoinDefinition | null>(null);
  let pbaasCandidates = $state<PbaasCandidate[]>([]);

  let erc20Contract = $state('');
  let erc20Resolving = $state(false);
  let erc20Adding = $state(false);
  let erc20Error = $state('');
  let erc20ResolvedCoin = $state<CoinDefinition | null>(null);

  const catalogView = $derived(
    buildAddAssetCatalogView({
      coins,
      network,
      query: debouncedSearch
    })
  );

  $effect(() => {
    const query = searchInput;
    const timer = setTimeout(() => {
      debouncedSearch = query;
    }, 150);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (isOpen) return;
    resetSheetState();
  });

  function handleOpenAutoFocus(event: Event) {
    event.preventDefault();
  }

  function resetSheetState() {
    searchInput = '';
    debouncedSearch = '';
    actionError = '';
    actionSuccess = '';
    activeRowKey = null;

    pbaasQuery = '';
    pbaasResolving = false;
    pbaasAdding = false;
    pbaasError = '';
    pbaasResolvedCoin = null;
    pbaasCandidates = [];

    erc20Contract = '';
    erc20Resolving = false;
    erc20Adding = false;
    erc20Error = '';
    erc20ResolvedCoin = null;
  }

  function extractWalletErrorType(error: unknown): string | null {
    if (typeof error === 'string') {
      try {
        const parsed = JSON.parse(error) as unknown;
        if (parsed && typeof parsed === 'object' && 'type' in parsed) {
          const typed = (parsed as { type?: unknown }).type;
          if (typeof typed === 'string') return typed;
        }
      } catch {
        return null;
      }
      return null;
    }

    if (!error || typeof error !== 'object') return null;

    const obj = error as { type?: unknown; data?: unknown; message?: unknown };
    if (typeof obj.type === 'string') return obj.type;
    if (obj.data && typeof obj.data === 'object') {
      const data = obj.data as { type?: unknown };
      if (typeof data.type === 'string') return data.type;
    }
    if (typeof obj.message === 'string') {
      try {
        const parsed = JSON.parse(obj.message) as { type?: unknown };
        if (typeof parsed.type === 'string') return parsed.type;
      } catch {
        return null;
      }
    }

    return null;
  }

  function translateAssetError(error: unknown, fallbackKey: string): string {
    const errorType = extractWalletErrorType(error);

    switch (errorType) {
      case 'AssetAlreadyExists':
      case 'DuplicatePbaasCurrency':
        return i18n.t('wallet.addAsset.error.assetExists');
      case 'PbaasNotFound':
        return i18n.t('wallet.addAsset.error.pbaasNotFound');
      case 'PbaasAmbiguous':
        return i18n.t('wallet.addAsset.error.pbaasAmbiguous');
      case 'InvalidContract':
        return i18n.t('wallet.addAsset.error.invalidContract');
      case 'UnsupportedNetwork':
        return i18n.t('wallet.addAsset.error.unsupportedNetwork');
      case 'EthNotConfigured':
        return i18n.t('wallet.addAsset.error.ethNotConfigured');
      case 'WalletLocked':
        return i18n.t('wallet.addAsset.error.walletLocked');
      default:
        break;
    }

    if (error instanceof Error && error.message.trim().length > 0) {
      return error.message;
    }

    return i18n.t(fallbackKey);
  }

  async function refreshRegistryState() {
    const allCoins = await coinsService.getCoinRegistry();
    const networkCoins = filterVisibleAssets(
      allCoins.filter((coin) => isWalletSupportedAsset(coin, network)),
      network
    );
    coinsStore.set(networkCoins);
    walletChannelsStore.set(buildWalletChannels(networkCoins, walletChannels.vrpcAddress));
  }

  async function addCoinToRegistry(definition: CoinDefinition, successMessageKey: string) {
    const visibilityKey = assetVisibilityKey(definition.id, definition.proto);
    try {
      await coinsService.addCoinDefinition(definition);
      showAssetByKey(visibilityKey, network);
      await refreshRegistryState();
      actionError = '';
      actionSuccess = i18n.t(successMessageKey, { ticker: definition.displayTicker });
    } catch (error) {
      const errorType = extractWalletErrorType(error);
      if (errorType === 'AssetAlreadyExists' && isAssetHiddenByKey(visibilityKey, network)) {
        showAssetByKey(visibilityKey, network);
        await refreshRegistryState();
        actionError = '';
        actionSuccess = i18n.t('wallet.addAsset.toast.enabled', { ticker: definition.displayTicker });
        return;
      }

      throw error;
    }
  }

  async function handleCatalogAction(entry: AddAssetEntry) {
    actionSuccess = '';
    actionError = '';
    activeRowKey = entry.key;

    try {
      if (entry.status === 'added') {
        hideAssetByKey(entry.key, network);
        await refreshRegistryState();
        actionSuccess = i18n.t('wallet.addAsset.toast.disabled', { ticker: entry.displayTicker });
        return;
      }

      if (isAssetHiddenByKey(entry.key, network)) {
        showAssetByKey(entry.key, network);
        await refreshRegistryState();
        actionSuccess = i18n.t('wallet.addAsset.toast.enabled', { ticker: entry.displayTicker });
        return;
      }

      if (entry.addStrategy === 'direct') {
        const definition = catalogEntryToCoinDefinition(entry, network);
        if (!definition) {
          throw new Error(i18n.t('wallet.addAsset.error.addFailed'));
        }
        await addCoinToRegistry(definition, 'wallet.addAsset.toast.added');
      } else if (entry.addStrategy === 'resolve_pbaas') {
        const result = await coinsService.resolvePbaasCurrency(pbaasLookupValue(entry));
        if (result.status === 'ambiguous') {
          throw new Error(i18n.t('wallet.addAsset.error.pbaasAmbiguous'));
        }
        const hydratedCoin = applyCatalogMetadataToCoinDefinition(result.coin);
        await addCoinToRegistry(hydratedCoin, 'wallet.addAsset.toast.added');
      } else if (entry.addStrategy === 'resolve_erc20') {
        const result = await coinsService.resolveErc20Contract(erc20ContractValue(entry));
        const hydratedCoin = applyCatalogMetadataToCoinDefinition(result.coin);
        await addCoinToRegistry(hydratedCoin, 'wallet.addAsset.toast.added');
      }
    } catch (error) {
      actionError = translateAssetError(error, 'wallet.addAsset.error.addFailed');
    } finally {
      activeRowKey = null;
    }
  }

  async function resolvePbaas() {
    pbaasError = '';
    pbaasCandidates = [];
    pbaasResolvedCoin = null;
    actionSuccess = '';

    if (!pbaasQuery.trim()) {
      pbaasError = i18n.t('wallet.addAsset.error.pbaasInputRequired');
      return;
    }

    pbaasResolving = true;
    try {
      const result = await coinsService.resolvePbaasCurrency(pbaasQuery.trim());
      if (result.status === 'ambiguous') {
        pbaasCandidates = result.candidates;
        pbaasError = i18n.t('wallet.addAsset.error.pbaasAmbiguous');
        return;
      }

      pbaasResolvedCoin = result.coin;
    } catch (error) {
      pbaasError = translateAssetError(error, 'wallet.addAsset.error.pbaasResolveFailed');
    } finally {
      pbaasResolving = false;
    }
  }

  async function addResolvedPbaas() {
    if (!pbaasResolvedCoin) return;
    pbaasAdding = true;
    pbaasError = '';

    try {
      const hydratedCoin = applyCatalogMetadataToCoinDefinition(pbaasResolvedCoin);
      await addCoinToRegistry(hydratedCoin, 'wallet.addAsset.toast.added');
      pbaasQuery = '';
      pbaasResolvedCoin = null;
      pbaasCandidates = [];
    } catch (error) {
      pbaasError = translateAssetError(error, 'wallet.addAsset.error.addFailed');
    } finally {
      pbaasAdding = false;
    }
  }

  async function resolveErc20() {
    erc20Error = '';
    erc20ResolvedCoin = null;
    actionSuccess = '';

    if (!erc20Contract.trim()) {
      erc20Error = i18n.t('wallet.addAsset.error.contractRequired');
      return;
    }

    erc20Resolving = true;
    try {
      const result = await coinsService.resolveErc20Contract(erc20Contract.trim());
      erc20ResolvedCoin = result.coin;
    } catch (error) {
      erc20Error = translateAssetError(error, 'wallet.addAsset.error.erc20ResolveFailed');
    } finally {
      erc20Resolving = false;
    }
  }

  async function addResolvedErc20() {
    if (!erc20ResolvedCoin) return;

    erc20Adding = true;
    erc20Error = '';

    try {
      const hydratedCoin = applyCatalogMetadataToCoinDefinition(erc20ResolvedCoin);
      await addCoinToRegistry(hydratedCoin, 'wallet.addAsset.toast.added');
      erc20Contract = '';
      erc20ResolvedCoin = null;
    } catch (error) {
      erc20Error = translateAssetError(error, 'wallet.addAsset.error.addFailed');
    } finally {
      erc20Adding = false;
    }
  }
</script>

<StandardRightSheet
  bind:isOpen
  title={i18n.t('wallet.addAsset.title')}
  onOpenAutoFocus={handleOpenAutoFocus}
>
  <div class="flex h-full min-h-0 flex-col">
    <div class="min-h-0 flex-1 overflow-y-auto pr-1">
      <div class="sticky top-0 z-10 bg-background pb-3">
        <div class="relative">
          <SearchIcon class="text-muted-foreground absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2" />
          <Input
            type="text"
            bind:value={searchInput}
            placeholder={i18n.t('wallet.addAsset.searchPlaceholder')}
            class="h-10 pl-9 focus-visible:ring-[2px] focus-visible:ring-ring/40"
          />
        </div>

        {#if actionError}
          <div class="mt-2 flex items-start gap-2 rounded-md border border-destructive/35 bg-destructive/10 px-2.5 py-2 text-xs text-destructive">
            <AlertCircleIcon class="mt-0.5 h-4 w-4 shrink-0" />
            <p>{actionError}</p>
          </div>
        {/if}

        {#if actionSuccess}
          <div class="mt-2 rounded-md border border-emerald-500/25 bg-emerald-500/10 px-2.5 py-2 text-xs text-emerald-700 dark:text-emerald-300">
            {actionSuccess}
          </div>
        {/if}
      </div>

      <section class="mt-1 space-y-2">
        <h3 class="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {i18n.t('wallet.addAsset.sectionAdded')}
        </h3>
        {#if catalogView.addedEntries.length === 0}
          <p class="rounded-md border border-border/60 px-3 py-2 text-xs text-muted-foreground">
            {i18n.t('wallet.addAsset.emptySearch')}
          </p>
        {:else}
          <ul class="space-y-1.5">
            {#each catalogView.addedEntries as entry (entry.key)}
              <AddAssetRow
                {entry}
                busy={activeRowKey === entry.key}
                onAction={handleCatalogAction}
              />
            {/each}
          </ul>
        {/if}
      </section>

      <section class="mt-4 space-y-2">
        <h3 class="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {i18n.t('wallet.addAsset.sectionAvailable')}
        </h3>
        {#if catalogView.availableEntries.length === 0}
          <p class="rounded-md border border-border/60 px-3 py-2 text-xs text-muted-foreground">
            {i18n.t('wallet.addAsset.emptySearch')}
          </p>
        {:else}
          <ul class="space-y-1.5">
            {#each catalogView.availableEntries as entry (entry.key)}
              <AddAssetRow
                {entry}
                busy={activeRowKey === entry.key}
                onAction={handleCatalogAction}
              />
            {/each}
          </ul>
        {/if}
      </section>

      <section class="mt-5 space-y-4 border-t border-border/70 pt-4">
        <div>
          <h3 class="text-sm font-semibold text-foreground">{i18n.t('wallet.addAsset.cantFindTitle')}</h3>
          <p class="mt-1 text-xs text-muted-foreground">{i18n.t('wallet.addAsset.cantFindDescription')}</p>
        </div>

        <div class="space-y-2 rounded-md border border-border/70 p-3">
          <p class="text-xs font-semibold text-foreground">{i18n.t('wallet.addAsset.pbaasTitle')}</p>
          <Input
            type="text"
            bind:value={pbaasQuery}
            placeholder={i18n.t('wallet.addAsset.pbaasPlaceholder')}
            class="h-10"
          />
          <div class="flex gap-2">
            <Button
              variant="secondary"
              class="h-8"
              onclick={resolvePbaas}
              disabled={pbaasResolving || pbaasAdding}
            >
              {pbaasResolving ? i18n.t('wallet.addAsset.resolving') : i18n.t('wallet.addAsset.resolve')}
            </Button>

            {#if pbaasResolvedCoin}
              <Button class="h-8" onclick={addResolvedPbaas} disabled={pbaasAdding || pbaasResolving}>
                {pbaasAdding ? i18n.t('wallet.addAsset.adding') : i18n.t('wallet.addAsset.add')}
              </Button>
            {/if}
          </div>

          {#if pbaasResolvedCoin}
            <div class="flex items-center gap-2 rounded-md border border-border/60 bg-muted/15 px-2.5 py-2">
              <CoinIcon coinId={pbaasResolvedCoin.id} coinName={pbaasResolvedCoin.displayName} size={20} decorative />
              <p class="text-xs text-foreground">
                {pbaasResolvedCoin.displayTicker} - {pbaasResolvedCoin.displayName}
              </p>
            </div>
          {/if}

          {#if pbaasCandidates.length > 1}
            <div class="space-y-1 rounded-md border border-border/60 px-2.5 py-2">
              <p class="text-[11px] text-muted-foreground">{i18n.t('wallet.addAsset.pbaasMatches')}</p>
              {#each pbaasCandidates as candidate}
                <button
                  type="button"
                  class="text-left text-xs text-foreground underline-offset-2 hover:underline"
                  onclick={() => {
                    pbaasQuery = candidate.currencyId;
                    resolvePbaas();
                  }}
                >
                  {candidate.displayTicker} ({candidate.currencyId})
                </button>
              {/each}
            </div>
          {/if}

          {#if pbaasError}
            <p class="text-xs text-destructive">{pbaasError}</p>
          {/if}
        </div>

        <div class="space-y-2 rounded-md border border-border/70 p-3">
          <p class="text-xs font-semibold text-foreground">{i18n.t('wallet.addAsset.erc20Title')}</p>
          <Input
            type="text"
            bind:value={erc20Contract}
            placeholder={i18n.t('wallet.addAsset.erc20Placeholder')}
            class="h-10"
          />
          <div class="flex gap-2">
            <Button
              variant="secondary"
              class="h-8"
              onclick={resolveErc20}
              disabled={erc20Resolving || erc20Adding}
            >
              {erc20Resolving ? i18n.t('wallet.addAsset.resolving') : i18n.t('wallet.addAsset.resolve')}
            </Button>

            {#if erc20ResolvedCoin}
              <Button class="h-8" onclick={addResolvedErc20} disabled={erc20Resolving || erc20Adding}>
                {erc20Adding ? i18n.t('wallet.addAsset.adding') : i18n.t('wallet.addAsset.add')}
              </Button>
            {/if}
          </div>

          {#if erc20ResolvedCoin}
            <div class="flex items-center gap-2 rounded-md border border-border/60 bg-muted/15 px-2.5 py-2">
              <CoinIcon coinId={erc20ResolvedCoin.id} coinName={erc20ResolvedCoin.displayName} size={20} decorative />
              <p class="text-xs text-foreground">
                {erc20ResolvedCoin.displayTicker} - {erc20ResolvedCoin.displayName}
              </p>
            </div>
          {/if}

          {#if erc20Error}
            <p class="text-xs text-destructive">{erc20Error}</p>
          {/if}
        </div>
      </section>
    </div>
  </div>
</StandardRightSheet>
