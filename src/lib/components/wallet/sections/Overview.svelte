<!--
  Component: Overview
  Purpose: Wallet overview with balance card (from store), quick actions, and transaction history
  Last Updated: Module 9 — balanceStore, transactionStore, coinsStore; navigate to Send/Receive
  Security: No sensitive operations - display only
-->

<script lang="ts">
  import * as Card from '$lib/components/ui/card';
  import * as Avatar from '$lib/components/ui/avatar';
  import { Button } from '$lib/components/ui/button';
  import SendIcon from '@lucide/svelte/icons/send';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import ArrowDownUpIcon from '@lucide/svelte/icons/arrow-down-up';
  import { balanceStore } from '$lib/stores/balances.js';
  import { getTransactions, transactionStore } from '$lib/stores/transactions.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { walletChannelsStore } from '$lib/stores/walletChannels.js';
  import { i18nStore } from '$lib/i18n';
  import { getWalletColorHex } from '$lib/constants/walletColors';

  interface WalletData {
    name: string;
    emoji: string;
    color: string;
    network?: 'mainnet' | 'testnet';
  }

  let {
    walletData,
    onNavigateToSend = () => {},
    onNavigateToReceive = () => {}
  }: {
    walletData: WalletData;
    onNavigateToSend?: () => void;
    onNavigateToReceive?: () => void;
  } = $props();

  const colorHex = $derived(getWalletColorHex(walletData.color));

  const coins = $derived($coinsStore);
  const i18n = $derived($i18nStore);
  const walletChannels = $derived($walletChannelsStore);
  const primaryChannel = $derived(walletChannels.primaryChannelId);
  const hasPrimaryChannel = $derived(Boolean(primaryChannel));
  const balances = $derived($balanceStore);
  const transactions = $derived($transactionStore);
  const balance = $derived(
    primaryChannel ? balances[primaryChannel] : null
  );
  const hasBalanceSnapshot = $derived(
    primaryChannel ? Object.prototype.hasOwnProperty.call(balances, primaryChannel) : false
  );
  const totalBalanceText = $derived(hasBalanceSnapshot ? balance?.total ?? '0.00' : '—');
  const availableBalanceText = $derived(hasBalanceSnapshot ? balance?.confirmed ?? '0.00' : '—');
  const pendingBalanceText = $derived(hasBalanceSnapshot ? balance?.pending ?? '0.00' : '—');
  const primaryCoinId = $derived(
    primaryChannel
      ? Object.entries(walletChannels.byCoinId).find(([, channel]) => channel === primaryChannel)?.[0] ?? null
      : null
  );
  const primaryTicker = $derived(
    primaryCoinId ? coins.find((c) => c.id === primaryCoinId)?.displayTicker ?? 'VRSC' : 'VRSC'
  );
  const recentTxs = $derived(
    primaryChannel ? getTransactions(primaryChannel, transactions) : []
  );
</script>

<div class="flex flex-col gap-6 p-6">
  <!-- Balance Card -->
  <Card.Root>
    <Card.Header>
      <Card.Title class="flex items-center gap-3">
        <Avatar.Root class="h-12 w-12">
          <Avatar.Fallback class="text-white" style={`background-color: ${colorHex};`}>
            <span class="text-2xl">{walletData.emoji}</span>
          </Avatar.Fallback>
        </Avatar.Root>
        <div>
          <div class="text-xl">{walletData.name}</div>
          <div class="text-sm text-muted-foreground font-normal">{i18n.t('wallet.overview.mainWallet')}</div>
        </div>
      </Card.Title>
    </Card.Header>
    <Card.Content>
      {#if hasPrimaryChannel}
        <div class="space-y-2">
          <div class="text-4xl font-bold">
            {totalBalanceText} {primaryTicker}
          </div>
          <div class="flex gap-4 text-sm text-muted-foreground">
            <span
              >{i18n.t('wallet.overview.available')} <span class="text-foreground font-medium"
                >{availableBalanceText} {primaryTicker}</span
              ></span
            >
            <span>•</span>
            <span
              >{i18n.t('wallet.overview.pending')} <span class="text-foreground font-medium"
                >{pendingBalanceText} {primaryTicker}</span
              ></span
            >
          </div>
        </div>
      {:else}
        <p class="text-sm text-muted-foreground">
          {i18n.t('wallet.overview.noChannelRefresh')}
        </p>
      {/if}
    </Card.Content>
  </Card.Root>

  <!-- Quick Actions -->
  <div class="flex gap-4">
    <Button class="flex-1" size="lg" onclick={onNavigateToSend}>
      <SendIcon class="h-4 w-4 mr-2" />
      {i18n.t('wallet.overview.send')}
    </Button>
    <Button variant="outline" class="flex-1" size="lg" onclick={onNavigateToReceive}>
      <DownloadIcon class="h-4 w-4 mr-2" />
      {i18n.t('wallet.overview.receive')}
    </Button>
  </div>

  <!-- Recent Transactions Card -->
  <Card.Root>
    <Card.Header>
      <Card.Title>{i18n.t('wallet.overview.recentTransactions')}</Card.Title>
      <Card.Description>{i18n.t('wallet.overview.latestActivity')}</Card.Description>
    </Card.Header>
    <Card.Content>
      {#if !hasPrimaryChannel}
        <p class="text-muted-foreground text-sm">
          {i18n.t('wallet.overview.noChannel')}
        </p>
      {:else if recentTxs.length === 0}
        <div class="flex flex-col items-center justify-center py-12 text-center">
          <div class="rounded-full bg-muted p-3 mb-4">
            <ArrowDownUpIcon class="h-6 w-6 text-muted-foreground" />
          </div>
          <p class="text-muted-foreground text-sm">{i18n.t('wallet.overview.noTransactions')}</p>
          <p class="text-xs text-muted-foreground mt-1">
            {i18n.t('wallet.overview.txHint', { ticker: primaryTicker })}
          </p>
        </div>
      {:else}
        <ul class="divide-y divide-border">
          {#each recentTxs.slice(0, 10) as tx (tx.txid)}
            <li class="py-3 first:pt-0">
              <div class="flex justify-between text-sm">
                <span class="font-mono text-muted-foreground truncate max-w-[12ch]">{tx.txid}</span>
                <span class="text-foreground font-medium">{tx.amount} {primaryTicker}</span>
              </div>
              <div class="text-xs text-muted-foreground">
                {tx.pending
                  ? i18n.t('wallet.overview.pendingStatus')
                  : i18n.t('wallet.overview.confirmations', { count: tx.confirmations })}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </Card.Content>
  </Card.Root>
</div>
