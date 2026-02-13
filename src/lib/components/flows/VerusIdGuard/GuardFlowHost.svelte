<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Button } from '$lib/components/ui/button';
  import StepperLayout from '$lib/components/shared/StepperLayout.svelte';
  import { i18nStore } from '$lib/i18n';
  import {
    beginGuardSession,
    endGuardSession,
    preflightGuardIdentityUpdate,
    sendGuardIdentityUpdate
  } from '$lib/services/guardService';
  import type {
    GuardPreflightResult,
    GuardSendResult,
    IdentityPatch,
    WalletNetwork
  } from '$lib/types/wallet.js';
  import GuardRecoverPatchStep from './GuardRecoverPatchStep.svelte';
  import GuardResultStep from './GuardResultStep.svelte';
  import GuardReviewStep from './GuardReviewStep.svelte';
  import GuardSecretStep from './GuardSecretStep.svelte';
  import GuardTargetStep from './GuardTargetStep.svelte';
  import type {
    GuardFlowErrorCode,
    GuardFlowMode,
    GuardFlowStep,
    GuardRecoverDraft,
    GuardReviewContext
  } from './types';

  const MAINNET_SYSTEM_ID = 'i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV';
  const TESTNET_SYSTEM_ID = 'iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq';

  type GuardFlowHostProps = {
    mode: GuardFlowMode;
    defaultNetwork?: WalletNetwork;
    onClose?: () => void;
  };

  const defaultHandler = () => {};

  /* eslint-disable prefer-const */
  let { mode, defaultNetwork = 'mainnet', onClose = defaultHandler }: GuardFlowHostProps = $props();
  /* eslint-enable prefer-const */

  let step = $state<GuardFlowStep>('secret');
  let importText = $state('');
  let network = $state<WalletNetwork>('mainnet');
  let guardSessionId = $state<string | null>(null);
  let authorityAddress = $state('');
  let targetIdentity = $state('');
  let preflight = $state<GuardPreflightResult | null>(null);
  let sendResult = $state<GuardSendResult | null>(null);
  let errorMessage = $state('');
  let busy = $state(false);
  let copyFeedback = $state('');
  let recoverDraft = $state<GuardRecoverDraft>({
    primaryAddress: '',
    recoveryAuthority: '',
    revocationAuthority: '',
    privateAddress: ''
  });

  let copyFeedbackTimer: ReturnType<typeof setTimeout> | null = null;

  const i18n = $derived($i18nStore);

  const steps = $derived(
    mode === 'recover'
      ? (['secret', 'target', 'patch', 'review', 'result'] as GuardFlowStep[])
      : (['secret', 'target', 'review', 'result'] as GuardFlowStep[])
  );
  const currentStepIndex = $derived(Math.max(steps.indexOf(step), 0) + 1);
  const totalSteps = $derived(steps.length);
  const hasResultSuccess = $derived(!!sendResult && !errorMessage);
  const canContinueTarget = $derived(
    !!targetIdentity.trim() && (mode === 'revoke' || !!recoverDraft.primaryAddress.trim())
  );

  const reviewContext = $derived<GuardReviewContext | null>(
    preflight
      ? {
          mode,
          network,
          targetIdentity,
          authorityAddress,
          preflight,
          recoverDraft
        }
      : null
  );

  $effect(() => {
    if (!guardSessionId && step === 'secret') {
      network = defaultNetwork;
    }
  });

  function systemIdForNetwork(selectedNetwork: WalletNetwork): string {
    return selectedNetwork === 'testnet' ? TESTNET_SYSTEM_ID : MAINNET_SYSTEM_ID;
  }

  function coinIdForNetwork(selectedNetwork: WalletNetwork): string {
    return selectedNetwork === 'testnet' ? 'VRSCTEST' : 'VRSC';
  }

  function buildChannelId(selectedNetwork: WalletNetwork, fromAddress: string): string {
    return `vrpc.${fromAddress}.${systemIdForNetwork(selectedNetwork)}`;
  }

  function extractWalletErrorType(error: unknown): GuardFlowErrorCode {
    if (typeof error === 'string') {
      if (error.includes('invalid args')) return 'Unknown';
      return 'Unknown';
    }
    if (!error || typeof error !== 'object') return 'Unknown';
    const obj = error as Record<string, unknown>;

    if (typeof obj.type === 'string') {
      return (obj.type as GuardFlowErrorCode) ?? 'Unknown';
    }

    if (obj.data && typeof obj.data === 'object') {
      const data = obj.data as Record<string, unknown>;
      if (typeof data.type === 'string') {
        return (data.type as GuardFlowErrorCode) ?? 'Unknown';
      }
    }

    return 'Unknown';
  }

  function errorKeyFromCode(code: GuardFlowErrorCode): string {
    switch (code) {
      case 'InvalidImportText':
        return 'guard.error.invalidImportText';
      case 'GuardSessionNotFound':
        return 'guard.error.guardSessionNotFound';
      case 'IdentityNotFound':
        return 'guard.error.identityNotFound';
      case 'IdentityInvalidState':
        return 'guard.error.identityInvalidState';
      case 'IdentityUnsupportedAuthority':
        return 'guard.error.identityUnsupportedAuthority';
      case 'InvalidPreflight':
        return 'guard.error.invalidPreflight';
      case 'InsufficientFunds':
        return 'guard.error.insufficientFunds';
      case 'NetworkError':
        return 'guard.error.network';
      case 'OperationFailed':
        return 'guard.error.operationFailed';
      case 'IdentityBuildFailed':
        return 'guard.error.identityBuildFailed';
      case 'IdentitySignFailed':
        return 'guard.error.identitySignFailed';
      default:
        return 'guard.error.generic';
    }
  }

  function setMappedError(error: unknown) {
    const code = extractWalletErrorType(error);
    errorMessage = i18n.t(errorKeyFromCode(code));
  }

  function clearError() {
    errorMessage = '';
  }

  function clearCopyFeedback() {
    copyFeedback = '';
    if (copyFeedbackTimer) {
      clearTimeout(copyFeedbackTimer);
      copyFeedbackTimer = null;
    }
  }

  function handleImportTextChange(value: string) {
    importText = value;
    clearError();
  }

  function handleNetworkChange(value: WalletNetwork) {
    network = value;
    clearError();
  }

  function handleTargetIdentityChange(value: string) {
    targetIdentity = value;
    clearError();
  }

  function handlePrimaryAddressChange(value: string) {
    recoverDraft = { ...recoverDraft, primaryAddress: value };
    clearError();
  }

  function handleRecoverDraftChange(next: GuardRecoverDraft) {
    recoverDraft = next;
    clearError();
  }

  function validateRecoverPatch(): string | null {
    if (!recoverDraft.primaryAddress.trim()) {
      return i18n.t('guard.error.primaryRequired');
    }

    const advancedValues = [
      recoverDraft.recoveryAuthority.trim(),
      recoverDraft.revocationAuthority.trim(),
      recoverDraft.privateAddress.trim()
    ];

    const hasInvalidAdvancedValue = advancedValues.some(
      (value) => value.length > 0 && (value.length < 4 || /\s/.test(value))
    );

    if (hasInvalidAdvancedValue) {
      return i18n.t('guard.error.patchFieldInvalid');
    }

    return null;
  }

  function buildRecoverPatch(): IdentityPatch {
    const patch: IdentityPatch = {
      primaryAddresses: [recoverDraft.primaryAddress.trim()]
    };

    const recoveryAuthority = recoverDraft.recoveryAuthority.trim();
    const revocationAuthority = recoverDraft.revocationAuthority.trim();
    const privateAddress = recoverDraft.privateAddress.trim();

    if (recoveryAuthority) patch.recoveryAuthority = recoveryAuthority;
    if (revocationAuthority) patch.revocationAuthority = revocationAuthority;
    if (privateAddress) patch.privateAddress = privateAddress;

    return patch;
  }

  async function cleanupGuardSession() {
    if (!guardSessionId) return;

    const sessionId = guardSessionId;
    guardSessionId = null;

    try {
      await endGuardSession({ guardSessionId: sessionId });
    } catch {
      // Best-effort cleanup only.
    }
  }

  async function closeFlow() {
    if (busy) return;
    await cleanupGuardSession();
    onClose();
  }

  async function handleBeginSession() {
    clearError();
    clearCopyFeedback();

    if (!importText.trim()) {
      errorMessage = i18n.t('guard.error.invalidImportText');
      return;
    }

    busy = true;
    try {
      await cleanupGuardSession();

      const result = await beginGuardSession({
        importText: importText.trim(),
        network
      });

      guardSessionId = result.guardSessionId;
      authorityAddress = result.vrscAddress;
      network = result.network;
      step = 'target';
      targetIdentity = '';
      preflight = null;
      sendResult = null;
    } catch (error) {
      setMappedError(error);
    } finally {
      busy = false;
    }
  }

  async function preflightIdentity(operation: 'revoke' | 'recover', patch: IdentityPatch | null) {
    if (!guardSessionId || !authorityAddress.trim()) {
      errorMessage = i18n.t('guard.error.guardSessionNotFound');
      return;
    }

    busy = true;
    clearError();
    clearCopyFeedback();

    try {
      const preflightResult = await preflightGuardIdentityUpdate({
        guardSessionId,
        params: {
          coinId: coinIdForNetwork(network),
          channelId: buildChannelId(network, authorityAddress),
          operation,
          targetIdentity: targetIdentity.trim(),
          patch,
          memo: null
        }
      });

      preflight = preflightResult;
      step = 'review';
    } catch (error) {
      setMappedError(error);
    } finally {
      busy = false;
    }
  }

  async function handleTargetContinue() {
    clearError();

    if (!targetIdentity.trim()) {
      errorMessage = i18n.t('guard.error.targetRequired');
      return;
    }

    if (mode === 'revoke') {
      await preflightIdentity('revoke', null);
      return;
    }

    if (!recoverDraft.primaryAddress.trim()) {
      errorMessage = i18n.t('guard.error.primaryRequired');
      return;
    }

    step = 'patch';
  }

  async function handlePatchContinue() {
    clearError();

    const patchError = validateRecoverPatch();
    if (patchError) {
      errorMessage = patchError;
      return;
    }

    await preflightIdentity('recover', buildRecoverPatch());
  }

  async function handleSubmit() {
    if (!guardSessionId || !preflight) {
      errorMessage = i18n.t('guard.error.invalidPreflight');
      return;
    }

    busy = true;
    clearError();
    clearCopyFeedback();

    try {
      const result = await sendGuardIdentityUpdate({
        guardSessionId,
        preflightId: preflight.preflightId
      });

      sendResult = result;
      step = 'result';
    } catch (error) {
      sendResult = null;
      setMappedError(error);
      step = 'result';
    } finally {
      busy = false;
    }
  }

  function handleTryAgain() {
    clearError();
    clearCopyFeedback();
    sendResult = null;
    preflight = null;
    step = mode === 'recover' ? 'patch' : 'target';
  }

  async function handleCopyTxid() {
    if (!sendResult?.txid) return;

    const clipboard = globalThis.navigator?.clipboard;
    if (!clipboard) {
      copyFeedback = i18n.t('guard.flow.result.copyFailed');
      return;
    }

    try {
      await clipboard.writeText(sendResult.txid);
      copyFeedback = i18n.t('guard.flow.result.copySuccess');
    } catch {
      copyFeedback = i18n.t('guard.flow.result.copyFailed');
    }

    if (copyFeedbackTimer) {
      clearTimeout(copyFeedbackTimer);
    }
    copyFeedbackTimer = setTimeout(() => {
      copyFeedback = '';
      copyFeedbackTimer = null;
    }, 2000);
  }

  function handleBack() {
    clearError();
    clearCopyFeedback();

    switch (step) {
      case 'target':
        step = 'secret';
        break;
      case 'patch':
        step = 'target';
        break;
      case 'review':
        step = mode === 'recover' ? 'patch' : 'target';
        break;
      case 'result':
        step = mode === 'recover' ? 'patch' : 'target';
        break;
      default:
        break;
    }
  }

  function handleWindowKeyDown(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;

    if (step === 'secret') {
      void closeFlow();
      return;
    }

    handleBack();
  }

  function handleHeaderClose() {
    void closeFlow();
  }

  onDestroy(() => {
    clearCopyFeedback();
    void cleanupGuardSession();
  });
