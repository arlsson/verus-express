<!-- 
  Component: WalletCreation
  Purpose: Skeleton layout for wallet creation flow - routes to individual step components
  Last Updated: Added seed verification enforcement and password strength validation integration
  Security: Manages shared state and sensitive data clearing, enforces security acknowledgment
-->

<script lang="ts">
  // Components
  import { Button } from '$lib/components/ui/button';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import TopBar from '$lib/components/shared/TopBar.svelte';
  import StepLayout from '$lib/components/shared/StepLayout.svelte';
  import IntroStep from './IntroStep.svelte';
  import NameStep from './NameStep.svelte';
  import SecurityStep from './SecurityStep.svelte';
  import BackupStep from './BackupStep.svelte';
  import VerifyStep from './VerifyStep.svelte';
  import PasswordStep from './PasswordStep.svelte';
  import CompleteStep from './CompleteStep.svelte';
  import { goto } from '$app/navigation';
  
  type WalletData = {
    name: string;
    emoji: string;
    color: string;
    password: string;
  };

  type WalletUpdate = Partial<WalletData>;

  // Props
  let { onGoHome = () => {} } = $props();
  
  // Shared state for all steps
  let currentStep = $state(1);
  let walletData = $state<WalletData>({
    name: '',
    emoji: '💰',
    color: 'blue',
    password: ''
  });
  let seedPhrase = $state<string>(''); // Security: Cleared on unmount
  let verificationIndices = $state<number[]>([]);
  let securityAccepted = $state(false);
  let allVerificationFieldsFilled = $state(false);
  let verifyStepRef: any = $state(null);
  let canCreateWallet = $state(false);
  
  // Navigation functions
  function goToStep(step: number) {
    currentStep = step;
  }
  
  function nextStep() {
    currentStep++;
  }
  
  // Handle verification and continue
  function handleVerifyAndContinue() {
    if (verifyStepRef && verifyStepRef.verifyWords) {
      const isValid = verifyStepRef.verifyWords();
      if (isValid) {
        nextStep();
      }
      // If invalid, errors will show inline - no need to do anything else
    }
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
    securityAccepted = false;
  }
  
  // Security: Clear all sensitive data on component destroy
  $effect(() => {
    return () => {
      clearSensitiveData();
      console.info('[WALLET] Component destroyed, sensitive data cleared');
    };
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
      totalSteps={7}
      onGoHome={handleGoHome}
      requireConfirmation={currentStep >= 4}
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
          onUpdate={(data: WalletUpdate) => { walletData = { ...walletData, ...data }; }}
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
          <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">Import information before we begin.</h1>
          <p class="text-muted-foreground text-sm mt-4">Please read and acknowledge these important security guidelines before you seeyour recovery phrase.</p>
        </div>
        
        <SecurityStep slot="right" />
        
        <div slot="action" class="flex items-center gap-4">
          <div class="flex items-center space-x-3">
            <Checkbox 
              id="security-acceptance-main"
              bind:checked={securityAccepted}
            />
            <label 
              for="security-acceptance-main" 
              class="text-sm text-foreground cursor-pointer select-none"
            >
              I understand and will follow these guidelines
            </label>
          </div>
          <Button 
            onclick={() => nextStep()} 
            disabled={!securityAccepted}
            class="w-48" 
            size="lg"
          >
            Show my backup
          </Button>
        </div>
      </StepLayout>
      
    {:else if currentStep === 4}
      <StepLayout>
        <div slot="left">
          <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">Backup Recovery Phrase</h1>
          <p class="text-muted-foreground text-sm mt-4">Write down your 24-word backup in exact order. This is your only way to recover your wallet.</p>
        </div>
        
        <BackupStep slot="right" 
          walletData={walletData}
          seedPhrase={seedPhrase}
          onSeedGenerated={(seed: string) => { seedPhrase = seed; }}
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
      
    {:else if currentStep === 5}
      <StepLayout>
        <div slot="left">
          <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">Verify Your Backup</h1>
          <p class="text-muted-foreground text-sm mt-4">Enter the 3 requested words to confirm you wrote them down correctly.</p>
        </div>
        
        <VerifyStep slot="right" 
          seedPhrase={seedPhrase}
          verificationIndices={verificationIndices}
          onVerified={nextStep}
          onSetupVerification={(indices: number[]) => { verificationIndices = indices; }}
          onFieldsChanged={(filled: boolean) => { allVerificationFieldsFilled = filled; }}
          bind:this={verifyStepRef}
        />
        
        <Button slot="action" 
          onclick={handleVerifyAndContinue} 
          disabled={!allVerificationFieldsFilled}
          class="w-48" 
          size="lg"
        >
          Verify & Continue
        </Button>
      </StepLayout>
      
    {:else if currentStep === 6}
      <StepLayout>
        <div slot="left">
          <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">Set Password</h1>
          <p class="text-muted-foreground text-sm mt-4">Create a password to encrypt your wallet on this device.</p>
        </div>
        
        <PasswordStep slot="right" 
          walletData={walletData}
          onUpdate={(data: WalletUpdate) => { walletData = { ...walletData, ...data }; }}
          seedPhrase={seedPhrase}
          onWalletCreated={() => { seedPhrase = ''; nextStep(); }}
          onCanCreateChanged={(canCreate: boolean) => { canCreateWallet = canCreate; }}
        />
        
        <Button slot="action" 
          onclick={() => nextStep()} 
          disabled={!canCreateWallet}
          class="w-48" 
          size="lg"
        >
          Create Wallet
        </Button>
      </StepLayout>
      
    {:else if currentStep === 7}
      <StepLayout>
        <div slot="left">
          <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight text-green-700 dark:text-green-400">Wallet Ready!</h1>
          <p class="text-muted-foreground text-sm mt-4">Your secure wallet is created and ready to use.</p>
        </div>
        
        <CompleteStep slot="right" walletData={walletData} />
        
        <Button slot="action" 
          onclick={() => { clearSensitiveData(); goto('/wallet'); }} 
          class="w-48" 
          size="lg"
        >
          Open My Wallet
        </Button>
      </StepLayout>
    {/if}
    
  </div>
  
</main>
