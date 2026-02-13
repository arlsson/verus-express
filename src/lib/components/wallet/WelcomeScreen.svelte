<!--
  WelcomeScreen Component for Verus Express Wallet
  Matches login screen proportions and hero image panel for visual consistency
-->

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import HelpDrawerLink from '$lib/components/common/HelpDrawerLink.svelte';
  import WalletCreation from '$lib/components/flows/WalletCreation/WalletCreation.svelte';
  import WalletImport from '$lib/components/flows/WalletImport/WalletImport.svelte';
  import ImportMethodList from '$lib/components/flows/WalletImport/ImportMethodList.svelte';
  import VerusIdGuardDock from '$lib/components/wallet/VerusIdGuardDock.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { ImportMethod } from '$lib/components/flows/WalletImport/types';

  const i18n = $derived($i18nStore);

  function handleCreateWallet() {
    console.info('[WALLET] Create new wallet flow initiated');
    showCreateWallet = true;
  }

  function handleImportWallet() {
    showImportOptionsDrawer = true;
  }

  function handleStartImportWalletFlow(method: ImportMethod) {
    selectedImportMethod = method;
    showImportOptionsDrawer = false;
    showWalletImport = true;
  }

  let showCreateWallet = $state(false);
  let showWalletImport = $state(false);
  let showImportOptionsDrawer = $state(false);
  let selectedImportMethod = $state<ImportMethod>('seed24');

  const walletHelpContent = $derived({
    topics: [
      {
        id: 'wallet-different',
        label: i18n.t('help.topic.walletDifferent'),
        title: i18n.t('help.topic.walletDifferent'),
        qas: [
          {
            id: 'wallet-different-accounts',
            question: i18n.t('help.walletDifferent.accountsQuestion'),
            answer: i18n.t('help.walletDifferent.accountsAnswer')
          },
          {
            id: 'wallet-different-identity',
            question: i18n.t('help.walletDifferent.identityQuestion'),
            answer: i18n.t('help.walletDifferent.identityAnswer')
          },
          {
            id: 'wallet-different-payments',
            question: i18n.t('help.walletDifferent.paymentsQuestion'),
            answer: i18n.t('help.walletDifferent.paymentsAnswer')
          },
          {
            id: 'wallet-different-trust',
            question: i18n.t('help.walletDifferent.trustQuestion'),
            answer: i18n.t('help.walletDifferent.trustAnswer')
          }
        ]
      },
      {
        id: 'keep-safe',
        label: i18n.t('help.topic.keepSafe'),
        title: i18n.t('help.topic.keepSafe'),
        qas: [
          {
            id: 'keep-safe-items',
            question: i18n.t('help.keepSafe.itemsQuestion'),
            answer: i18n.t('help.keepSafe.itemsAnswer')
          },
          {
            id: 'keep-safe-phone',
            question: i18n.t('help.keepSafe.phoneQuestion'),
            answer: i18n.t('help.keepSafe.phoneAnswer')
          },
          {
            id: 'keep-safe-compromised',
            question: i18n.t('help.keepSafe.compromisedQuestion'),
            answer: i18n.t('help.keepSafe.compromisedAnswer')
          }
        ]
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
      <div class="absolute inset-0 flex flex-col items-center pt-24">
        <img
          src="/images/verus-logo-white.svg"
          alt="Verus"
          class="h-8 w-auto cursor-default select-none"
        />
      </div>
    </section>

    <section class="flex min-w-0 flex-1 items-center justify-center px-6 py-10 sm:px-8">
      <div class="w-full max-w-[420px] space-y-8">
        <div>
          <h1 class="text-foreground text-4xl leading-tight tracking-tight font-bold">
            {i18n.t('welcome.titleLine1')} <br />{i18n.t('welcome.titleLine2')}
          </h1>
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
          <HelpDrawerLink
            linkText={i18n.t('help.link.needHelp')}
            title={i18n.t('help.sheet.title')}
            content={walletHelpContent}
          />
        </div>
      </div>
    </section>
  </div>
</main>

<VerusIdGuardDock context="welcome" defaultNetwork="mainnet" />

{#if showCreateWallet}
  <div class="fixed inset-0 z-50">
    <WalletCreation
      onGoHome={() => {
        showCreateWallet = false;
      }}
    />
  </div>
{/if}

{#if showWalletImport}
  <div class="fixed inset-0 z-50">
    <WalletImport
      initialMethod={selectedImportMethod}
      onGoHome={() => {
        showWalletImport = false;
        selectedImportMethod = 'seed24';
      }}
    />
  </div>
{/if}

<StandardRightSheet bind:isOpen={showImportOptionsDrawer} title={i18n.t('unlock.importMethods.title')}>
  <ImportMethodList
    showHeader={false}
    onSelect={(method) => {
      handleStartImportWalletFlow(method);
    }}
  />
</StandardRightSheet>
