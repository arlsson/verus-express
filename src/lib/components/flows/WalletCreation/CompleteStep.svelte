<!-- 
  Component: CompleteStep
  Purpose: Finalizing/loading state before wallet is opened
  Last Updated: Streamlined to a compact status view with loading and error states
  Security: No sensitive data - status feedback only
-->

<script lang="ts">
  import { Spinner } from '$lib/components/ui/spinner';
  import { i18nStore } from '$lib/i18n';

  // Props
  let {
    isOpening = false,
    openError = ''
  } = $props();

  const i18n = $derived($i18nStore);
</script>

<!-- Content only for finalizing step -->
<div class="mx-auto flex w-full max-w-[560px] flex-col items-center justify-center space-y-6 py-4 text-center">
  <div class="flex h-20 w-20 items-center justify-center rounded-full bg-muted/30">
    {#if openError}
      <span class="text-4xl">⚠️</span>
    {:else if isOpening}
      <Spinner class="size-10 text-primary" />
    {:else}
      <span class="inline-block h-3 w-3 animate-pulse rounded-full bg-primary/80"></span>
    {/if}
  </div>

  <div class="w-full rounded-xl border border-border/70 bg-muted/20 p-4 text-left">
    <div class="space-y-2 text-sm">
      <div class="flex items-center gap-2 text-foreground">
        <span>✓</span>
        <span>{i18n.t('walletCreation.complete.statusCreated')}</span>
      </div>
      <div class="flex items-center gap-2 text-foreground">
        <span>✓</span>
        <span>{i18n.t('walletCreation.complete.statusSecured')}</span>
      </div>
      <div class="flex items-center gap-2 {openError ? 'text-destructive' : 'text-muted-foreground'}">
        {#if openError}
          <span>!</span>
          <span>{i18n.t('walletCreation.complete.statusOpenFailed')}</span>
        {:else}
          <span class="inline-block h-3 w-3 animate-pulse rounded-full bg-primary/80"></span>
          <span>{i18n.t('walletCreation.complete.statusOpening')}</span>
        {/if}
      </div>
    </div>
  </div>

  <p class="text-muted-foreground text-xs">
    {#if openError}
      {i18n.t('walletCreation.step7.loadingHint')}
    {:else if isOpening}
      {i18n.t('walletCreation.complete.waitingHint')}
    {:else}
      {i18n.t('walletCreation.step7.loadingHint')}
    {/if}
  </p>
</div>
