<script lang="ts">
  import { onMount } from 'svelte';
  import XIcon from '@lucide/svelte/icons/x';
  import type { Snippet } from 'svelte';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import { i18nStore } from '$lib/i18n';

  type WalletTransferStepperShellProps = {
    currentStep: number;
    totalSteps: number;
    onClose?: () => void;
    closeDisabled?: boolean;
    dirty?: boolean;
    children?: Snippet;
    footer?: Snippet;
  };

  const defaultCloseHandler = () => {};

  /* eslint-disable prefer-const */
  let {
    currentStep,
    totalSteps,
    onClose = defaultCloseHandler,
    closeDisabled = false,
    dirty = false,
    children,
    footer
  }: WalletTransferStepperShellProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);

  let showDiscardDialog = $state(false);

  function requestClose() {
    if (closeDisabled) return;
    if (dirty) {
      showDiscardDialog = true;
      return;
    }
    onClose();
  }

  function confirmDiscard() {
    showDiscardDialog = false;
    onClose();
  }

  onMount(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      if (showDiscardDialog) {
        showDiscardDialog = false;
        return;
      }
      requestClose();
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  });
</script>

<section class="mx-auto flex h-full min-h-0 w-full max-w-4xl flex-col px-6 pb-6 pt-4 sm:px-8">
  <div class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border border-border/70 bg-background shadow-sm">
    <header class="relative flex h-[50px] shrink-0 items-center justify-center border-b border-border/80 px-6">
      <div class="flex items-center gap-4">
        <span class="text-muted-foreground text-sm font-medium">
          {i18n.t('shared.stepOf', { current: currentStep, total: totalSteps })}
        </span>
        <div class="flex items-center gap-2">
          {#each [...Array(totalSteps).keys()] as stepIndex}
            {@const stepNumber = stepIndex + 1}
            <div
              class="h-2 w-2 rounded-full transition-all duration-200
                {stepNumber === currentStep
                  ? 'bg-primary scale-125'
                  : stepNumber < currentStep
                    ? 'bg-primary/60'
                    : 'bg-muted-foreground/30'}"
            ></div>
          {/each}
        </div>
      </div>
      <button
        type="button"
        class="ring-offset-background focus-visible:ring-ring absolute right-3 inline-flex h-8 w-8 items-center justify-center rounded-md opacity-70 transition-opacity hover:opacity-100 focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-hidden disabled:pointer-events-none"
        onclick={requestClose}
        disabled={closeDisabled}
        aria-label={i18n.t('common.cancel')}
      >
        <XIcon class="size-5" />
      </button>
    </header>

    <div class="min-h-0 flex-1 overflow-y-auto p-6 sm:p-8">
      {@render children?.()}
    </div>

    <footer class="shrink-0 border-t border-border/80 bg-muted/10 p-4 sm:px-6">
      {@render footer?.()}
    </footer>
  </div>
</section>

<Dialog.Root
  open={showDiscardDialog}
  onOpenChange={(open) => {
    if (!open) showDiscardDialog = false;
  }}
>
  <Dialog.Content class="max-w-md">
    <Dialog.Header>
      <Dialog.Title>{i18n.t('wallet.transfer.closeDiscardTitle')}</Dialog.Title>
      <Dialog.Description>{i18n.t('wallet.transfer.closeDiscardDescription')}</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="flex justify-end gap-3">
      <Button variant="secondary" onclick={() => (showDiscardDialog = false)}>
        {i18n.t('common.cancel')}
      </Button>
      <Button variant="destructive" onclick={confirmDiscard}>
        {i18n.t('wallet.transfer.closeDiscardConfirm')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
