<!--
  Component: Overview
  Purpose: Wallet overview with hero balances, quick actions, and currency list
  Last Updated: Wallet overview redesign with dev-only demo fallback
  Security: No sensitive operations - display only
-->

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import SendIcon from '@lucide/svelte/icons/send';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import ArrowLeftRightIcon from '@lucide/svelte/icons/arrow-left-right';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import TrendingDownIcon from '@lucide/svelte/icons/trending-down';
  import TrendingUpIcon from '@lucide/svelte/icons/trending-up';
  import MinusIcon from '@lucide/svelte/icons/minus';
  import { balanceStore } from '$lib/stores/balances.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { ratesStore } from '$lib/stores/rates.js';
  import { walletChannelsStore } from '$lib/stores/walletChannels.js';
  import { i18nStore } from '$lib/i18n';
  import { buildWalletOverviewViewModel } from '$lib/utils/walletOverview.js';
  import { getWalletOverviewDemoSnapshot } from '$lib/utils/walletOverviewDemo.js';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import AddAssetSheet from '$lib/components/wallet/AddAssetSheet.svelte';

  interface WalletData {
    name: string;
    emoji: string;
    color: string;
    network?: 'mainnet' | 'testnet';
  }

  const {
    walletData,
    onNavigateToSend = () => {},
    onNavigateToReceive = () => {},
    onNavigateToConvert = () => {}
  }: {
    walletData: WalletData;
    onNavigateToSend?: () => void;
    onNavigateToReceive?: () => void;
    onNavigateToConvert?: () => void;
  } = $props();

  const coins = $derived($coinsStore);
  const i18n = $derived($i18nStore);
  const walletNetwork = $derived(walletData.network ?? 'mainnet');
  const walletChannels = $derived($walletChannelsStore);
  const balances = $derived($balanceStore);
  const rates = $derived($ratesStore);
  let showAddAssetSheet = $state(false);
  let listScrollElement = $state<HTMLElement | null>(null);
  let hasOverviewScroll = $state(false);
  let canScrollDown = $state(false);
  let hasSeenScrollHint = $state(false);

  function updateScrollAffordance(): void {
    if (!listScrollElement) {
      hasOverviewScroll = false;
      canScrollDown = false;
      return;
    }

    const maxScrollTop = Math.max(0, listScrollElement.scrollHeight - listScrollElement.clientHeight);
    hasOverviewScroll = listScrollElement.scrollTop > 0;
    canScrollDown = maxScrollTop > 1 && listScrollElement.scrollTop < maxScrollTop - 1;
  }

  function onOverviewScroll(event: Event): void {
    const target = event.currentTarget;
    if (!(target instanceof HTMLElement)) return;
    const maxScrollTop = Math.max(0, target.scrollHeight - target.clientHeight);
    hasOverviewScroll = target.scrollTop > 0;
    canScrollDown = maxScrollTop > 1 && target.scrollTop < maxScrollTop - 1;
    if (target.scrollTop > 0) {
      hasSeenScrollHint = true;
    }
  }

  $effect(() => {
    overview.rows.length;
    const element = listScrollElement;
    if (!element) return () => {};

    const resizeObserver = new ResizeObserver(() => {
      updateScrollAffordance();
    });
    resizeObserver.observe(element);
    const content = element.firstElementChild;
    if (content instanceof HTMLElement) {
      resizeObserver.observe(content);
    }
    const frame = window.requestAnimationFrame(() => {
      updateScrollAffordance();
      if (!canScrollDown) {
        hasSeenScrollHint = false;
      }
    });

    return () => {
      window.cancelAnimationFrame(frame);
      resizeObserver.disconnect();
    };
  });

  const liveOverview = $derived(
    buildWalletOverviewViewModel({
      coins,
      walletChannels,
      balances,
      rates,
      intlLocale: i18n.intlLocale,
      network: walletData.network
    })
  );
  const shouldUseDemoData = $derived(import.meta.env.DEV && !liveOverview.hasUsableLiveData);
  const overview = $derived(
    shouldUseDemoData
      ? getWalletOverviewDemoSnapshot(walletData.network, i18n.intlLocale)
      : liveOverview
  );
