<!--
  Route: /wallet
  Purpose: Main wallet dashboard; guarded - redirects to / if locked. Sets up event bridge, loads coins, starts update engine polling.
  Last Updated: Module 10 — startup polling owned by backend update engine
  Security: No sensitive operations - display only
-->

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import WalletLayout from '$lib/components/wallet/WalletLayout.svelte';
  import * as walletService from '$lib/services/walletService.js';
  import * as coinsService from '$lib/services/coinsService.js';
  import { setupWalletEventBridge } from '$lib/services/eventBridge.js';
  import { balanceStore } from '$lib/stores/balances.js';
  import { transactionStore } from '$lib/stores/transactions.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { walletChannelsStore, buildWalletChannels, resetWalletChannels } from '$lib/stores/walletChannels.js';
  import { clearWalletErrors, pushWalletError } from '$lib/stores/walletErrors.js';
  import type { WalletNetwork } from '$lib/types/wallet.js';

  let loading = $state(true);
  let walletData = $state<{ name: string; emoji: string; color: string; network: WalletNetwork } | null>(null);
  let teardownEventBridge: (() => void) | null = null;

  onMount(async () => {
    clearWalletErrors();
    balanceStore.set({});
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
        : { name: 'Wallet', emoji: '💰', color: 'blue', network: walletNetwork };
      const addresses = await walletService.getAddresses().catch(() => null);
      if (!addresses) {
        pushWalletError('wallet: Could not load receive addresses');
      }

      const allCoins = await coinsService.getCoinRegistry();
      const coins = allCoins.filter((c) => c.isTestnet === (walletNetwork === 'testnet'));
      coinsStore.set(coins);

      const channels = buildWalletChannels(coins, addresses?.vrsc_address ?? null);
      walletChannelsStore.set(channels);
      teardownEventBridge = await setupWalletEventBridge();
      await walletService.startUpdateEngine();
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      pushWalletError(`wallet: ${message}`);
      goto('/');
      return;
    }
    loading = false;
  });

  onDestroy(() => {
    teardownEventBridge?.();
    balanceStore.set({});
    transactionStore.set({});
    resetWalletChannels();
  });
</script>

{#if loading}
  <main class="bg-background flex min-h-screen items-center justify-center">
    <div class="text-muted-foreground">Loading…</div>
  </main>
{:else if walletData}
  <WalletLayout walletData={walletData} />
{/if}
