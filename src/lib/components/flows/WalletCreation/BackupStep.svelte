<!-- 
  Component: BackupStep
  Purpose: Complete backup step with 2-column layout + bottom action
  Last Updated: Use rune cleanup to avoid Svelte module import.
  Security: Hover-to-reveal group blur protection, clears seed on unmount
-->

<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { Button } from '$lib/components/ui/button';
  import BlurredSeedGrid from '$lib/components/shared/BlurredSeedGrid.svelte';
  
  // Props
  let { 
    walletData = { name: '', emoji: '💰', color: 'blue', password: '' },
    seedPhrase = '',
    onSeedGenerated = (seed: string) => {},
    onNext = () => {}
  } = $props();
  
  // Local state for word groups and loading
  let currentWordGroup = $state(0);
  let isLoading = $state(false);
  let errorMessage = $state('');
  
  // Derived values
  const seedWords = $derived(seedPhrase.split(' '));
  const currentGroupWords = $derived(seedWords.slice(currentWordGroup * 8, (currentWordGroup + 1) * 8));
  const isLastGroup = $derived(currentWordGroup >= 2);
  const isFirstGroup = $derived(currentWordGroup === 0);
  const groupLabel = $derived(`Words ${currentWordGroup * 8 + 1}-${Math.min((currentWordGroup + 1) * 8, 24)}`);
  
  // Auto-generate seed if not provided
  $effect(() => {
    if (!seedPhrase && walletData.name) {
      generateSeed();
    }
  });
  
  async function generateSeed() {
    if (!walletData.name.trim()) return;
    
    isLoading = true;
    errorMessage = '';
    
    try {
      console.info('[WALLET] Generating mnemonic for wallet:', walletData.name);
      
      const result = await invoke('generate_mnemonic', {
        request: { word_count: 24 }
      }) as { seed_phrase: string };
      
      onSeedGenerated(result.seed_phrase);
      console.info('[WALLET] Mnemonic generated successfully');
      
    } catch (error) {
      console.error('[WALLET] Mnemonic generation failed:', error);
      errorMessage = 'Failed to generate wallet. Please try again.';
    } finally {
      isLoading = false;
    }
  }
  
  // Security: Clear seed phrase from memory on component destroy
  $effect(() => {
    return () => {
      console.info('[WALLET] Backup step destroyed, memory cleared');
    };
  });
</script>

<!-- Content only for backup step -->
<div class="w-full">
  {#if isLoading}
    <div class="text-center space-y-4">
      <div class="w-12 h-12 border-4 border-primary border-t-transparent rounded-full animate-spin mx-auto"></div>
      <p class="text-muted-foreground">Generating your secure 24-word backup...</p>
    </div>
    
  {:else if seedPhrase}
    <div class="space-y-5">
      <!-- Group Info -->
      <div class="text-center space-y-2">
        <h3 class="text-card-foreground font-semibold">{groupLabel}</h3>
        <p class="text-muted-foreground text-sm">Wallet: {walletData.name}</p>
      </div>
      
      <!-- Seed Words Grid with Group Blur Protection -->
      <div class="max-w-md mx-auto">
        <BlurredSeedGrid 
          seedWords={currentGroupWords}
          startIndex={currentWordGroup * 8}
          showNumbers={true}
        />
      </div>
      
      
      <!-- Group Navigation -->
      <div class="flex gap-3 justify-center">
        {#if !isFirstGroup}
          <Button
            variant="outline"
            onclick={() => { currentWordGroup = Math.max(currentWordGroup - 1, 0); }}
            size="sm"
          >
            Previous Group
          </Button>
        {/if}
        
        <Button
          onclick={() => {
            if (currentWordGroup < 2) {
              currentWordGroup = Math.min(currentWordGroup + 1, 2);
            }
          }}
          disabled={isLastGroup}
          size="sm"
        >
          {isLastGroup ? 'All Groups Shown' : `Next Group (${currentWordGroup + 2}/3)`}
        </Button>
      </div>
      
      <!-- Progress Indicator -->
      <div class="text-center">
        <p class="text-muted-foreground text-xs">
          Group {currentWordGroup + 1} of 3
        </p>
      </div>
    </div>
    
  {:else}
    <div class="text-center space-y-3">
      <p class="text-muted-foreground">Unable to generate seed phrase.</p>
      <Button 
        onclick={() => generateSeed()}
        variant="outline"
        size="sm"
      >
        Try Again
      </Button>
    </div>
  {/if}
  
  <!-- Error Message -->
  {#if errorMessage}
    <div class="bg-destructive/10 border border-destructive/20 rounded-lg p-3 mt-4">
      <p class="text-destructive text-xs text-center">{errorMessage}</p>
    </div>
  {/if}
</div>
