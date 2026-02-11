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
  const selectedLabel = $derived(
    i18n.locale === 'nl' ? i18n.t('languageGate.option.nl') : i18n.t('languageGate.option.en')
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

    <section class="flex min-w-0 flex-1 flex-col">
      <div class="flex-1 flex items-center justify-center px-6 py-10 sm:px-8">
        <div class="w-full max-w-[560px] space-y-6">
          <div class="space-y-3">
            <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
              {i18n.t('languageGate.title')}
            </h1>
            <p class="text-muted-foreground text-sm">
              {i18n.t('languageGate.description')}
            </p>
          </div>

          <div class="w-full max-w-[520px] space-y-2">
            <label for="language-trigger" class="text-sm font-medium text-card-foreground">
              {i18n.t('languageGate.label')}
            </label>

            <DropdownMenu.Root>
              <DropdownMenu.Trigger id="language-trigger">
                {#snippet child({ props })}
                  <Button
                    {...props}
                    variant="outline"
                    class="w-full justify-between border-input bg-background px-3 py-2 text-left text-base"
                  >
                    <span>{selectedLabel}</span>
                    <ChevronDownIcon class="size-4 opacity-70" />
                  </Button>
                {/snippet}
              </DropdownMenu.Trigger>

              <DropdownMenu.Content align="start" class="w-[var(--bits-dropdown-menu-anchor-width)] min-w-[320px]">
                <DropdownMenu.RadioGroup value={i18n.locale}>
                  <DropdownMenu.RadioItem value="en" onclick={() => chooseLocale('en')}>
                    {i18n.t('languageGate.option.en')}
                  </DropdownMenu.RadioItem>
                  <DropdownMenu.RadioItem value="nl" onclick={() => chooseLocale('nl')}>
                    {i18n.t('languageGate.option.nl')}
                  </DropdownMenu.RadioItem>
                </DropdownMenu.RadioGroup>
              </DropdownMenu.Content>
            </DropdownMenu.Root>

            <p class="text-xs text-muted-foreground pt-2">
              {i18n.t('languageGate.hint')}
            </p>
          </div>
        </div>
      </div>

      <div class="shrink-0 border-t border-black/10 bg-muted/10 dark:border-white/20">
        <div class="mx-auto flex w-full max-w-[560px] items-center justify-end px-6 py-4 sm:px-8">
          <Button onclick={onContinue} class="w-48" size="lg">
            {i18n.t('languageGate.button')}
          </Button>
        </div>
      </div>
    </section>
  </div>
</main>