</script>

<svelte:window onkeydown={handleWindowKeyDown} />

<StepperLayout
  currentStep={currentStepIndex}
  totalSteps={totalSteps}
  onClose={handleHeaderClose}
  closeDisabled={busy}
  showNetworkToggle={step === 'secret'}
  network={network}
  networkLabel={i18n.t('guard.flow.secret.networkLabel')}
  networkToggleDisabled={busy}
  onNetworkChange={handleNetworkChange}
>
  {#snippet children()}
    {#if step === 'secret'}
      <GuardSecretStep {mode} {importText} onImportTextChange={handleImportTextChange} />
    {:else if step === 'target'}
      <GuardTargetStep
        {mode}
        {busy}
        {targetIdentity}
        primaryAddress={recoverDraft.primaryAddress}
        onTargetIdentityChange={handleTargetIdentityChange}
        onPrimaryAddressChange={handlePrimaryAddressChange}
      />
    {:else if step === 'patch'}
      <GuardRecoverPatchStep draft={recoverDraft} {busy} onDraftChange={handleRecoverDraftChange} />
    {:else if step === 'review' && reviewContext}
      <GuardReviewStep context={reviewContext} />
    {:else}
      <GuardResultStep
        {mode}
        {sendResult}
        {errorMessage}
        {copyFeedback}
        onCopyTxid={handleCopyTxid}
      />
    {/if}
  {/snippet}

  {#snippet footer()}
    {#if step !== 'result' && errorMessage}
      <p class="text-destructive mb-2 text-right text-sm" aria-live="polite">
        {errorMessage}
      </p>
    {/if}

    <div class="flex w-full items-center justify-between gap-4">
      {#if step === 'result'}
        {#if hasResultSuccess}
          <div class="min-w-48 px-6"></div>
        {:else}
          <Button variant="secondary" onclick={handleTryAgain} disabled={busy} class="min-w-48 px-6">
            {i18n.t('guard.flow.result.tryAgain')}
          </Button>
        {/if}
      {:else}
        <Button
          variant="secondary"
          onclick={step === 'secret' ? closeFlow : handleBack}
          disabled={busy}
          class="min-w-48 px-6"
        >
          {step === 'secret' ? i18n.t('common.cancel') : i18n.t('common.back')}
        </Button>
      {/if}

      {#if step === 'secret'}
        <Button onclick={handleBeginSession} disabled={busy || !importText.trim()} class="min-w-48 px-6">
          {busy ? i18n.t('guard.flow.secret.continueBusy') : i18n.t('guard.flow.secret.continue')}
        </Button>
      {:else if step === 'target'}
        <Button onclick={handleTargetContinue} disabled={busy || !canContinueTarget} class="min-w-48 px-6">
          {busy
            ? i18n.t('guard.flow.target.preflightBusy')
            : mode === 'recover'
              ? i18n.t('guard.flow.target.preflightRecover')
              : i18n.t('guard.flow.target.preflightRevoke')}
        </Button>
      {:else if step === 'patch'}
        <Button
          onclick={handlePatchContinue}
          disabled={busy || !recoverDraft.primaryAddress.trim()}
          class="min-w-48 px-6"
        >
          {busy ? i18n.t('guard.flow.patch.continueBusy') : i18n.t('guard.flow.patch.continue')}
        </Button>
      {:else if step === 'review'}
        <Button onclick={handleSubmit} disabled={busy || !preflight} class="min-w-48 px-6">
          {busy
            ? i18n.t('guard.flow.review.submitBusy')
            : mode === 'revoke'
              ? i18n.t('guard.flow.review.submitRevoke')
              : i18n.t('guard.flow.review.submitRecover')}
        </Button>
      {:else}
        <Button onclick={closeFlow} disabled={busy} class="min-w-48 px-6">
          {i18n.t('common.done')}
        </Button>
      {/if}
    </div>
  {/snippet}
</StepperLayout>
