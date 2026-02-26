<!--
  Component: DisplayLanguageSettings
  Purpose: Focused settings detail page for display currency and app language.
-->

<script lang="ts">
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import CheckIcon from '@lucide/svelte/icons/check';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import LocaleSelector from '$lib/components/common/LocaleSelector.svelte';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import { i18nStore } from '$lib/i18n';
  import { setDisplayCurrency, settingsStore } from '$lib/stores/settings.js';
  import {
    filterFiatCurrencyOptions,
    getFiatCurrencyOptions,
    QUICK_PICK_DISPLAY_CURRENCIES
  } from '$lib/utils/fiatDisplay.js';

  type DisplayLanguageSettingsProps = {
    onBack: () => void;
  };

  const { onBack }: DisplayLanguageSettingsProps = $props();

  const i18n = $derived($i18nStore);
  const settings = $derived($settingsStore);
  const displayCurrency = $derived(settings.displayCurrency);
  const fiatOptions = getFiatCurrencyOptions();
  const quickPickOptions = QUICK_PICK_DISPLAY_CURRENCIES
    .map((code) => fiatOptions.find((option) => option.code === code) ?? null)
    .filter((option): option is NonNullable<(typeof fiatOptions)[number]> => Boolean(option));

  let showAllCurrenciesSheet = $state(false);
  let currencySearchTerm = $state('');

  const filteredFiatOptions = $derived(
    filterFiatCurrencyOptions(fiatOptions, currencySearchTerm)
  );

  function chooseDisplayCurrency(code: string): void {
    setDisplayCurrency(code);
  }

  function openAllCurrenciesSheet(): void {
    currencySearchTerm = '';
    showAllCurrenciesSheet = true;
  }
</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col px-6 pb-6 pt-0 sm:px-8">
  <section class="flex min-h-0 flex-1 flex-col overflow-auto pt-3">
    <div class="space-y-4">
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground inline-flex w-fit items-center gap-1.5 text-sm"
        onclick={onBack}
      >
        <ArrowLeftIcon class="size-4" />
        {i18n.t('common.back')}
      </button>

      <section class="space-y-4">
        <div class="space-y-1">
          <h2 class="text-xl font-semibold">{i18n.t('wallet.settings.display.title')}</h2>
          <p class="text-muted-foreground text-sm">{i18n.t('wallet.settings.display.description')}</p>
        </div>

        <div class="space-y-4">
          <div class="space-y-2">
            <div class="flex items-center justify-between gap-4">
              <p class="text-sm font-medium">{i18n.t('wallet.settings.display.currency.label')}</p>
              <p class="text-muted-foreground text-xs">
                {i18n.t('wallet.settings.display.currency.current', { code: displayCurrency })}
              </p>
            </div>

            <p class="text-muted-foreground text-xs">{i18n.t('wallet.settings.display.currency.quickPicks')}</p>
            <div class="space-y-2">
              <div class="flex flex-wrap items-center gap-2">
                {#each quickPickOptions as option (option.code)}
                  <button
                    type="button"
                    class={`inline-flex h-8 items-center gap-1.5 rounded-md px-2.5 text-xs font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 ${
                      displayCurrency === option.code
                        ? 'bg-primary/14 text-foreground hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                        : 'bg-muted/65 text-muted-foreground hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'
                    }`}
                    onclick={() => chooseDisplayCurrency(option.code)}
                  >
                    <span>{option.symbol}</span>
                    <span>{option.code}</span>
                    {#if displayCurrency === option.code}
                      <CheckIcon class="size-3.5 shrink-0" />
                    {/if}
                  </button>
                {/each}
              </div>

              <button
                type="button"
                class="hover:bg-muted/45 flex w-full items-center justify-between rounded-lg px-3 py-2 text-left transition-colors"
                onclick={openAllCurrenciesSheet}
              >
                <span class="text-sm font-medium">{i18n.t('wallet.settings.display.currency.otherAction')}</span>
                <ChevronRightIcon class="text-muted-foreground size-4 shrink-0" />
              </button>
            </div>
          </div>

          <div class="space-y-2 pt-2">
            <p class="text-sm font-medium">{i18n.t('wallet.settings.display.language.label')}</p>
            <LocaleSelector triggerAriaLabel={i18n.t('wallet.settings.display.language.label')} />
            <p class="text-muted-foreground text-xs">{i18n.t('wallet.settings.display.language.description')}</p>
          </div>
        </div>
      </section>
    </div>
  </section>
</div>

<StandardRightSheet
  bind:isOpen={showAllCurrenciesSheet}
  title={i18n.t('wallet.settings.display.currency.sheetTitle')}
>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <SearchInput
      bind:value={currencySearchTerm}
      placeholder={i18n.t('wallet.settings.display.currency.searchPlaceholder')}
      inputClass="h-10"
    />

    {#if filteredFiatOptions.length === 0}
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.settings.display.currency.noResults')}</p>
    {:else}
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full pr-1">
          <div class="space-y-2 pb-1">
            {#each filteredFiatOptions as option (option.code)}
                <button
                  type="button"
                  class={`group flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 ${
                    displayCurrency === option.code
                      ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                      : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'
                  }`}
                  onclick={() => {
                    chooseDisplayCurrency(option.code);
                    showAllCurrenciesSheet = false;
                  }}
                >
                  <span class="min-w-0">
                    <span class="font-medium">{option.code}</span>
                    <span class="text-muted-foreground ml-2 truncate text-xs">{option.name}</span>
                  </span>
                  {#if displayCurrency === option.code}
                    <CheckIcon class="size-4 shrink-0" />
                  {/if}
                </button>
            {/each}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    {/if}
  </div>
</StandardRightSheet>
