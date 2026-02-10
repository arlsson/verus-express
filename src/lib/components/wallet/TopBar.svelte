<!--
  Component: TopBar
  Purpose: Top navigation bar with wallet info, settings, and lock button
  Last Updated: handleLock invokes lock_wallet and navigates to /
  Security: No sensitive operations - display and navigation only
-->

<script lang="ts">
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import * as Avatar from '$lib/components/ui/avatar';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import { Button } from '$lib/components/ui/button';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import LockIcon from '@lucide/svelte/icons/lock';

  interface WalletData {
    name: string;
    emoji: string;
    color: string;
    network?: 'mainnet' | 'testnet';
  }

  let { walletData }: { walletData: WalletData } = $props();

  // Color class lookup (matching CompleteStep pattern)
  const colorOptions = [
    { name: 'blue', class: 'bg-blue-100 dark:bg-blue-900' },
    { name: 'green', class: 'bg-green-100 dark:bg-green-900' },
    { name: 'purple', class: 'bg-purple-100 dark:bg-purple-900' },
    { name: 'orange', class: 'bg-orange-100 dark:bg-orange-900' },
    { name: 'pink', class: 'bg-pink-100 dark:bg-pink-900' },
    { name: 'yellow', class: 'bg-yellow-100 dark:bg-yellow-900' }
  ];

  const colorClass = $derived(
    colorOptions.find(c => c.name === walletData.color)?.class || colorOptions[0].class
  );

  async function handleLock() {
    try {
      await invoke('lock_wallet');
      goto('/');
    } catch {
      console.error('[WALLET] Lock failed');
    }
  }
</script>

<div class="sticky top-0 z-10 flex h-16 items-center justify-between border-b border-border bg-background/95 px-6 backdrop-blur supports-[backdrop-filter]:bg-background/60">
  <!-- Left: Wallet Display -->
  <div class="flex items-center gap-3">
    <Avatar.Root class="h-10 w-10">
      <Avatar.Fallback class={colorClass}>
        <span class="text-xl">{walletData.emoji}</span>
      </Avatar.Fallback>
    </Avatar.Root>
    <span class="font-semibold text-lg">{walletData.name}</span>
  </div>

  <!-- Right: Actions -->
  <div class="flex items-center gap-2">
    <!-- Settings Dropdown -->
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        {#snippet child({ props })}
          <Button {...props} variant="ghost" size="icon">
            <SettingsIcon class="h-5 w-5" />
            <span class="sr-only">Settings</span>
          </Button>
        {/snippet}
      </DropdownMenu.Trigger>
      <DropdownMenu.Content align="end" class="w-48">
        <DropdownMenu.Label>Settings</DropdownMenu.Label>
        <DropdownMenu.Separator />
        <DropdownMenu.Item>Wallet Settings</DropdownMenu.Item>
        <DropdownMenu.Item>Network Settings</DropdownMenu.Item>
        <DropdownMenu.Separator />
        <DropdownMenu.Item>About</DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>

    <!-- Lock Button -->
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <Button {...props} variant="ghost" size="icon" onclick={handleLock}>
            <LockIcon class="h-5 w-5" />
            <span class="sr-only">Lock Wallet</span>
          </Button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content>
        <p>Lock Wallet</p>
      </Tooltip.Content>
    </Tooltip.Root>
  </div>
</div>
