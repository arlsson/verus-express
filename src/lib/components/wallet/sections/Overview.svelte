<!-- 
  Component: Overview
  Purpose: Wallet overview section with balance card, quick actions, and transaction history
  Last Updated: Initial creation
  Security: No sensitive operations - display only (balance is placeholder)
-->

<script lang="ts">
  import * as Card from '$lib/components/ui/card';
  import * as Avatar from '$lib/components/ui/avatar';
  import { Button } from '$lib/components/ui/button';
  import SendIcon from '@lucide/svelte/icons/send';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import ArrowDownUpIcon from '@lucide/svelte/icons/arrow-down-up';

  interface WalletData {
    name: string;
    emoji: string;
    color: string;
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

  function handleSend() {
    // Placeholder for send functionality
    console.log('Send clicked');
  }

  function handleReceive() {
    // Placeholder for receive functionality
    console.log('Receive clicked');
  }
</script>

<div class="flex flex-col gap-6 p-6">
  <!-- Balance Card -->
  <Card.Root>
    <Card.Header>
      <Card.Title class="flex items-center gap-3">
        <Avatar.Root class="h-12 w-12">
          <Avatar.Fallback class={colorClass}>
            <span class="text-2xl">{walletData.emoji}</span>
          </Avatar.Fallback>
        </Avatar.Root>
        <div>
          <div class="text-xl">{walletData.name}</div>
          <div class="text-sm text-muted-foreground font-normal">Main Wallet</div>
        </div>
      </Card.Title>
    </Card.Header>
    <Card.Content>
      <div class="space-y-2">
        <div class="text-4xl font-bold">0.00 VRSC</div>
        <div class="flex gap-4 text-sm text-muted-foreground">
          <span>Available: <span class="text-foreground font-medium">0.00 VRSC</span></span>
          <span>•</span>
          <span>Pending: <span class="text-foreground font-medium">0.00 VRSC</span></span>
        </div>
      </div>
    </Card.Content>
  </Card.Root>

  <!-- Quick Actions -->
  <div class="flex gap-4">
    <Button class="flex-1" size="lg" onclick={handleSend}>
      <SendIcon class="h-4 w-4 mr-2" />
      Send
    </Button>
    <Button variant="outline" class="flex-1" size="lg" onclick={handleReceive}>
      <DownloadIcon class="h-4 w-4 mr-2" />
      Receive
    </Button>
  </div>

  <!-- Recent Transactions Card -->
  <Card.Root>
    <Card.Header>
      <Card.Title>Recent Transactions</Card.Title>
      <Card.Description>Your latest wallet activity</Card.Description>
    </Card.Header>
    <Card.Content>
      <div class="flex flex-col items-center justify-center py-12 text-center">
        <div class="rounded-full bg-muted p-3 mb-4">
          <ArrowDownUpIcon class="h-6 w-6 text-muted-foreground" />
        </div>
        <p class="text-muted-foreground text-sm">No transactions yet</p>
        <p class="text-xs text-muted-foreground mt-1">
          Transactions will appear here once you send or receive VRSC
        </p>
      </div>
    </Card.Content>
  </Card.Root>
</div>
