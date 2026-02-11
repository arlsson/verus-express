<!-- 
  Component: BackupStep
  Purpose: Complete backup step with 2-column layout + bottom action
  Last Updated: Use rune cleanup to avoid Svelte module import.
  Security: Hover-to-reveal group blur protection, clears seed on unmount
-->

<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import { Spinner } from '$lib/components/ui/spinner';
  import BlurredSeedGrid from '$lib/components/shared/BlurredSeedGrid.svelte';
  import { i18nStore } from '$lib/i18n';

  // Props
  let {
    walletData = { name: '', emoji: '💰', color: 'blue', password: '', network: 'mainnet' },
    seedPhrase = '',
    onSeedGenerated = (seed: string) => {},
    canContinue = $bindable(false)
  } = $props();

  const i18n = $derived($i18nStore);

  // Local state for word groups and loading
  let currentWordGroup = $state(0);
  let isLoading = $state(false);
  let errorMessage = $state('');
  let seenGroups = $state<Set<number>>(new Set());
  let lastSeedPhrase = $state('');

  // Derived values
  const seedWords = $derived(seedPhrase.split(' '));
  const currentGroupWords = $derived(seedWords.slice(currentWordGroup * 8, (currentWordGroup + 1) * 8));
  const isLastGroup = $derived(currentWordGroup >= 2);
  const isFirstGroup = $derived(currentWordGroup === 0);
  const hasSeenAllGroups = $derived(seenGroups.size >= 3);
  const groupLabel = $derived(
    i18n.t('walletCreation.backup.groupLabel', {
      start: currentWordGroup * 8 + 1,
      end: Math.min((currentWordGroup + 1) * 8, 24)
    })
  );

  // Auto-generate seed if not provided
  $effect(() => {
    if (!seedPhrase && walletData.name) {
      generateSeed();
    }
  });

  $effect(() => {
    if (seedPhrase !== lastSeedPhrase) {
      lastSeedPhrase = seedPhrase;
      currentWordGroup = 0;
      seenGroups = new Set();
      canContinue = false;
    }
  });

  $effect(() => {
    if (!seedPhrase) return;
    if (seenGroups.has(currentWordGroup)) return;
    const nextSeen = new Set(seenGroups);
    nextSeen.add(currentWordGroup);
    seenGroups = nextSeen;
  });

  $effect(() => {
    canContinue = hasSeenAllGroups;
  });

  async function generateSeed() {
    if (!walletData.name.trim()) return;

    isLoading = true;
    errorMessage = '';

    try {
      console.info('[WALLET] Generating mnemonic for wallet:', walletData.name);

      const result = (await invoke('generate_mnemonic', {
        request: { word_count: 24 }
      })) as { seed_phrase: string };

      onSeedGenerated(result.seed_phrase);
      console.info('[WALLET] Mnemonic generated successfully');
    } catch (error) {
      console.error('[WALLET] Mnemonic generation failed:', error);
      errorMessage = i18n.t('walletCreation.backup.failed');
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
      <Spinner class="mx-auto size-10 text-primary" />
      <p class="text-muted-foreground">{i18n.t('walletCreation.backup.generating')}</p>
    </div>
  {:else if seedPhrase}
    <div class="space-y-4">
      <!-- Group Info -->
      <div class="mx-auto flex w-full max-w-[560px] items-center justify-between">
        <h3 class="text-card-foreground text-base font-semibold">{groupLabel}</h3>
        <p class="text-muted-foreground text-xs">
          {i18n.t('walletCreation.backup.groupProgress', { current: currentWordGroup + 1 })}
        </p>
      </div>

      <!-- Seed Words Grid with Group Blur Protection -->
      <div class="mx-auto w-full max-w-[560px]">
        <BlurredSeedGrid seedWords={currentGroupWords} startIndex={currentWordGroup * 8} showNumbers={true} />
      </div>

      <!-- Group Navigation -->
      <div class="flex items-center justify-center gap-3">
        {#if !isFirstGroup}
          <Button
            variant="outline"
            onclick={() => {
              currentWordGroup = Math.max(currentWordGroup - 1, 0);
            }}
            size="sm"
          >
            {i18n.t('walletCreation.backup.previousGroup')}
          </Button>
        {/if}

        {#if !isLastGroup}
          <Button
            onclick={() => {
              if (currentWordGroup < 2) {
                currentWordGroup = Math.min(currentWordGroup + 1, 2);
              }
            }}
            size="sm"
          >
            {i18n.t('walletCreation.backup.nextGroup')}
          </Button>
        {:else}
          <Badge variant="outline" class="border-border/80 bg-muted/30 px-3 py-1 text-xs font-medium">
            {i18n.t('walletCreation.backup.reviewComplete')}
          </Badge>
        {/if}
      </div>

      <!-- Progress Indicator -->
      <div class="text-center min-h-5">
        {#if !hasSeenAllGroups}
          <p class="text-muted-foreground text-xs">{i18n.t('walletCreation.backup.reviewHint')}</p>
        {/if}
      </div>
    </div>
  {:else}
    <div class="text-center space-y-3">
      <p class="text-muted-foreground">{i18n.t('walletCreation.backup.unable')}</p>
      <Button onclick={() => generateSeed()} variant="outline" size="sm">
        {i18n.t('common.retry')}
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
