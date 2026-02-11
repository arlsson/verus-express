<!--
  WelcomeScreen Component for Verus Express Wallet
  Matches login screen proportions and hero image panel for visual consistency
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

<main class="bg-background relative flex min-h-screen overflow-hidden">
  <div class="absolute inset-0 bg-[#fbfbfb] dark:bg-[#111111]"></div>
  <div class="absolute top-0 right-0 left-0 z-20 h-11" data-tauri-drag-region aria-hidden="true"></div>

  <div class="relative z-10 flex min-h-screen w-full">
    <section class="relative hidden w-[clamp(320px,38vw,500px)] shrink-0 overflow-hidden md:block">
      <img
        src="/images/seedling-sky.png"
        alt=""
        aria-hidden="true"
        class="h-full w-full object-cover dark:hidden"
      />
      <img
        src="/images/seedling-sky-dark.png"
        alt=""
        aria-hidden="true"
        class="hidden h-full w-full object-cover dark:block"
      />
      <div class="absolute inset-0 flex flex-col justify-start items-start pl-12 pr-8 pt-20">
        <img
          src="/images/verus-logo-white.svg"
          alt="Verus"
          class="h-5 w-auto cursor-default select-none"
        />
        <p class="text-2xl leading-tight font-bold text-white text-balance dark:text-white mt-8 cursor-default select-none">
          {i18n.t('unlock.hero.tagline')}
        </p>
      </div>
    </section>

    <section class="flex min-w-0 flex-1 items-center justify-center px-6 py-10 sm:px-8">
      <div class="w-full max-w-[420px] space-y-8">
        <div class="space-y-4">
          <h1 class="text-foreground text-4xl leading-tight tracking-tight font-bold">
            {i18n.t('welcome.titleLine1')} <br />{i18n.t('welcome.titleLine2')}
          </h1>

          <HelpDrawerLink
            linkText={i18n.t('welcome.help.link')}
            title={i18n.t('welcome.help.title')}
            content={walletHelpContent}
          />
        </div>

        <div class="space-y-3">
          <Button variant="default" size="lg" onclick={handleCreateWallet} class="w-full">
            {i18n.t('welcome.cta.start')}
          </Button>

          <Button variant="secondary" size="lg" onclick={handleImportWallet} class="w-full">
            {i18n.t('welcome.cta.existing')}
          </Button>
        </div>

        <div class="text-muted-foreground text-xs">
          {i18n.t('welcome.footer.confidence')}
        </div>
      </div>
    </section>
  </div>
</main>

{#if showCreateWallet}
  <div class="fixed inset-0 z-50">
    <WalletCreation
      onGoHome={() => {
        showCreateWallet = false;
      }}
    />
  </div>
{/if}
