<!-- 
  Component: WalletLayout
  Purpose: Main layout component that assembles sidebar and content sections
  Last Updated: Initial creation
  Security: No sensitive operations - layout and navigation only
-->

<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import AppSidebar from './AppSidebar.svelte';
  import Overview from './sections/Overview.svelte';
  import Send from './sections/Send.svelte';
  import Receive from './sections/Receive.svelte';
  import Conversions from './sections/Conversions.svelte';
  import Identity from './sections/Identity.svelte';
  import Apps from './sections/Apps.svelte';
  import Activity from './sections/Activity.svelte';
  import AddressBook from './sections/AddressBook.svelte';
  import { dismissWalletError, walletErrorsStore } from '$lib/stores/walletErrors.js';
  import { i18nStore } from '$lib/i18n';

  interface WalletData {
    name: string;
    emoji: string;
    color: string;
    network?: 'mainnet' | 'testnet';
  }

  type SectionId =
    | 'overview'
    | 'send'
    | 'receive'
    | 'conversions'
    | 'identity'
    | 'apps'
    | 'activity'
    | 'address-book';

  const { walletData }: { walletData: WalletData } = $props();
  let activeSection = $state<SectionId>('overview');
  const walletErrors = $derived($walletErrorsStore);
  const latestError = $derived(walletErrors.latest);
  const i18n = $derived($i18nStore);
</script>

<div class="relative h-screen overflow-hidden">
  <div class="absolute top-0 left-0 z-40 h-11 w-[15.25rem]" data-tauri-drag-region aria-hidden="true"></div>
  <Sidebar.Provider class="h-full overflow-hidden">
    <AppSidebar bind:activeSection {walletData} />
    <Sidebar.Inset class="h-full min-h-0 dark:bg-[#111111]">
      <div class="h-6 shrink-0" data-tauri-drag-region aria-hidden="true"></div>
      {#if latestError}
        <div class="mx-6 mt-4 rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900">
          <div class="flex items-start justify-between gap-3">
            <p class="break-all">{latestError}</p>
            <button class="shrink-0 text-xs underline" onclick={dismissWalletError}>
              {i18n.t('wallet.layout.dismiss')}
            </button>
          </div>
        </div>
      {/if}
      <main class={activeSection === 'overview' ? 'flex flex-1 min-h-0 overflow-hidden' : 'flex-1 min-h-0 overflow-auto'}>
        {#if activeSection === 'overview'}
          <Overview
            {walletData}
            onNavigateToSend={() => (activeSection = 'send')}
            onNavigateToReceive={() => (activeSection = 'receive')}
            onNavigateToConvert={() => (activeSection = 'conversions')}
          />
        {:else if activeSection === 'send'}
          <Send onClose={() => (activeSection = 'overview')} />
        {:else if activeSection === 'receive'}
          <Receive />
        {:else if activeSection === 'conversions'}
          <Conversions onClose={() => (activeSection = 'overview')} />
        {:else if activeSection === 'identity'}
          <Identity />
        {:else if activeSection === 'apps'}
          <Apps />
        {:else if activeSection === 'activity'}
          <Activity />
        {:else if activeSection === 'address-book'}
          <AddressBook />
        {/if}
      </main>
    </Sidebar.Inset>
  </Sidebar.Provider>
</div>
