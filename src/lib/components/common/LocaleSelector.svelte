<!--
  Component: LocaleSelector
  Purpose: Shared locale picker used by onboarding and settings surfaces.
-->

<script lang="ts">
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import { Button } from '$lib/components/ui/button';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { i18nStore, setLocale } from '$lib/i18n';
  import type { Locale } from '$lib/i18n';
  import { buildLocaleOptions } from '$lib/utils/localeOptions.js';

  type LocaleSelectorProps = {
    triggerId?: string;
    triggerAriaLabel: string;
    buttonClass?: string;
    contentClass?: string;
  };

  const {
    triggerId = 'locale-selector-trigger',
    triggerAriaLabel,
    buttonClass = '',
    contentClass = ''
  }: LocaleSelectorProps = $props();

  const i18n = $derived($i18nStore);
  const localeOptions = $derived(buildLocaleOptions(i18n.t));
  const selectedOption = $derived(
    localeOptions.find((option) => option.value === i18n.locale) ?? localeOptions[0]
  );

  function chooseLocale(locale: Locale): void {
    setLocale(locale);
  }
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger id={triggerId} aria-label={triggerAriaLabel}>
    {#snippet child({ props })}
      <Button
        {...props}
        variant="outline"
        class={`w-full justify-between border-input bg-background px-3 py-2 text-left text-sm font-normal ${buttonClass}`}
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
    class={`w-[var(--bits-dropdown-menu-anchor-width)] overflow-y-auto ${contentClass}`}
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
