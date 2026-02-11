<!--
  Component: WalletCreation
  Purpose: Wallet creation flow with onboarding, backup, and setup
  Last Updated: Login-style hero panel layout with setup-only steps (intro removed)
  Security: Manages shared state and sensitive data clearing, enforces security acknowledgment
-->

<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import NameStep from './NameStep.svelte';
  import SecurityStep from './SecurityStep.svelte';
  import BackupStep from './BackupStep.svelte';
  import VerifyStep from './VerifyStep.svelte';
  import PasswordStep from './PasswordStep.svelte';
  import CompleteStep from './CompleteStep.svelte';
  import { i18nStore } from '$lib/i18n';

  type WalletData = {
    name: string;
    emoji: string;
    color: string;
    password: string;
    network: 'mainnet' | 'testnet';
  };

  type WalletUpdate = Partial<WalletData>;
  type CreateWalletResult = { wallet_id: string; success: boolean };

  const TOTAL_STEPS = 6;

  // Props
  const { onGoHome = () => {} } = $props();

  // Shared state for all steps
  let currentStep = $state(1);
  let walletData = $state<WalletData>({
    name: '',
    emoji: '💰',
    color: 'blue',
    password: '',
    network: 'mainnet'
  });
  let seedPhrase = $state<string>(''); // Security: Cleared on unmount
  let verificationIndices = $state<number[]>([]);
  let securityAccepted = $state(false);
  let allVerificationFieldsFilled = $state(false);
  let verifyStepRef: any = $state(null);
  let backupCanContinue = $state(false);
  let canCreateWallet = $state(false);
  let createLoading = $state(false);
  let createError = $state('');
  let createdWalletId = $state('');
  let openWalletLoading = $state(false);
  let openWalletError = $state('');
  let openWalletAttempted = $state(false);
  const i18n = $derived($i18nStore);

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

  // Create wallet via backend then advance to CompleteStep
  async function handleCreateWallet() {
    if (!canCreateWallet) return;
    createLoading = true;
    createError = '';
    openWalletError = '';
    try {
      const result = await invoke<CreateWalletResult>('create_wallet', {
        request: {
          wallet_name: walletData.name,
          seed_phrase: seedPhrase,
          network: walletData.network,
          emoji: walletData.emoji,
          color: walletData.color
        },
        password: walletData.password
      });

      if (!result?.wallet_id) {
        throw new Error('Missing wallet_id from create_wallet');
      }

      createdWalletId = result.wallet_id;
      seedPhrase = '';
      openWalletAttempted = false;
      nextStep();
    } catch (error) {
      const errorType = extractWalletErrorType(error);
      if (errorType === 'WalletExists') {
        createError = i18n.t('walletCreation.error.walletExists');
      } else {
        createError = i18n.t('walletCreation.error.createFailed');
      }
    } finally {
      createLoading = false;
    }
  }

  async function handleOpenWallet() {
    if (!createdWalletId || !walletData.password) {
      openWalletError = i18n.t('walletCreation.error.openMissingData');
      return;
    }

    openWalletLoading = true;
    openWalletError = '';

    try {
      await invoke('unlock_wallet', {
        account_id: createdWalletId,
        password: walletData.password
      });
      clearSensitiveData();
      goto('/wallet');
    } catch (error) {
      const errorType = extractWalletErrorType(error);
      if (errorType === 'InvalidPassword' || errorType === 'OperationFailed') {
        openWalletError = i18n.t('walletCreation.error.openFailed');
      } else if (errorType === 'InvalidArgs') {
        openWalletError = i18n.t('walletCreation.error.openInvalidArgs');
      } else {
        openWalletError = i18n.t('walletCreation.error.openGeneric');
      }
    } finally {
      openWalletLoading = false;
    }
  }

  function nextStep() {
    currentStep = Math.min(currentStep + 1, TOTAL_STEPS);
  }

  function handleBack() {
    if (currentStep > 1) {
      currentStep -= 1;
      return;
    }
    handleGoHome();
  }

  // Handle verification and continue
  function handleVerifyAndContinue() {
    if (verifyStepRef && verifyStepRef.verifyWords) {
      const isValid = verifyStepRef.verifyWords();
      if (isValid) {
        nextStep();
      }
      // If invalid, errors will show inline - no need to do anything else
    }
  }

  // Clear sensitive data and go home
  function handleGoHome() {
    console.info('[WALLET_CREATION] Clearing sensitive data before navigation');
    clearSensitiveData();
    console.info('[WALLET_CREATION] Calling parent onGoHome');
    onGoHome();
  }

  function clearSensitiveData() {
    seedPhrase = '';
    walletData = { name: '', emoji: '💰', color: 'blue', password: '', network: 'mainnet' };
    verificationIndices = [];
    securityAccepted = false;
    backupCanContinue = false;
    createdWalletId = '';
    openWalletError = '';
    openWalletLoading = false;
    openWalletAttempted = false;
  }

  $effect(() => {
    if (currentStep !== TOTAL_STEPS) return;
    if (!createdWalletId || !walletData.password) return;
    if (openWalletAttempted || openWalletLoading || openWalletError) return;
    openWalletAttempted = true;
    handleOpenWallet();
  });

  // Security: Clear all sensitive data on component destroy
  $effect(() => {
    return () => {
      clearSensitiveData();
      console.info('[WALLET] Component destroyed, sensitive data cleared');
    };
  });
