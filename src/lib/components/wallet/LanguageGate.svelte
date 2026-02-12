<!--
  LanguageGate
  Purpose: First-launch language selection shown before first wallet onboarding.
  Uses shadcn-svelte dropdown menu primitives for selection.
-->

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import { i18nStore, setLocale, type Locale } from '$lib/i18n';

  const { onContinue = () => {} }: { onContinue?: () => void } = $props();
  const i18n = $derived($i18nStore);
  const localeOptions = $derived([
    { value: 'en' as const, flag: '🇺🇸', label: i18n.t('languageGate.option.en') },
    { value: 'nl' as const, flag: '🇳🇱', label: i18n.t('languageGate.option.nl') }
  ]);
  const selectedOption = $derived(
    localeOptions.find((option) => option.value === i18n.locale) ?? localeOptions[0]
  );

  function chooseLocale(locale: Locale) {
    setLocale(locale);
  }
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

    <section class="flex min-w-0 flex-1 flex-col">
      <div class="flex-1 flex items-center justify-center px-6 py-10 sm:px-8">
        <div class="w-full max-w-[320px] space-y-6">
          <div>
            <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
              {i18n.t('languageGate.title')}
            </h1>
          </div>

          <div class="w-full">
            <DropdownMenu.Root>
              <DropdownMenu.Trigger id="language-trigger" aria-label={i18n.t('languageGate.title')}>
                {#snippet child({ props })}
                  <Button
                    {...props}
                    variant="outline"
                    class="w-full justify-between border-input bg-background px-3 py-2 text-left text-sm font-normal"
                  >
                    <span class="flex items-center gap-2">
                      <span aria-hidden="true">{selectedOption.flag}</span>
                      <span>{selectedOption.label}</span>
                    </span>
                    <ChevronDownIcon class="size-4 opacity-70" />
                  </Button>
                {/snippet}
              </DropdownMenu.Trigger>

              <DropdownMenu.Content
                align="start"
                class="w-[var(--bits-dropdown-menu-anchor-width)] overflow-y-auto"
                style="max-height: min(16rem, var(--bits-dropdown-menu-content-available-height));"
              >
                <DropdownMenu.RadioGroup value={i18n.locale}>
                  {#each localeOptions as option (option.value)}
                    <DropdownMenu.RadioItem value={option.value} onclick={() => chooseLocale(option.value)}>
                      <span class="flex min-w-0 items-center gap-2">
                        <span aria-hidden="true">{option.flag}</span>
                        <span>{option.label}</span>
                      </span>
                    </DropdownMenu.RadioItem>
                  {/each}
                </DropdownMenu.RadioGroup>
              </DropdownMenu.Content>
            </DropdownMenu.Root>
          </div>
        </div>
      </div>

      <div class="shrink-0 border-t border-black/10 bg-muted/10 dark:border-white/20">
        <div class="flex w-full items-center justify-end px-4 py-4 sm:px-4">
          <Button onclick={onContinue} class="w-48">
            {i18n.t('languageGate.button')}
          </Button>
        </div>
      </div>
    </section>
  </div>
</main>