</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-6xl flex-col px-6 pb-6 pt-0 sm:px-8">
  <section class="flex min-h-0 flex-1 flex-col overflow-hidden">
    <div
      class={`z-10 bg-background pb-4 pt-3 sm:pt-4 ${
        hasOverviewScroll ? 'shadow-[0_10px_22px_-18px_rgba(0,0,0,0.72)]' : ''
      }`}
    >
      <div class="min-w-0">
        <div class="flex -translate-y-2 items-start">
          {#if overview.heroFiatSymbolDisplay}
            <span class="text-muted-foreground mt-1 mr-1.5 text-xl font-semibold sm:text-2xl">
              {overview.heroFiatSymbolDisplay}
            </span>
          {/if}
          <p class="font-google-sans-17pt text-4xl leading-[1.02] font-semibold tracking-tight sm:text-5xl">
            {overview.heroFiatValueDisplay}
          </p>
        </div>
      </div>

      <div class="mt-5 w-full md:max-w-[700px]">
        <div class="flex w-full gap-2">
          <div class="grid w-full flex-1 grid-cols-3 gap-2">
            <Button variant="secondary" size="lg" class="h-10 w-full gap-1.5 rounded-md px-3" onclick={onNavigateToReceive}>
              <DownloadIcon class="h-4 w-4" />
              <span>{i18n.t('wallet.overview.receive')}</span>
            </Button>
            <Button variant="secondary" size="lg" class="h-10 w-full gap-1.5 rounded-md px-3" onclick={onNavigateToSend}>
              <SendIcon class="h-4 w-4" />
              <span>{i18n.t('wallet.overview.send')}</span>
            </Button>
            <Button variant="secondary" size="lg" class="h-10 w-full gap-1.5 rounded-md px-3" onclick={onNavigateToConvert}>
              <ArrowLeftRightIcon class="h-4 w-4" />
              <span>{i18n.t('wallet.overview.convert')}</span>
            </Button>
          </div>
          <Button
            variant="default"
            size="icon"
            class="h-10 w-10 shrink-0 rounded-md"
            aria-label={i18n.t('wallet.addAsset.open')}
            onclick={() => {
              showAddAssetSheet = true;
            }}
          >
            <PlusIcon class="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>

    <div class="relative min-h-0 flex-1">
      <div class="h-full overflow-y-auto overscroll-contain" bind:this={listScrollElement} onscroll={onOverviewScroll}>
        {#if overview.rows.length === 0}
          <p class="text-muted-foreground px-1 py-8 text-sm">{i18n.t('wallet.overview.noChannel')}</p>
        {:else}
          <ul class="space-y-0.5 pb-3">
            {#each overview.rows as row (row.key)}
              <li class="grid grid-cols-[minmax(0,1fr)_11rem_10.25rem] items-center gap-3 rounded-md px-3 py-2.5 transition-colors hover:bg-muted/40">
                <div class="min-w-0 flex items-center gap-3">
                  <CoinIcon coinId={row.coinId} coinName={row.name} proto={row.proto} size={28} showBadge decorative />
                  <div class="min-w-0 flex min-h-7 items-center">
                    <p class="text-foreground truncate text-[15px] leading-tight font-medium">{row.name}</p>
                  </div>
                </div>

                <div class="pl-1 text-left tabular-nums">
                  <p class="text-foreground text-[14px] font-medium">{row.marketPriceDisplay}</p>
                  <div
                    class={`mt-0.5 inline-flex items-center gap-1 text-xs ${
                      row.change24hDirection === 'up'
                        ? 'text-emerald-700 dark:text-emerald-300'
                        : row.change24hDirection === 'down'
                          ? 'text-destructive'
                          : 'text-muted-foreground'
                    }`}
                  >
                    {#if row.change24hDirection === 'up'}
                      <TrendingUpIcon class="h-3.5 w-3.5" aria-hidden="true" />
                    {:else if row.change24hDirection === 'down'}
                      <TrendingDownIcon class="h-3.5 w-3.5" aria-hidden="true" />
                    {:else if row.change24hDirection === 'flat'}
                      <MinusIcon class="h-3.5 w-3.5" aria-hidden="true" />
                    {/if}
                    <span>{row.change24hDisplay}</span>
                  </div>
                </div>

                <div class="text-right tabular-nums">
                  <p class="text-foreground text-[15px] font-semibold">{row.fiatValueDisplay}</p>
                  <p class="text-muted-foreground mt-0.5 text-xs">{row.cryptoAmountDisplay}</p>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      {#if canScrollDown}
        <div class="pointer-events-none absolute inset-x-0 bottom-0 h-14 bg-gradient-to-t from-background to-transparent"></div>
      {/if}

      {#if canScrollDown && !hasOverviewScroll && !hasSeenScrollHint}
        <div class="pointer-events-none absolute inset-x-0 bottom-3 flex justify-center">
          <div class="scroll-hint text-muted-foreground/85 inline-flex items-center gap-1 text-[11px]">
            <ChevronDownIcon class="scroll-hint-icon h-3.5 w-3.5" aria-hidden="true" />
            <span>{i18n.t('wallet.overview.scrollHintMoreAssets')}</span>
          </div>
        </div>
      {/if}
    </div>
  </section>
</div>

<AddAssetSheet bind:isOpen={showAddAssetSheet} network={walletNetwork} />

<style>
  .scroll-hint {
    animation: scroll-hint-nudge 1.8s ease-in-out infinite;
  }

  .scroll-hint-icon {
    animation: scroll-hint-icon-nudge 1.8s ease-in-out infinite;
  }

  @keyframes scroll-hint-nudge {
    0%,
    100% {
      transform: translateY(0);
      opacity: 0.82;
    }
    50% {
      transform: translateY(2px);
      opacity: 1;
    }
  }

  @keyframes scroll-hint-icon-nudge {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(1px);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .scroll-hint,
    .scroll-hint-icon {
      animation: none;
    }
  }
</style>
