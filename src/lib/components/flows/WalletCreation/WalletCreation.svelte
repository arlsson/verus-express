<!-- 
  Component: WalletCreation
  Purpose: Skeleton layout for wallet creation flow - routes to individual step components
  Last Updated: Refactored to be layout skeleton only
  Security: Manages shared state and sensitive data clearing
-->

<script lang="ts">
  import { onDestroy } from 'svelte';
  
  // Components
  import { Button } from '$lib/components/ui/button';
  import TopBar from '$lib/components/shared/TopBar.svelte';
  import StepLayout from '$lib/components/shared/StepLayout.svelte';
  import IntroStep from './IntroStep.svelte';
  import NameStep from './NameStep.svelte';
  import BackupStep from './BackupStep.svelte';
  import VerifyStep from './VerifyStep.svelte';
  import PasswordStep from './PasswordStep.svelte';
  import CompleteStep from './CompleteStep.svelte';
  
  // Props
  let { onGoHome = () => {} } = $props();
  
  // Shared state for all steps
  let currentStep = $state(1);
  let walletData = $state({
    name: '',
    emoji: '💰',
    color: 'blue',
    password: ''
  });
  let seedPhrase = $state(''); // Security: Cleared on unmount
  let verificationIndices = $state<number[]>([]);
  
  // Navigation functions
  function goToStep(step: number) {
    currentStep = step;
  }
  
  function nextStep() {
    currentStep++;
  }
  
  // Clear sensitive data and go home
  function handleGoHome() {
    console.info('[WALLET_CREATION] Clearing sensitive data before navigation');
    clearSensitiveData();
    console.info('[WALLET_CREATION] Calling parent onGoHome');
    onGoHome();
  }
  
  function clearSensitiveData() {
    seedPhrase = '';
    walletData = { name: '', emoji: '💰', color: 'blue', password: '' };
    verificationIndices = [];
  }
  
  // Security: Clear all sensitive data on component destroy
  onDestroy(() => {
    clearSensitiveData();
    console.info('[WALLET] Component destroyed, sensitive data cleared');
  });
</script>

<!-- Main Layout (simplified to work in fixed overlay) -->
<main class="h-screen flex flex-col overflow-hidden">
  <!-- Background (matches WelcomeScreen) -->
  <div class="absolute inset-0 bg-[#fbfbfb] dark:bg-[#111111]"></div>
  
  <!-- Top Bar with Progress and Home -->
  <div class="relative z-20 shrink-0">
    <TopBar 
      currentStep={currentStep}
      totalSteps={6}
      onGoHome={handleGoHome}
      requireConfirmation={currentStep >= 3}
      confirmationMessage="Are you sure you want to go back? Your wallet creation progress will be lost and any seed phrase will be cleared."
    />
  </div>
  
  <!-- Step Content using reusable layout -->
  <div class="relative z-10 flex-1">
    
    {#if currentStep === 1}
      <StepLayout>
        <div slot="left">
          <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">Let's get you started with a new wallet.</h1>
          <p class="text-muted-foreground text-sm mt-4">Together, we'll get you started to be self-sovereign.</p>
        </div>
        
        <IntroStep slot="right" />
        
        <Button slot="action" size="lg" onclick={() => nextStep()} class="w-48" >
          I understand, continue
        </Button>
      </StepLayout>
      
    {:else if currentStep === 2}
      <StepLayout>
        <div slot="left">
          <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">Choose a name for your wallet.</h1>
          <p class="text-muted-foreground text-sm mt-4">Personalize your wallet with a name, emoji, and color.</p>
          <p class="text-muted-foreground text-sm mt-4">Name examples: Savings, Investments, Business, Personal, etc.</p>
        </div>
        
        <NameStep slot="right" 
          walletData={walletData}
          onUpdate={(data) => { walletData = { ...walletData, ...data }; }}
          errorMessage=""
        />
        
        <Button slot="action" 
          onclick={() => { nextStep(); }} 
          disabled={!walletData.name.trim() || /[/\\:*?"<>|]/.test(walletData.name)}
          class="w-48" 
          size="lg"
        >
          Continue
        </Button>
      </StepLayout>
      
    {:else if currentStep === 3}
      <StepLayout>
        <div slot="left">
          <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">Backup Recovery Phrase</h1>
          <p class="text-muted-foreground text-sm mt-4">Write down your 24-word backup in exact order. This is your only way to recover your wallet.</p>
        </div>
        
        <BackupStep slot="right" 
          walletData={walletData}
          seedPhrase={seedPhrase}
          onSeedGenerated={(seed) => { seedPhrase = seed; }}
        />
        
        <Button slot="action" 
          onclick={() => nextStep()} 
          disabled={!seedPhrase}
          class="w-48" 
          size="lg"
        >
          I've Written It Down
        </Button>
      </StepLayout>
      
    {:else if currentStep === 4}
      <StepLayout>
        <div slot="left">
          <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">Verify Your Backup</h1>
          <p class="text-muted-foreground text-sm mt-4">Enter specific words to confirm you wrote them down correctly.</p>
        </div>
        
        <VerifyStep slot="right" 
          seedPhrase={seedPhrase}
          verificationIndices={verificationIndices}
          onVerified={nextStep}
          onSetupVerification={(indices) => { verificationIndices = indices; }}
        />
        
        <Button slot="action" 
          onclick={() => nextStep()} 
          class="w-48" 
          size="lg"
        >
          Continue
        </Button>
      </StepLayout>
      
    {:else if currentStep === 5}
      <StepLayout>
        <div slot="left">
          <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">Set Password</h1>
          <p class="text-muted-foreground text-sm mt-4">Create a password to encrypt your wallet on this device.</p>
        </div>
        
        <PasswordStep slot="right" 
          walletData={walletData}
          onUpdate={(data) => { walletData = { ...walletData, ...data }; }}
          seedPhrase={seedPhrase}
          onWalletCreated={() => { seedPhrase = ''; nextStep(); }}
        />
        
        <Button slot="action" 
          onclick={() => nextStep()} 
          disabled={!walletData.password || walletData.password.length < 8}
          class="w-48" 
          size="lg"
        >
          Create Wallet
        </Button>
      </StepLayout>
      
    {:else if currentStep === 6}
      <StepLayout>
        <div slot="left">
          <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight text-green-700 dark:text-green-400">Wallet Ready!</h1>
          <p class="text-muted-foreground text-sm mt-4">Your secure wallet is created and ready to use.</p>
        </div>
        
        <CompleteStep slot="right" walletData={walletData} />
        
        <Button slot="action" 
          onclick={() => { clearSensitiveData(); onGoHome(); }} 
          class="w-48" 
          size="lg"
        >
          Open My Wallet
        </Button>
      </StepLayout>
    {/if}
    
  </div>
  
</main>
