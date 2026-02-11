<!-- 
  Component: TopBar
  Purpose: Top navigation bar with progress and home button (70px height)
  Last Updated: Created for wallet creation redesign
  Security: No sensitive data - UI navigation only
-->

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Home } from '@lucide/svelte';
  import * as Dialog from '$lib/components/ui/dialog';
  import { i18nStore } from '$lib/i18n';

  // Props
  let {
    currentStep = 1,
    totalSteps = 3,
    onGoHome = () => {},
    requireConfirmation = false,
    confirmationMessage = ''
  } = $props();

  const i18n = $derived($i18nStore);

  // Dialog state
  let showConfirmDialog = $state(false);

  function handleHomeClick() {
    console.info('[TOPBAR] Home button clicked');
    console.info('[TOPBAR] Current requireConfirmation:', requireConfirmation);

    if (requireConfirmation) {
      console.info('[TOPBAR] Opening custom confirmation dialog');
      showConfirmDialog = true;
    } else {
      console.info('[TOPBAR] No confirmation needed, calling onGoHome');
      onGoHome();
    }
  }

  function confirmGoHome() {
    console.info('[TOPBAR] User confirmed via custom dialog');
    showConfirmDialog = false;
    onGoHome();
  }

  function cancelGoHome() {
    console.info('[TOPBAR] User cancelled via custom dialog');
    showConfirmDialog = false;
  }
</script>

<div class="h-[50px] bg-background border-b border-border flex items-center justify-between pl-4 pr-6">
  <!-- Home Button (moved more left) -->
  <Button
    variant="ghost"
    size="sm"
    onclick={handleHomeClick}
    class="text-muted-foreground hover:text-foreground p-2"
  >
    <Home class="h-5 w-5" />
  </Button>

  <!-- Step Indicator (dot style) -->
  <div class="flex-1 flex items-center justify-center gap-4">
    <!-- Step Text -->
    <span class="text-sm text-muted-foreground font-medium">
      {i18n.t('shared.stepOf', { current: currentStep, total: totalSteps })}
    </span>

    <!-- Dots -->
    <div class="flex items-center gap-2">
      {#each Array(totalSteps) as _, index}
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

  <!-- Right spacer to center progress -->
  <div class="w-[44px]"></div>
</div>

<!-- Custom Confirmation Dialog -->
<Dialog.Root open={showConfirmDialog} onOpenChange={(open) => { if (!open) showConfirmDialog = false; }}>
  <Dialog.Content class="max-w-md">
    <Dialog.Header>
      <Dialog.Title>{i18n.t('shared.goBackTitle')}</Dialog.Title>
      <Dialog.Description>
        {confirmationMessage}
      </Dialog.Description>
    </Dialog.Header>

    <Dialog.Footer class="flex gap-3 justify-end">
      <Button variant="secondary" onclick={cancelGoHome}>
        {i18n.t('common.cancel')}
      </Button>
      <Button variant="destructive" onclick={confirmGoHome}>
        {i18n.t('shared.goBackConfirm')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
