<!--
  UnlockScreen: Login with password for existing wallet(s).
  Shows wallet list and password field; invokes unlock_wallet then navigates to /wallet.
  Hero panel: Verus logo (white) above tagline; light/dark seedling images; tagline left-aligned, theme-aware text.
  Security: No password logging; generic error messages only.
-->

<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as Sheet from '$lib/components/ui/sheet';
  import { i18nStore, networkLocaleKey } from '$lib/i18n';
  import HelpDrawerLink from '$lib/components/common/HelpDrawerLink.svelte';
  import WalletCreation from '$lib/components/flows/WalletCreation/WalletCreation.svelte';
  import WalletImport from '$lib/components/flows/WalletImport/WalletImport.svelte';
  import ImportMethodList from '$lib/components/flows/WalletImport/ImportMethodList.svelte';
  import type { ImportMethod } from '$lib/components/flows/WalletImport/types';

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
  let shakePasswordField = $state(false);
  let showCreateOptionsDrawer = $state(false);
  let showWalletSwitcherDrawer = $state(false);
  let showCreateWallet = $state(false);
  let showWalletImport = $state(false);
  let selectedImportMethod = $state<ImportMethod>('seed24');
  let createDrawerView = $state<'root' | 'importMethods'>('root');
  let shakeResetTimer: ReturnType<typeof setTimeout> | null = null;
  const appWindow = getCurrentWindow();
  const i18n = $derived($i18nStore);

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
    return i18n.t(networkLocaleKey(network));
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
    createDrawerView = 'root';
    showCreateOptionsDrawer = true;
  }

  function handleStartNewWalletFlow() {
    showCreateOptionsDrawer = false;
    createDrawerView = 'root';
    showCreateWallet = true;
  }

  function handleShowImportMethods() {
    createDrawerView = 'importMethods';
  }

  function handleStartImportWalletFlow(method: ImportMethod) {
    selectedImportMethod = method;
    showCreateOptionsDrawer = false;
    createDrawerView = 'root';
    showWalletImport = true;
  }

  const lostAccessHelpContent = $derived({
    sections: [
      {
        text: i18n.t('unlock.lostAccess.intro')
      },
      {
        heading: i18n.t('unlock.lostAccess.howHeading'),
        text: i18n.t('unlock.lostAccess.howText')
      },
      {
        heading: i18n.t('unlock.lostAccess.needHeading'),
        text: i18n.t('unlock.lostAccess.needText')
      }
    ]
  });

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

  async function triggerWrongPasswordShake() {
    shakePasswordField = false;
    await tick();
    shakePasswordField = true;

    if (shakeResetTimer) {
      clearTimeout(shakeResetTimer);
    }
    shakeResetTimer = setTimeout(() => {
      shakePasswordField = false;
    }, 320);
  }

  onDestroy(() => {
    if (shakeResetTimer) {
      clearTimeout(shakeResetTimer);
    }
  });

  $effect(() => {
    if (!showCreateOptionsDrawer) {
      createDrawerView = 'root';
    }
  });

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
        errorMessage = i18n.t('unlock.error.invalidPassword');
        await triggerWrongPasswordShake();
      } else if (errorType === 'OperationFailed') {
        errorMessage = i18n.t('unlock.error.operationFailed');
      } else if (errorType === 'InvalidArgs') {
        errorMessage = i18n.t('unlock.error.invalidArgs');
      } else {
        errorMessage = i18n.t('unlock.error.generic');
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
        <img
          src="/images/verus-logo-white.svg"
          alt="Verus"
          class="h-5 w-auto cursor-default select-none"
        />
        <p class="text-2xl leading-tight font-bold text-white text-balance dark:text-white mt-8 cursor-default select-none">
          {i18n.t('unlock.hero.tagline')}
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
                  {i18n.t('unlock.switch')}
                </button>
              {/if}
            </div>

          </div>
        {/if}

        <div class="space-y-2">
          <Label for="unlock-password" class="sr-only">{i18n.t('unlock.password')}</Label>
          <div class={shakePasswordField ? 'unlock-error-shake' : ''}>
            <Input
              id="unlock-password"
              type="password"
              bind:value={password}
              placeholder={i18n.t('unlock.password')}
              autocomplete="current-password"
              class={errorMessage ? 'border-destructive' : ''}
              onkeydown={(e) => e.key === 'Enter' && handleUnlock()}
            />
          </div>
          <p class="text-destructive min-h-10 text-sm leading-5" aria-live="polite">{errorMessage}</p>
        </div>

        <div class="flex justify-end">
          <HelpDrawerLink
            linkText={i18n.t('unlock.lostAccess.link')}
            title={i18n.t('unlock.lostAccess.title')}
            content={lostAccessHelpContent}
          />
        </div>

        <div class="space-y-2">
          <Button
            class="w-full"
            onclick={handleUnlock}
            disabled={!effectiveAccountId || !password.trim() || isLoading}
          >
            {isLoading ? i18n.t('unlock.button.unlocking') : i18n.t('unlock.button.unlock')}
          </Button>

          <Button
            variant="secondary"
            class="w-full"
            onclick={handleCreateWallet}
          >
            {i18n.t('unlock.button.createWallet')}
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
          <Sheet.Title>{i18n.t('unlock.switcher.title')}</Sheet.Title>
          <Sheet.Description>{i18n.t('unlock.switcher.description')}</Sheet.Description>
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
        {#if createDrawerView === 'root'}
          <Sheet.Header>
            <Sheet.Title>{i18n.t('unlock.create.title')}</Sheet.Title>
            <Sheet.Description>{i18n.t('unlock.create.description')}</Sheet.Description>
          </Sheet.Header>

          <div class="mt-5 space-y-3">
            <button
              type="button"
              class="border-input hover:bg-muted/60 w-full rounded-lg border p-4 text-left transition-colors"
              onclick={handleStartNewWalletFlow}
            >
              <div class="flex items-start gap-3">
                <svg
                  class="h-4 w-4 mt-0.5 shrink-0 text-muted-foreground"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  aria-hidden="true"
                >
                  <circle cx="12" cy="12" r="9"></circle>
                  <path d="M12 8v8"></path>
                  <path d="M8 12h8"></path>
                </svg>
                <div class="min-w-0">
                  <p class="text-sm font-semibold text-foreground">{i18n.t('unlock.create.newTitle')}</p>
                  <p class="text-xs text-muted-foreground mt-1">
                    {i18n.t('unlock.create.newDescription')}
                  </p>
                </div>
              </div>
            </button>

            <button
              type="button"
              class="border-input hover:bg-muted/60 w-full rounded-lg border p-4 text-left transition-colors"
              onclick={handleShowImportMethods}
            >
              <div class="flex items-start gap-3">
                <svg
                  class="h-4 w-4 mt-0.5 shrink-0 text-muted-foreground"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  aria-hidden="true"
                >
                  <path d="M12 3v11"></path>
                  <path d="m8 10 4 4 4-4"></path>
                  <path d="M4 20h16"></path>
                </svg>
                <div class="min-w-0 flex-1">
                  <p class="text-sm font-semibold text-foreground">{i18n.t('unlock.create.importTitle')}</p>
                  <p class="text-xs text-muted-foreground mt-1">
                    {i18n.t('unlock.create.importDescription')}
                  </p>
                </div>
              </div>
            </button>
          </div>
        {:else}
          <ImportMethodList
            onBack={() => {
              createDrawerView = 'root';
            }}
            onSelect={(method) => {
              handleStartImportWalletFlow(method);
            }}
          />
        {/if}
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

{#if showWalletImport}
  <div class="fixed inset-0 z-50">
    <WalletImport
      initialMethod={selectedImportMethod}
      onGoHome={() => {
        showWalletImport = false;
        selectedImportMethod = 'seed24';
      }}
    />
  </div>
{/if}

<style>
  @keyframes unlock-error-shake {
    0%,
    100% {
      transform: translateX(0);
    }
    20% {
      transform: translateX(-5px);
    }
    40% {
      transform: translateX(5px);
    }
    60% {
      transform: translateX(-4px);
    }
    80% {
      transform: translateX(4px);
    }
  }

  .unlock-error-shake {
    animation: unlock-error-shake 320ms cubic-bezier(0.36, 0.07, 0.19, 0.97);
    will-change: transform;
  }

  @media (prefers-reduced-motion: reduce) {
    .unlock-error-shake {
      animation: none;
    }
  }
</style>
