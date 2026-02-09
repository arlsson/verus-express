<!-- 
  Route: /wallet
  Purpose: Main wallet dashboard page
  Last Updated: Initial creation
  Security: No sensitive operations - placeholder wallet data for now
-->

<script lang="ts">
  import WalletLayout from '$lib/components/wallet/WalletLayout.svelte';

  // TODO: Get actual wallet data from Tauri backend
  const walletData = {
    name: 'My Wallet',
    emoji: '💰',
    color: 'blue'
  };
</script>

<WalletLayout {walletData} />
