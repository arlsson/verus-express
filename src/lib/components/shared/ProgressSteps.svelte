<!-- 
  Component: ProgressSteps
  Purpose: Visual progress indicator for multi-step flows
  Last Updated: Created for wallet creation flow implementation
  Security: No sensitive data - UI component only
-->

<script lang="ts">
  import { i18nStore } from '$lib/i18n';

  const i18n = $derived($i18nStore);

  // Props
  let { 
    currentStep = 1,
    totalSteps = 4,
    stepLabels = []
  } = $props();

  const effectiveStepLabels = $derived(
    stepLabels.length > 0
      ? stepLabels
      : [
          i18n.t('walletCreation.step2.title'),
          i18n.t('walletCreation.step4.title'),
          i18n.t('walletCreation.step5.title'),
          i18n.t('walletCreation.step7.title')
        ]
  );
  
  // Calculate progress percentage
  const progressPercentage = $derived((currentStep / totalSteps) * 100);
</script>

<div class="space-y-4">
  <!-- Progress Bar -->
  <div class="w-full bg-muted rounded-full h-2">
    <div 
      class="bg-primary h-2 rounded-full transition-all duration-300 ease-in-out"
      style="width: {progressPercentage}%"
    ></div>
  </div>
  
  <!-- Step Indicators -->
  <div class="flex justify-between">
    {#each effectiveStepLabels as label, index}
      {@const stepNumber = index + 1}
      {@const isActive = stepNumber === currentStep}
      {@const isCompleted = stepNumber < currentStep}
      
      <div class="flex flex-col items-center space-y-2">
        <!-- Step Circle -->
        <div class="w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium
          {isCompleted 
            ? 'bg-primary text-primary-foreground' 
            : isActive 
              ? 'bg-primary/20 text-primary border-2 border-primary' 
              : 'bg-muted text-muted-foreground'}"
        >
          {#if isCompleted}
            ✓
          {:else}
            {stepNumber}
          {/if}
        </div>
        
        <!-- Step Label -->
        <span class="text-xs font-medium
          {isActive 
            ? 'text-primary' 
            : isCompleted 
              ? 'text-card-foreground' 
              : 'text-muted-foreground'}"
        >
          {label}
        </span>
      </div>
    {/each}
  </div>
  
  <!-- Step Text -->
  <div class="text-center">
    <p class="text-sm text-muted-foreground">
      {i18n.t('shared.stepOf', { current: currentStep, total: totalSteps })}
    </p>
  </div>
</div>
