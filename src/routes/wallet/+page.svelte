<!--
  Route: /wallet
  Purpose: Main wallet dashboard; guarded - redirects to / if locked. Sets up event bridge, loads coins, starts update engine polling.
  Last Updated: Module 10 — startup polling owned by backend update engine
  Security: No sensitive operations - display only
-->

<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import WalletLayout from '$lib/components/wallet/WalletLayout.svelte';
  import * as walletService from '$lib/services/walletService.js';
  import * as coinsService from '$lib/services/coinsService.js';
  import { setupWalletEventBridge } from '$lib/services/eventBridge.js';
  import { balanceStore } from '$lib/stores/balances.js';
  import { ratesStore } from '$lib/stores/rates.js';
  import { transactionStore } from '$lib/stores/transactions.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { buildWalletChannels, resetWalletChannels, walletChannelsStore } from '$lib/stores/walletChannels.js';
  import { clearWalletErrors, pushWalletError } from '$lib/stores/walletErrors.js';
  import { isWalletSupportedAsset } from '$lib/coins/supportedAssets.js';
  import { i18nStore } from '$lib/i18n';
  import type { WalletNetwork } from '$lib/types/wallet.js';

  let loading = $state(true);
  let walletData = $state<{ name: string; emoji: string; color: string; network: WalletNetwork } | null>(null);
  let teardownEventBridge: (() => void) | null = null;
  const i18n = $derived($i18nStore);

  onMount(async () => {
    clearWalletErrors();
    balanceStore.set({});
    ratesStore.set({});
    transactionStore.set({});
    try {
      const unlocked = await walletService.isUnlocked();
      if (!unlocked) {
        goto('/');
        return;
      }
      const active = await walletService.getActiveWallet();
      const walletNetwork: WalletNetwork = active?.network ?? 'mainnet';
      walletData = active
        ? {
            name: active.wallet_name,
            emoji: active.emoji || '💰',
            color: active.color || 'blue',
            network: walletNetwork
          }
        : { name: i18n.t('wallet.overview.mainWallet'), emoji: '💰', color: 'blue', network: walletNetwork };
      const addresses = await walletService.getAddresses().catch(() => null);
      if (!addresses) {
        pushWalletError(i18n.t('wallet.receive.errorLoad'));
      }

      const allCoins = await coinsService.getCoinRegistry();
      const coins = allCoins.filter((coin) => isWalletSupportedAsset(coin, walletNetwork));
      coinsStore.set(coins);

      const channels = buildWalletChannels(coins, addresses?.vrsc_address ?? null);
      walletChannelsStore.set(channels);
      teardownEventBridge = await setupWalletEventBridge();
      await walletService.startUpdateEngine(false);
    } catch (error) {
      const message = error instanceof Error ? error.message : i18n.t('common.unknownError');
      pushWalletError(message);
      goto('/');
      return;
    }
    loading = false;
  });

  onDestroy(() => {
    teardownEventBridge?.();
    balanceStore.set({});
    ratesStore.set({});
    transactionStore.set({});
    resetWalletChannels();
  });
</script>

{#if loading}
  <main class="bg-background flex min-h-screen items-center justify-center">
    <div class="text-muted-foreground">{i18n.t('common.loading')}</div>
  </main>
{:else if walletData}
  <WalletLayout walletData={walletData} />
{/if}
