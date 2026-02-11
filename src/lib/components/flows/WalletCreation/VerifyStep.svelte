<!-- 
  Component: VerifyStep
  Purpose: Complete verify step with 2-column layout + bottom action
  Last Updated: Added verifyWords() exposure and field completion tracking for parent component
  Security: Validates backup without storing sensitive data locally
-->

<script lang="ts">
  import { Input } from '$lib/components/ui/input';
  import { i18nStore } from '$lib/i18n';

  const i18n = $derived($i18nStore);

  // Props
  let {
    seedPhrase = '',
    verificationIndices = [],
    onVerified = () => {},
    onSetupVerification = (indices: number[]) => {},
    onFieldsChanged = (filled: boolean) => {} // NEW: callback for field completion state
  } = $props();

  // Local verification state
  let verificationWords = $state(['', '', '']);
  let verificationErrors = $state([false, false, false]);
  let hasAttempted = $state(false);
  let isVerified = $state(false);

  // Auto-setup verification indices if not provided
  $effect(() => {
    if (verificationIndices.length === 0 && seedPhrase) {
      const indices: number[] = [];
      while (indices.length < 3) {
        const randomIndex = Math.floor(Math.random() * 24);
        if (!indices.includes(randomIndex)) {
          indices.push(randomIndex);
        }
      }
      onSetupVerification(indices.sort((a, b) => a - b));
    }
  });

  // Get correct words for verification
  const seedWords = $derived(seedPhrase.split(' '));
  const correctWords = $derived(verificationIndices.map((index) => seedWords[index] || ''));

  // Watch for field changes and notify parent
  $effect(() => {
    const filled = verificationWords.every((word) => word.trim() !== '');
    onFieldsChanged(filled);
  });

  function handleWordInput(index: number, event: Event) {
    const target = event.target as HTMLInputElement;
    verificationWords[index] = target.value;

    // Clear error when user types
    if (hasAttempted) {
      verificationErrors[index] = false;
    }
  }

  // Expose verifyWords function - returns boolean for success/failure
  export function verifyWords(): boolean {
    hasAttempted = true;
    let hasErrors = false;

    verificationIndices.forEach((wordIndex, i) => {
      const userWord = verificationWords[i].toLowerCase().trim();
      const correctWord = correctWords[i].toLowerCase();

      if (userWord !== correctWord) {
        verificationErrors[i] = true;
        hasErrors = true;
      } else {
        verificationErrors[i] = false;
      }
    });

    if (!hasErrors) {
      console.info('[WALLET] Seed verification successful');
      isVerified = true;
      return true;
    } else {
      console.info('[WALLET] Seed verification failed');
      isVerified = false;
      return false;
    }
  }
</script>

<!-- Content only for verify step -->
<div class="space-y-5 max-w-sm mx-auto">
  <!-- Verification Inputs -->
  {#each verificationIndices as wordIndex, i}
    <div class="space-y-2">
      <label for="word-{i}" class="flex items-center gap-3 text-sm font-medium">
        <div class="w-8 h-6 bg-primary text-primary-foreground rounded flex items-center justify-center text-xs font-bold">
          {wordIndex + 1}
        </div>
        <span>{i18n.t('walletCreation.verify.word', { index: wordIndex + 1 })}</span>
      </label>

      <Input
        id="word-{i}"
        value={verificationWords[i]}
        oninput={(e) => handleWordInput(i, e)}
        placeholder={i18n.t('walletCreation.verify.enterWord')}
        autocomplete="off"
        class={verificationErrors[i] ? 'border-destructive' : ''}
      />

      {#if verificationErrors[i]}
        <div class="flex items-center gap-2 text-xs text-destructive">
          <span>❌</span>
          <span>{i18n.t('walletCreation.verify.incorrect')}</span>
        </div>
      {/if}
    </div>
  {/each}

  <!-- Verification Tip -->
  <div class="bg-muted/50 border border-border rounded-lg p-3">
    <div class="text-center space-y-1">
      <h4 class="text-card-foreground font-semibold text-sm">{i18n.t('walletCreation.verify.tipTitle')}</h4>
      <p class="text-xs text-muted-foreground">
        {i18n.t('walletCreation.verify.tipText')}
      </p>
    </div>
  </div>
</div>