</script>

<main class="h-screen flex flex-col overflow-hidden">
  <div class="absolute inset-0 bg-[#fbfbfb] dark:bg-[#111111]"></div>
  <div class="absolute top-0 right-0 left-0 z-30 h-11" data-tauri-drag-region aria-hidden="true"></div>

  <div class="relative z-10 flex min-h-0 flex-1 w-full">
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

    <section class="flex min-w-0 flex-1">
      <div class="flex min-h-0 flex-1 flex-col">
        <div class="shrink-0 border-b border-border/80">
          <div class="flex h-[50px] items-center justify-center px-6">
            <div class="flex items-center gap-4">
              <span class="text-sm text-muted-foreground font-medium">
                {i18n.t('shared.stepOf', { current: currentStep, total: TOTAL_STEPS })}
              </span>

              <div class="flex items-center gap-2">
                {#each Array(TOTAL_STEPS) as _, index}
                  {@const stepNum = index + 1}
                  <div
                    class="w-2 h-2 rounded-full transition-all duration-200
                           {stepNum === currentStep
                             ? 'bg-primary scale-125'
                             : stepNum < currentStep
                               ? 'bg-primary/60'
                               : 'bg-muted-foreground/30'}"
                  ></div>
                {/each}
              </div>
            </div>
          </div>
        </div>

        <div class="flex-1 overflow-y-auto px-6 py-10 sm:px-8">
          <div class="mx-auto w-full max-w-[620px] space-y-6">
            {#if currentStep === 1}
              <div class="space-y-3">
                <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
                  {i18n.t('walletCreation.step2.title')}
                </h1>
              </div>
              <NameStep
                walletData={walletData}
                onUpdate={(data: WalletUpdate) => {
                  walletData = { ...walletData, ...data };
                }}
                errorMessage=""
              />
            {:else if currentStep === 2}
              <div class="space-y-3">
                <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
                  {i18n.t('walletCreation.step3.title')}
                </h1>
              </div>
              <SecurityStep bind:securityAccepted />
            {:else if currentStep === 3}
              <div class="space-y-3">
                <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
                  {i18n.t('walletCreation.step4.title')}
                </h1>
                <p class="text-muted-foreground text-sm">{i18n.t('walletCreation.step4.description')}</p>
              </div>
              <BackupStep
                walletData={walletData}
                seedPhrase={seedPhrase}
                bind:canContinue={backupCanContinue}
                onSeedGenerated={(seed: string) => {
                  seedPhrase = seed;
                }}
              />
            {:else if currentStep === 4}
              <div class="space-y-3">
                <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
                  {i18n.t('walletCreation.step5.title')}
                </h1>
                <p class="text-muted-foreground text-sm">{i18n.t('walletCreation.step5.description')}</p>
              </div>
              <VerifyStep
                seedPhrase={seedPhrase}
                verificationIndices={verificationIndices}
                onVerified={nextStep}
                onSetupVerification={(indices: number[]) => {
                  verificationIndices = indices;
                }}
                onFieldsChanged={(filled: boolean) => {
                  allVerificationFieldsFilled = filled;
                }}
                bind:this={verifyStepRef}
              />
            {:else if currentStep === 5}
              <div class="space-y-3">
                <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
                  {i18n.t('walletCreation.step6.title')}
                </h1>
                <p class="text-muted-foreground text-sm">{i18n.t('walletCreation.step6.description')}</p>
              </div>
              <PasswordStep
                walletData={walletData}
                onUpdate={(data: WalletUpdate) => {
                  walletData = { ...walletData, ...data };
                }}
                onCanCreateChanged={(canCreate: boolean) => {
                  canCreateWallet = canCreate;
                }}
              />
            {:else if currentStep === 6}
              <div class="space-y-3">
                <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
                  {i18n.t('walletCreation.step7.title')}
                </h1>
                <p class="text-muted-foreground text-sm">{i18n.t('walletCreation.step7.description')}</p>
              </div>
              <CompleteStep isOpening={openWalletLoading} openError={openWalletError} />
            {/if}
          </div>
        </div>

        <div class="shrink-0 border-t border-black/10 bg-muted/10 dark:border-white/20">
          <div class="mx-auto flex w-full max-w-[620px] items-center justify-between gap-4 px-6 py-4 sm:px-8">
            <Button variant="secondary" onclick={handleBack} class="w-48">
              {i18n.t('common.back')}
            </Button>

            {#if currentStep === 1}
              <Button
                onclick={nextStep}
                disabled={!walletData.name.trim() || /[/\\:*?"<>|]/.test(walletData.name)}
                class="w-48"
              >
                {i18n.t('common.continue')}
              </Button>
            {:else if currentStep === 2}
              <Button onclick={nextStep} disabled={!securityAccepted} class="w-48">
                {i18n.t('walletCreation.step3.button')}
              </Button>
            {:else if currentStep === 3}
              <Button onclick={nextStep} disabled={!backupCanContinue} class="w-48">
                {i18n.t('walletCreation.step4.button')}
              </Button>
            {:else if currentStep === 4}
              <Button
                onclick={handleVerifyAndContinue}
                disabled={!allVerificationFieldsFilled}
                class="w-48"
              >
                {i18n.t('walletCreation.step5.button')}
              </Button>
            {:else if currentStep === 5}
              <div class="flex flex-col items-end gap-2">
                {#if createError}
                  <p class="text-destructive text-sm">{createError}</p>
                {/if}
                {#if createLoading}
                  <p class="text-muted-foreground text-xs">{i18n.t('walletCreation.step6.loadingHint')}</p>
                {/if}
                <Button
                  onclick={handleCreateWallet}
                  disabled={!canCreateWallet || createLoading}
                  class="w-48"
                >
                  {createLoading
                    ? i18n.t('walletCreation.step6.buttonCreating')
                    : i18n.t('walletCreation.step6.buttonCreate')}
                </Button>
              </div>
            {:else if currentStep === 6}
              <div class="flex flex-col items-end gap-2">
                <div class="min-h-5">
                  {#if openWalletError}
                    <p class="text-destructive text-sm">{openWalletError}</p>
                  {/if}
                </div>
                {#if openWalletError}
                  <Button
                    onclick={() => {
                      openWalletError = '';
                      openWalletAttempted = true;
                      handleOpenWallet();
                    }}
                    disabled={openWalletLoading}
                    class="w-48"
                  >
                    {i18n.t('walletCreation.step7.buttonRetry')}
                  </Button>
                {:else}
                  <Button disabled class="w-48">
                    {i18n.t('walletCreation.step7.buttonOpening')}
                  </Button>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      </div>
    </section>
  </div>
</main>
