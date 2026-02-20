<!--
  Component: Settings
  Purpose: Wallet settings surface with private Verus setup controls.
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import { Button } from '$lib/components/ui/button';
  import { i18nStore } from '$lib/i18n';
  import * as walletService from '$lib/services/walletService';
  import type { WalletNetwork } from '$lib/types/wallet';

  type SettingsProps = {
    walletNetwork: WalletNetwork;
  };

  const { walletNetwork }: SettingsProps = $props();
  const i18n = $derived($i18nStore);

  let loadingStatus = $state(true);
  let configured = $state(false);
  let shieldedAddress = $state('');
  let errorMessage = $state('');
  let successMessage = $state('');
  let generatedSeedPhrase = $state('');
  let importText = $state('');
  let submittingMode = $state<'reuse_primary' | 'create_new' | 'import_text' | null>(null);

  async function loadStatus(): Promise<void> {
    loadingStatus = true;
    errorMessage = '';
    try {
      const status = await walletService.getDlightSeedStatus();
      configured = status.configured;
      shieldedAddress = status.shieldedAddress ?? '';
    } catch (error) {
      console.error('[SETTINGS] Failed to load dlight seed status', error);
      errorMessage = i18n.t('wallet.settings.privateVerus.statusLoadError');
      configured = false;
      shieldedAddress = '';
    } finally {
      loadingStatus = false;
    }
  }

  async function setup(mode: 'reuse_primary' | 'create_new' | 'import_text'): Promise<void> {
    if (submittingMode) return;

    successMessage = '';
    errorMessage = '';
    generatedSeedPhrase = '';
    submittingMode = mode;

    try {
      const result = await walletService.setupDlightSeed({
        mode,
        importText: mode === 'import_text' ? importText : undefined
      });
      configured = result.configured;
      if (result.generatedSeedPhrase) {
        generatedSeedPhrase = result.generatedSeedPhrase;
      }
      successMessage = result.requiresRelogin
        ? i18n.t('wallet.settings.privateVerus.setupSuccessRelogin')
        : i18n.t('wallet.settings.privateVerus.setupSuccess');
      await loadStatus();
    } catch (error) {
      console.error('[SETTINGS] Failed to setup dlight seed', error);
      errorMessage = i18n.t('wallet.settings.privateVerus.setupError');
    } finally {
      submittingMode = null;
    }
  }

  onMount(() => {
    void loadStatus();
  });

  function truncateAddress(value: string): string {
    if (!value) return '';
    if (value.length <= 22) return value;
    return `${value.slice(0, 11)}...${value.slice(-9)}`;
  }

  const privateLabel = $derived(
    walletNetwork === 'testnet'
      ? i18n.t('wallet.private.label.testnet')
      : i18n.t('wallet.private.label.mainnet')
  );
</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col px-6 pb-6 pt-0 sm:px-8">
  <section class="flex min-h-0 flex-1 flex-col overflow-auto pt-3">
    <div class="space-y-5">
      <div class="space-y-1">
        <h2 class="text-xl font-semibold">{i18n.t('wallet.settings.privateVerus.title')}</h2>
        <p class="text-muted-foreground text-sm">
          {i18n.t('wallet.settings.privateVerus.description', { label: privateLabel })}
        </p>
      </div>

      {#if loadingStatus}
        <p class="text-muted-foreground text-sm">{i18n.t('common.loading')}</p>
      {:else}
        <div class="rounded-lg border border-border/70 bg-muted/20 p-4">
          <p class="text-sm font-medium">
            {configured
              ? i18n.t('wallet.settings.privateVerus.statusConfigured')
              : i18n.t('wallet.settings.privateVerus.statusNotConfigured')}
          </p>
          {#if configured && shieldedAddress}
            <p class="text-muted-foreground mt-1 text-xs">
              {i18n.t('wallet.settings.privateVerus.statusAddress', {
                address: truncateAddress(shieldedAddress)
              })}
            </p>
          {/if}
        </div>
      {/if}

      {#if errorMessage}
        <div class="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {errorMessage}
        </div>
      {/if}

      {#if successMessage}
        <div class="rounded-md border border-emerald-300/70 bg-emerald-50 px-3 py-2 text-sm text-emerald-800 dark:border-emerald-500/35 dark:bg-emerald-500/12 dark:text-emerald-200">
          {successMessage}
        </div>
      {/if}

      {#if generatedSeedPhrase}
        <div class="rounded-lg border border-border/70 bg-card p-4">
          <p class="text-sm font-medium">{i18n.t('wallet.settings.privateVerus.generatedSeedTitle')}</p>
          <p class="text-muted-foreground mt-2 text-sm leading-relaxed">{generatedSeedPhrase}</p>
        </div>
      {/if}

      <div class="space-y-3">
        <Button
          variant="secondary"
          class="justify-start"
          disabled={loadingStatus || submittingMode !== null}
          onclick={() => {
            void setup('reuse_primary');
          }}
        >
          {submittingMode === 'reuse_primary'
            ? i18n.t('wallet.settings.privateVerus.settingUp')
            : i18n.t('wallet.settings.privateVerus.reusePrimary')}
        </Button>

        <Button
          variant="secondary"
          class="justify-start"
          disabled={loadingStatus || submittingMode !== null}
          onclick={() => {
            void setup('create_new');
          }}
        >
          {submittingMode === 'create_new'
            ? i18n.t('wallet.settings.privateVerus.settingUp')
            : i18n.t('wallet.settings.privateVerus.createNew')}
        </Button>

        <div class="space-y-2 rounded-lg border border-border/70 bg-card p-3">
          <label for="private-seed-import" class="text-sm font-medium">
            {i18n.t('wallet.settings.privateVerus.importLabel')}
          </label>
          <textarea
            id="private-seed-import"
            class="border-input bg-background ring-offset-background focus-visible:ring-ring min-h-[90px] w-full rounded-md border px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-2"
            placeholder={i18n.t('wallet.settings.privateVerus.importPlaceholder')}
            bind:value={importText}
          ></textarea>
          <Button
            variant="secondary"
            class="justify-start"
            disabled={loadingStatus || submittingMode !== null || !importText.trim()}
            onclick={() => {
              void setup('import_text');
            }}
          >
            {submittingMode === 'import_text'
              ? i18n.t('wallet.settings.privateVerus.settingUp')
              : i18n.t('wallet.settings.privateVerus.importAction')}
          </Button>
        </div>
      </div>
    </div>
  </section>
</div>
