<!-- 
  WelcomeScreen Component for Verus Express Wallet
  Two-section layout: text content (top) and action buttons (bottom) with clean divider
  Features minimal design, reusable HelpLink component, and centered action section
-->

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import HelpDrawerLink from '$lib/components/common/HelpDrawerLink.svelte';
  import WalletCreation from '$lib/components/flows/WalletCreation/WalletCreation.svelte';
  import { i18nStore } from '$lib/i18n';

  const i18n = $derived($i18nStore);

  function handleCreateWallet() {
    console.info('[WALLET] Create new wallet flow initiated');
    showCreateWallet = true;
  }

  function handleImportWallet() {
    console.info('[WALLET] Import existing wallet flow initiated');
    // TODO: Navigate to wallet import flow
  }

  let showCreateWallet = $state(false);

  const walletHelpContent = $derived({
    sections: [
      {
        text: i18n.t('welcome.help.intro')
      },
      {
        heading: i18n.t('welcome.help.privateKeysHeading'),
        text: i18n.t('welcome.help.privateKeysText')
      },
      {
        heading: i18n.t('welcome.help.completeControlHeading'),
        text: i18n.t('welcome.help.completeControlText')
      }
    ]
  });
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
            {i18n.t('welcome.titleLine1')} <br/>{i18n.t('welcome.titleLine2')}
          </h1>
          <img 
            src="/icons/verus-express-icons2.webp" 
            alt="Verus Express" 
            class="absolute -left-24 top-[43px] h-[70px] w-auto -translate-y-1/2 icon-glow"
          />
        </div>
        
        <HelpDrawerLink
          linkText={i18n.t('welcome.help.link')}
          title={i18n.t('welcome.help.title')}
          content={walletHelpContent}
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
          {i18n.t('welcome.cta.start')}
        </Button>

        <Button
          variant="secondary"
          size="lg"
          onclick={handleImportWallet}
          class="w-64 flex"
        >
          {i18n.t('welcome.cta.existing')}
        </Button>
      </div>

      <!-- Confidence Builder -->
      <div class="text-muted-foreground text-xs">
        {i18n.t('welcome.footer.confidence')}
      </div>
    </div>
  </div>
</main>

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
