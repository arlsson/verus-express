<!-- 
  WelcomeScreen Component for Verus Express Wallet
  Two-section layout: text content (top) and action buttons (bottom) with clean divider
  Features minimal design, reusable HelpLink component, and centered action section
-->

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import HelpSidebar from '$lib/components/common/HelpSidebar.svelte';
  import HelpLink from '$lib/components/common/HelpLink.svelte';
  import WalletCreation from '$lib/components/flows/WalletCreation/WalletCreation.svelte';

  function handleCreateWallet() {
    console.info('[WALLET] Create new wallet flow initiated');
    showCreateWallet = true;
  }

  function handleImportWallet() {
    console.info('[WALLET] Import existing wallet flow initiated');
    // TODO: Navigate to wallet import flow
  }

  let showWalletHelp = $state(false);
  let showCreateWallet = $state(false);

  const walletHelpContent = {
    sections: [
      {
        text: "A wallet is like a secure digital keychain that only you control."
      },
      {
        heading: "Your Private Keys",
        text: "These prove you own your money and identity. No one else can access them."
      },
      {
        heading: "Complete Control",
        text: "Send money globally, verify identity online - all without asking permission."
      }
    ]
  };
</script>

<main class="bg-background relative flex min-h-screen flex-col overflow-hidden">
  <!-- Simple light gray background -->
  <div class="absolute inset-0 bg-[#fbfbfb] dark:bg-[#111111]"></div>
  
  <!-- Text Section -->
  <div class="relative z-10 flex flex-1 items-center justify-center p-4">
    <div class="mx-auto max-w-xl text-left">
      <div class="space-y-4">
        <div class="relative">
          <h1 class="text-foreground leading-14 text-5xl tracking-tight font-bold">
            Your Digital Identity<br/>& Money, Unified
          </h1>
          <img 
            src="/icons/verus-express-icons2.webp" 
            alt="Verus Express" 
            class="absolute -left-24 top-[43px] h-[70px] w-auto -translate-y-1/2 icon-glow"
          />
        </div>
        <p class="text-muted-foreground text-lg max-w-[400px]">
          Manage your digital identity and money from one place you own. Your keys, your identity, your control.
        </p>
        <HelpLink 
          text="What's a wallet?"
          onclick={() => showWalletHelp = true}
        />
      </div>
    </div>
  </div>

 

  <!-- Action Section -->
  <div class="relative z-10 flex items-center justify-center py-12 bg-muted/10 border-t border-black/10 dark:border-white/20">
    <div class="flex flex-col items-center space-y-6 text-center">
      <div class="flex flex-col space-y-4">
        <Button
          variant="default"
          size="lg"
          onclick={handleCreateWallet}
          class="w-64 flex"
        >
          Let's get you started
        </Button>

        <Button
          variant="secondary"
          size="lg"
          onclick={handleImportWallet}
          class="w-64 flex"
        >
          I already have a wallet
        </Button>
      </div>

      <!-- Confidence Builder -->
      <div class="text-muted-foreground text-xs">
        Fully self-sovereign • Your keys, your coins
      </div>
    </div>
  </div>
</main>

<!-- Help Sidebar -->
<HelpSidebar 
  bind:isOpen={showWalletHelp}
  title="What's a Wallet?"
  content={walletHelpContent}
/>

<!-- Wallet Creation Flow -->
{#if showCreateWallet}
  <div class="fixed inset-0 z-50">
    <WalletCreation 
      onGoHome={() => {
        showCreateWallet = false;
      }}
    />
  </div>
{/if}

<style>
  .icon-glow {
    filter: drop-shadow(0 0 20px #A8C4FE) drop-shadow(0 0 40px #A8C4FE40) drop-shadow(0 0 60px #A8C4FE20);
  }
  
  :global(.dark) .icon-glow {
    filter: drop-shadow(0 0 20px #00329A) drop-shadow(0 0 40px #00329A40) drop-shadow(0 0 60px #00329A20);
  }
</style>
