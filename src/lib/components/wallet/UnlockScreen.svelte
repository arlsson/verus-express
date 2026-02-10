<!--
  UnlockScreen: Login with password for existing wallet(s).
  Shows wallet list and password field; invokes unlock_wallet then navigates to /wallet.
  Hero panel: light image (seedling-sky.png) in light mode, dark image (seedling-sky-dark.png) in dark mode; tagline vertically centered (slightly below center), left-aligned, theme-aware text color.
  Security: No password logging; generic error messages only.
-->

<script lang="ts">
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as Sheet from '$lib/components/ui/sheet';
  import HelpDrawerLink from '$lib/components/common/HelpDrawerLink.svelte';
  import WalletCreation from '$lib/components/flows/WalletCreation/WalletCreation.svelte';

  export type WalletListItem = {
    account_id: string;
    wallet_name: string;
    network?: 'mainnet' | 'testnet';
    emoji?: string;
    color?: string;
  };

  const { wallets }: { wallets: WalletListItem[] } = $props();

  let selectedAccountId = $state('');
  let password = $state('');
  let errorMessage = $state('');
  let isLoading = $state(false);
  let showCreateOptionsDrawer = $state(false);
  let showWalletSwitcherDrawer = $state(false);
  let showCreateWallet = $state(false);
  const appWindow = getCurrentWindow();

  const effectiveAccountId = $derived(
    wallets.length === 1 ? wallets[0].account_id : selectedAccountId
  );
  const selectedWallet = $derived(
    wallets.length === 0
      ? null
      : wallets.find((wallet) => wallet.account_id === effectiveAccountId) ?? wallets[0]
  );

  $effect(() => {
    if (wallets.length <= 1) {
      selectedAccountId = '';
      return;
    }
    const selectedExists = wallets.some((wallet) => wallet.account_id === selectedAccountId);
    if (!selectedExists) {
      selectedAccountId = wallets[0].account_id;
    }
  });

  function networkLabel(network?: 'mainnet' | 'testnet'): string {
    return network === 'testnet' ? 'Testnet' : 'Mainnet';
  }

  const colorClasses: Record<string, string> = {
    blue: 'bg-blue-500 dark:bg-blue-600',
    indigo: 'bg-indigo-500 dark:bg-indigo-600',
    sky: 'bg-sky-500 dark:bg-sky-600',
    cyan: 'bg-cyan-500 dark:bg-cyan-600',
    green: 'bg-green-500 dark:bg-green-600',
    emerald: 'bg-emerald-500 dark:bg-emerald-600',
    teal: 'bg-teal-500 dark:bg-teal-600',
    lime: 'bg-lime-500 dark:bg-lime-600',
    red: 'bg-red-500 dark:bg-red-600',
    orange: 'bg-orange-500 dark:bg-orange-600',
    amber: 'bg-amber-500 dark:bg-amber-600',
    yellow: 'bg-yellow-500 dark:bg-yellow-600',
    purple: 'bg-purple-500 dark:bg-purple-600',
    violet: 'bg-violet-500 dark:bg-violet-600',
    pink: 'bg-pink-500 dark:bg-pink-600',
    rose: 'bg-rose-500 dark:bg-rose-600',
    slate: 'bg-slate-500 dark:bg-slate-600',
    gray: 'bg-gray-500 dark:bg-gray-600',
    zinc: 'bg-zinc-500 dark:bg-zinc-600',
    stone: 'bg-stone-500 dark:bg-stone-600'
  };

  function walletColorClass(color?: string): string {
    if (!color) return colorClasses.blue;
    return colorClasses[color] ?? colorClasses.blue;
  }

  function walletEmoji(emoji?: string): string {
    return emoji?.trim() || '💰';
  }

  function handleCreateWallet() {
    showCreateOptionsDrawer = true;
  }

  function handleStartNewWalletFlow() {
    showCreateOptionsDrawer = false;
    showCreateWallet = true;
  }

  const lostAccessHelpContent = {
    sections: [
      {
        text: "We can't recover forgotten passwords in a self-custody wallet."
      },
      {
        heading: 'How to regain access',
        text: 'Import your wallet again with your recovery seed phrase, then set a new local password on this device.'
      },
      {
        heading: 'What you need',
        text: 'Use the exact 24 words in the same order. Without the seed phrase, wallet recovery is not possible.'
      }
    ]
  };

  function extractWalletErrorType(error: unknown): string | null {
    if (typeof error === 'string') {
      if (error.includes('invalid args')) return 'InvalidArgs';
      return null;
    }
    if (!error || typeof error !== 'object') return null;
    const obj = error as Record<string, unknown>;
    if (typeof obj.type === 'string') return obj.type;
    if (typeof obj.message === 'string' && obj.message.includes('invalid args')) {
      return 'InvalidArgs';
    }
    if (obj.data && typeof obj.data === 'object') {
      const data = obj.data as Record<string, unknown>;
      if (typeof data.type === 'string') return data.type;
    }
    return null;
  }

  async function handleUnlock() {
    if (!effectiveAccountId || !password.trim()) return;
    isLoading = true;
    errorMessage = '';
    try {
      await invoke('unlock_wallet', {
        account_id: effectiveAccountId,
        password
      });
      password = '';
      goto('/wallet');
    } catch (error) {
      const errorType = extractWalletErrorType(error);
      if (errorType === 'InvalidPassword') {
        errorMessage = 'Wrong password. Please try again.';
      } else if (errorType === 'OperationFailed') {
        errorMessage = "Couldn't unlock wallet on this device. Try again or recreate wallet.";
      } else if (errorType === 'InvalidArgs') {
        errorMessage = 'Unlock request was malformed. Please restart the app and try again.';
      } else {
        errorMessage = 'Unable to unlock wallet right now. Please try again.';
      }
    } finally {
      isLoading = false;
    }
  }

  async function handleDragMouseDown(event: MouseEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    try {
      await appWindow.startDragging();
    } catch {
      // Keep the data-tauri-drag-region fallback.
    }
  }
