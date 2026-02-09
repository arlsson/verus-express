<!-- 
  Component: WalletLayout
  Purpose: Main layout component that assembles sidebar, topbar, and content sections
  Last Updated: Initial creation
  Security: No sensitive operations - layout and navigation only
-->

<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import AppSidebar from './AppSidebar.svelte';
  import TopBar from './TopBar.svelte';
  import Overview from './sections/Overview.svelte';
  import Send from './sections/Send.svelte';
  import Receive from './sections/Receive.svelte';
  import Conversions from './sections/Conversions.svelte';
  import Identity from './sections/Identity.svelte';
  import AddressBook from './sections/AddressBook.svelte';

  interface WalletData {
    name: string;
    emoji: string;
    color: string;
  }

  let { walletData }: { walletData: WalletData } = $props();
  let activeSection = $state('overview');
</script>

<Sidebar.Provider>
  <AppSidebar bind:activeSection />
  <Sidebar.Inset>
    <TopBar {walletData} />
    <main class="flex-1 overflow-auto">
      {#if activeSection === 'overview'}
        <Overview {walletData} />
      {:else if activeSection === 'send'}
        <Send />
      {:else if activeSection === 'receive'}
        <Receive />
      {:else if activeSection === 'conversions'}
        <Conversions />
      {:else if activeSection === 'identity'}
        <Identity />
      {:else if activeSection === 'address-book'}
        <AddressBook />
      {/if}
    </main>
  </Sidebar.Inset>
</Sidebar.Provider>
