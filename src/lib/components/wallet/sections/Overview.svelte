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
  import { balanceStore } from '$lib/stores/balances.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { ratesStore } from '$lib/stores/rates.js';
  import { walletChannelsStore } from '$lib/stores/walletChannels.js';
  import { i18nStore } from '$lib/i18n';
  import { buildWalletOverviewViewModel } from '$lib/utils/walletOverview.js';
  import { getWalletOverviewDemoSnapshot } from '$lib/utils/walletOverviewDemo.js';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';

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
  const walletChannels = $derived($walletChannelsStore);
  const balances = $derived($balanceStore);
  const rates = $derived($ratesStore);

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

<div class="mx-auto w-full max-w-6xl px-6 pb-6 pt-0 sm:px-8">
  <section class="border-border border-b pb-5 pt-4 sm:pt-5">
    <div class="flex flex-col gap-5 md:flex-row md:items-start md:justify-between md:gap-24">
      <div class="min-w-0">
        <div class="flex items-start">
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

      <div class="w-full max-w-[172px] text-xs md:self-start">
        <div class="flex items-center justify-between py-1.5">
          <span class="text-muted-foreground/90">{i18n.t('wallet.overview.identitiesLabel')}</span>
          <span class="text-foreground min-w-4 text-right text-xs font-medium tabular-nums">
            {overview.identityCount}
          </span>
        </div>
        <div class="bg-border/20 h-px"></div>
        <div class="flex items-center justify-between py-1.5">
          <span class="text-muted-foreground/90">{i18n.t('wallet.overview.assetsLabel')}</span>
          <span class="text-foreground min-w-4 text-right text-xs font-medium tabular-nums">
            {overview.assetCount}
          </span>
        </div>
      </div>
    </div>

    <div class="mt-3 w-full md:max-w-[620px]">
      <div class="grid w-full grid-cols-3 gap-2">
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
    </div>
  </section>

  <section class="pt-3">
    {#if overview.rows.length === 0}
      <p class="text-muted-foreground py-8 text-sm">{i18n.t('wallet.overview.noChannel')}</p>
    {:else}
      <ul class="divide-border divide-y">
        {#each overview.rows as row (row.key)}
          <li class="flex items-center justify-between gap-4 py-4 first:pt-3">
            <div class="min-w-0 flex items-center gap-3">
              <CoinIcon coinId={row.coinId} coinName={row.name} proto={row.proto} size={30} showBadge decorative />
              <div class="min-w-0">
                <p class="text-foreground truncate text-[15px] leading-tight font-medium">{row.name}</p>
                <p class="text-muted-foreground mt-1 text-xs">{row.ticker}</p>
              </div>
            </div>
            <div class="shrink-0 text-right">
              <p class="text-foreground text-[15px] font-semibold">{row.fiatValueDisplay}</p>
              <p class="text-muted-foreground mt-1 text-sm">{row.cryptoAmountDisplay}</p>
              {#if row.unitRateDisplay}
                <p class="text-muted-foreground mt-1 text-[11px]">{row.unitRateDisplay}</p>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>
