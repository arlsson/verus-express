<!-- 
  Component: HomeButton
  Purpose: Navigation button to return home, with confirmation for sensitive flows
  Last Updated: Created for wallet creation flow implementation  
  Security: Confirms before navigating away from sensitive operations
-->

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { i18nStore } from '$lib/i18n';

  // Props
  let {
    onGoHome = () => {},
    requireConfirmation = false,
    confirmationMessage = ''
  } = $props();

  const i18n = $derived($i18nStore);

  function handleClick() {
    const fallbackMessage = i18n.t('shared.homeConfirm');
    if (requireConfirmation) {
      if (globalThis.confirm(confirmationMessage || fallbackMessage)) {
        onGoHome();
      }
    } else {
      onGoHome();
    }
  }
</script>

<div class="absolute top-6 left-6">
  <Button
    variant="ghost"
    size="sm"
    onclick={handleClick}
    class="text-muted-foreground hover:text-foreground"
  >
    {i18n.t('shared.home')}
  </Button>
</div>