</script>

<main class="bg-background relative flex min-h-screen overflow-hidden">
  <div class="absolute inset-0 bg-[#fbfbfb] dark:bg-[#111111]"></div>
  <div
    class="absolute top-0 right-0 left-0 z-20 h-11"
    data-tauri-drag-region
    aria-hidden="true"
    onmousedown={handleDragMouseDown}
  ></div>

  <div class="relative z-10 flex min-h-screen w-full">
    <section class="relative hidden w-[clamp(320px,38vw,500px)] shrink-0 overflow-hidden md:block">
      <img
        src="/images/seedling-sky.png"
        alt=""
        aria-hidden="true"
        class="h-full w-full object-cover dark:hidden"
      />
      <img
        src="/images/seedling-sky-dark.png"
        alt=""
        aria-hidden="true"
        class="hidden h-full w-full object-cover dark:block"
      />
      <div class="absolute inset-0 flex flex-col justify-start items-start pl-12 pr-8 pt-20">
        <p class="text-2xl leading-tight font-bold text-white text-balance dark:text-white mt-4 cursor-default select-none">
          Back in control of your digital life.
        </p>
      </div>
    </section>

    <section class="flex min-w-0 flex-1 items-center justify-center px-6 py-10 sm:px-8">
      <div class="w-full max-w-[360px] space-y-5">
        {#if selectedWallet}
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <div class="min-w-0 flex items-center gap-3">
                <div
                  class={"flex h-12 w-12 shrink-0 items-center justify-center rounded-xl text-lg text-white " +
                    walletColorClass(selectedWallet.color)}
                >
                  {walletEmoji(selectedWallet.emoji)}
                </div>
                <div class="min-w-0">
                  <p class="text-foreground truncate text-xl font-semibold leading-tight">{selectedWallet.wallet_name}</p>
                </div>
              </div>

              {#if wallets.length > 1}
                <button
                  type="button"
                  class="text-muted-foreground text-xs underline-offset-4 hover:text-foreground hover:underline"
                  onclick={() => {
                    showWalletSwitcherDrawer = true;
                  }}
                >
                  Switch
                </button>
              {/if}
            </div>

            {#if wallets.length > 1}
              <p class="text-muted-foreground text-xs">{wallets.length} wallets on this device</p>
            {/if}
          </div>
        {/if}

        <div class="space-y-2">
          <Label for="unlock-password" class="sr-only">Password</Label>
          <Input
            id="unlock-password"
            type="password"
            bind:value={password}
            placeholder="Password"
            autocomplete="current-password"
            class={errorMessage ? 'border-destructive' : ''}
            onkeydown={(e) => e.key === 'Enter' && handleUnlock()}
          />
          <p class="text-destructive min-h-10 text-sm leading-5" aria-live="polite">{errorMessage}</p>
        </div>

        <div class="flex justify-end">
          <HelpDrawerLink
            linkText="Lost access?"
            title="Lost access?"
            content={lostAccessHelpContent}
          />
        </div>

        <div class="space-y-2">
          <Button
            class="w-full"
            onclick={handleUnlock}
            disabled={!effectiveAccountId || !password.trim() || isLoading}
          >
            {isLoading ? 'Unlocking…' : 'Unlock'}
          </Button>

          <Button variant="secondary" class="w-full" onclick={handleCreateWallet}>
            Create new wallet
          </Button>
        </div>
      </div>
    </section>
  </div>
</main>

<Sheet.Root bind:open={showWalletSwitcherDrawer}>
  <Sheet.Content side="right" class="w-[360px] max-w-[92vw] p-6">
    {#snippet children()}
      <div class="flex h-full flex-col">
        <Sheet.Header>
          <Sheet.Title>Choose wallet</Sheet.Title>
          <Sheet.Description>Select the wallet you want to unlock.</Sheet.Description>
        </Sheet.Header>

        <div class="mt-5 flex-1 space-y-2 overflow-y-auto pr-1">
          {#each wallets as wallet}
            {@const isSelected = effectiveAccountId === wallet.account_id}
            <Button
              variant={isSelected ? 'secondary' : 'outline'}
              class="h-auto w-full justify-start gap-3 px-3 py-2.5"
              onclick={() => {
                selectedAccountId = wallet.account_id;
                errorMessage = '';
                password = '';
                showWalletSwitcherDrawer = false;
              }}
            >
              <div
                class={"flex h-9 w-9 shrink-0 items-center justify-center rounded-lg text-base text-white " +
                  walletColorClass(wallet.color)}
              >
                {walletEmoji(wallet.emoji)}
              </div>
              <div class="min-w-0 text-left">
                <p class="text-foreground truncate text-sm font-semibold">{wallet.wallet_name}</p>
                <p class="text-muted-foreground text-xs">{networkLabel(wallet.network)}</p>
              </div>
            </Button>
          {/each}
        </div>
      </div>
    {/snippet}
  </Sheet.Content>
</Sheet.Root>

<Sheet.Root bind:open={showCreateOptionsDrawer}>
  <Sheet.Content side="right" class="w-[420px] max-w-[92vw] p-6">
    {#snippet children()}
      <div class="flex h-full flex-col">
        <Sheet.Header>
          <Sheet.Title>Create or import wallet</Sheet.Title>
          <Sheet.Description>Choose how you want to continue.</Sheet.Description>
        </Sheet.Header>

        <div class="mt-5 space-y-3">
          <button
            type="button"
            class="border-input hover:bg-muted/60 w-full rounded-lg border p-4 text-left transition-colors"
            onclick={handleStartNewWalletFlow}
          >
            <p class="text-sm font-semibold text-foreground">Create a brand-new wallet</p>
            <p class="text-xs text-muted-foreground mt-1">
              Generate a new recovery phrase and set up a fresh wallet.
            </p>
          </button>

          <div class="border-input bg-muted/20 w-full rounded-lg border p-4">
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-semibold text-foreground">Import an existing wallet</p>
              <Badge variant="outline">Coming soon</Badge>
            </div>
            <p class="text-xs text-muted-foreground mt-1">
              Use your existing seed phrase to restore a wallet on this device.
            </p>
          </div>
        </div>
      </div>
    {/snippet}
  </Sheet.Content>
</Sheet.Root>

{#if showCreateWallet}
  <div class="fixed inset-0 z-50">
    <WalletCreation
      onGoHome={() => {
        showCreateWallet = false;
      }}
    />
  </div>
{/if}
