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
  import * as addressBookService from '$lib/services/addressBookService.js';
  import { setupWalletEventBridge } from '$lib/services/eventBridge.js';
  import { balanceStore } from '$lib/stores/balances.js';
  import { ratesStore } from '$lib/stores/rates.js';
  import { transactionStore } from '$lib/stores/transactions.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { buildWalletChannels, resetWalletChannels, walletChannelsStore } from '$lib/stores/walletChannels.js';
  import { filterVisibleAssets } from '$lib/stores/assetVisibility.js';
  import { clearWalletErrors, pushWalletError } from '$lib/stores/walletErrors.js';
  import { setAddressBookContacts } from '$lib/stores/addressBook.js';
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
        await goto('/');
        return;
      }

      const active = await walletService.getActiveWallet().catch((error) => {
        console.error('[WALLET_ROUTE] Failed to resolve active wallet', error);
        return null;
      });
      const walletNetwork: WalletNetwork = active?.network ?? 'mainnet';
      walletData = active
        ? {
            name: active.wallet_name,
            emoji: active.emoji || '💰',
            color: active.color || 'blue',
            network: walletNetwork
          }
        : { name: i18n.t('wallet.overview.mainWallet'), emoji: '💰', color: 'blue', network: walletNetwork };

      const addresses = await walletService.getAddresses().catch((error) => {
        console.error('[WALLET_ROUTE] Failed to load wallet addresses', error);
        return null;
      });
      if (!addresses) {
        pushWalletError(i18n.t('wallet.receive.errorLoad'));
      }

      const allCoins = await coinsService.getCoinRegistry().catch((error) => {
        console.error('[WALLET_ROUTE] Failed to load coin registry', error);
        return [];
      });
      const coins = filterVisibleAssets(
        allCoins.filter((coin) => isWalletSupportedAsset(coin, walletNetwork)),
        walletNetwork
      );
      coinsStore.set(coins);

      const channels = buildWalletChannels(coins, addresses?.vrsc_address ?? null);
      walletChannelsStore.set(channels);

      const contacts = await addressBookService.listAddressBookContacts().catch((error) => {
        console.error('[WALLET_ROUTE] Failed to load address book contacts', error);
        return [];
      });
      setAddressBookContacts(contacts);

      teardownEventBridge = await setupWalletEventBridge().catch((error) => {
        console.error('[WALLET_ROUTE] Failed to setup wallet event bridge', error);
        pushWalletError(error instanceof Error ? error.message : i18n.t('common.unknownError'));
        return null;
      });

      await walletService.startUpdateEngine({
        includeTransactions: false,
        priorityCoinIds: coins.map((coin) => coin.id),
        priorityChannelIds: channels.channels
      }).catch((error) => {
        console.error('[WALLET_ROUTE] Failed to start update engine', error);
        pushWalletError(error instanceof Error ? error.message : i18n.t('common.unknownError'));
      });
    } catch (error) {
      console.error('[WALLET_ROUTE] Startup failed', error);
      const message = error instanceof Error ? error.message : i18n.t('common.unknownError');
      pushWalletError(message);
      if (!walletData) {
        walletData = {
          name: i18n.t('wallet.overview.mainWallet'),
          emoji: '💰',
          color: 'blue',
          network: 'mainnet'
        };
      }
    } finally {
      loading = false;
    }
  });

  onDestroy(() => {
    teardownEventBridge?.();
    balanceStore.set({});
    ratesStore.set({});
    transactionStore.set({});
    resetWalletChannels();
    setAddressBookContacts([]);
  });
</script>

{#if loading}
  <main class="bg-background flex min-h-screen items-center justify-center">
    <div class="text-muted-foreground">{i18n.t('common.loading')}</div>
  </main>
{:else if walletData}
  <WalletLayout walletData={walletData} />
{/if}
