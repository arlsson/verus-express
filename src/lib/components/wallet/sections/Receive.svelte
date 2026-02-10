<!--
  Component: Receive
  Purpose: Show VRSC (and optionally BTC) address with copy. Uses get_addresses.
  Last Updated: Module 9 — walletService.getAddresses, copy button
  Security: Display-only; no sensitive handling beyond showing address
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import * as Card from '$lib/components/ui/card';
  import * as Label from '$lib/components/ui/label';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import CheckIcon from '@lucide/svelte/icons/check';
  import { toast } from 'svelte-sonner';
  import * as walletService from '$lib/services/walletService.js';
  import type { WalletNetwork } from '$lib/types/wallet.js';

  let addresses = $state<{ vrsc_address: string; eth_address: string; btc_address: string } | null>(null);
  let network = $state<WalletNetwork>('mainnet');
  let loading = $state(true);
  let error = $state('');
  let copied = $state<'vrsc' | 'btc' | null>(null);
  const vrscLabel = $derived(network === 'testnet' ? 'VRSCTEST address' : 'VRSC address');
  const btcLabel = $derived(network === 'testnet' ? 'BTCTEST address' : 'BTC address');

  onMount(async () => {
    try {
      addresses = await walletService.getAddresses();
      try {
        const active = await walletService.getActiveWallet();
        network = active?.network ?? 'mainnet';
      } catch {
        // keep default network
      }
    } catch {
      error = 'Could not load addresses';
    } finally {
      loading = false;
    }
  });

  async function copyAddress(addr: string, which: 'vrsc' | 'btc') {
    try {
      await globalThis.navigator.clipboard.writeText(addr);
      copied = which;
      toast.success('Address copied', {
        description: `${which.toUpperCase()} address copied to clipboard`
      });
      setTimeout(() => (copied = null), 2000);
    } catch {
      toast.error('Failed to copy address');
    }
  }
</script>

<div class="flex flex-col gap-6 p-6 max-w-lg mx-auto">
  <Card.Root>
    <Card.Header>
      <Card.Title class="flex items-center gap-2">
        <DownloadIcon class="h-5 w-5" />
        Receive
      </Card.Title>
      <Card.Description>Share your address to receive funds</Card.Description>
    </Card.Header>
    <Card.Content class="space-y-6">
      {#if loading}
        <p class="text-muted-foreground text-sm">Loading addresses…</p>
      {:else if error}
        <p class="text-destructive text-sm">{error}</p>
      {:else if addresses}
        <div>
          <Label.Root for="receive-vrsc" class="mb-2 block">{vrscLabel}</Label.Root>
          <div class="flex gap-2 items-center">
            <Input
              id="receive-vrsc"
              type="text"
              readonly
              value={addresses.vrsc_address}
              class="flex-1 truncate font-mono text-xs sm:text-sm"
            />
            <Button
              variant="outline"
              size="icon"
              onclick={() => copyAddress(addresses!.vrsc_address, 'vrsc')}
              title="Copy"
            >
              {#if copied === 'vrsc'}
                <CheckIcon class="h-4 w-4 text-green-600" />
              {:else}
                <CopyIcon class="h-4 w-4" />
              {/if}
            </Button>
          </div>
        </div>
        <div>
          <Label.Root for="receive-btc" class="mb-2 block">{btcLabel}</Label.Root>
          <div class="flex gap-2 items-center">
            <Input
              id="receive-btc"
              type="text"
              readonly
              value={addresses.btc_address}
              class="flex-1 truncate font-mono text-xs sm:text-sm"
            />
            <Button
              variant="outline"
              size="icon"
              onclick={() => copyAddress(addresses!.btc_address, 'btc')}
              title="Copy"
            >
              {#if copied === 'btc'}
                <CheckIcon class="h-4 w-4 text-green-600" />
              {:else}
                <CopyIcon class="h-4 w-4" />
              {/if}
            </Button>
          </div>
        </div>
      {/if}
    </Card.Content>
  </Card.Root>
</div>
