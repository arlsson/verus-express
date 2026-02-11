<!--
  Root Page Route
  Branches on list_wallets + is_unlocked: WelcomeScreen (no wallets), UnlockScreen (locked), or redirect to /wallet.
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import { i18nStore } from '$lib/i18n';
  import WelcomeScreen from '$lib/components/wallet/WelcomeScreen.svelte';
  import UnlockScreen from '$lib/components/wallet/UnlockScreen.svelte';

  type WalletListItem = {
    account_id: string;
    wallet_name: string;
    network?: 'mainnet' | 'testnet';
    emoji?: string;
    color?: string;
  };

  let loading = $state(true);
  let wallets = $state<WalletListItem[]>([]);
  let unlocked = $state(false);
  const i18n = $derived($i18nStore);

  onMount(async () => {
    try {
      const [listResult, unlockedResult] = await Promise.all([
        invoke<WalletListItem[]>('list_wallets').catch(() => []),
        invoke<boolean>('is_unlocked').catch(() => false)
      ]);
      wallets = Array.isArray(listResult) ? listResult : [];
      unlocked = !!unlockedResult;

      if (wallets.length > 0 && unlocked) {
        goto('/wallet');
        return;
      }
    } catch {
      wallets = [];
      unlocked = false;
    } finally {
      loading = false;
    }
  });
</script>

{#if loading || (wallets.length > 0 && unlocked)}
  <main class="bg-background flex min-h-screen flex-col items-center justify-center">
    <div class="bg-[#fbfbfb] dark:bg-[#111111] absolute inset-0"></div>
    <div class="text-muted-foreground relative z-10">{i18n.t('common.loading')}</div>
  </main>
{:else if wallets.length === 0}
  <WelcomeScreen />
{:else}
  <UnlockScreen {wallets} />
{/if}
