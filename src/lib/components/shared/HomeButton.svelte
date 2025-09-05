<!-- 
  Component: HomeButton
  Purpose: Navigation button to return home, with confirmation for sensitive flows
  Last Updated: Created for wallet creation flow implementation  
  Security: Confirms before navigating away from sensitive operations
-->

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  
  // Props
  let { 
    onGoHome = () => {},
    requireConfirmation = false,
    confirmationMessage = 'Are you sure you want to go back? Your progress will be lost.'
  } = $props();
  
  function handleClick() {
    if (requireConfirmation) {
      if (confirm(confirmationMessage)) {
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
    ← Home
  </Button>
</div>
