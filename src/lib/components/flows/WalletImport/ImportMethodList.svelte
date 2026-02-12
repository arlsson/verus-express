<script lang="ts">
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import { i18nStore } from '$lib/i18n';
  import type { ImportMethod } from './types';

  const defaultOnSelect = (method: ImportMethod) => {
    void method;
  };

  type ImportMethodListProps = {
    title?: string;
    description?: string;
    onSelect?: typeof defaultOnSelect;
    onBack?: (() => void) | null;
  };

  /* eslint-disable prefer-const */
  let {
    title = '',
    description = '',
    onSelect = defaultOnSelect,
    onBack = null
  }: ImportMethodListProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);
  const resolvedTitle = $derived(title || i18n.t('unlock.importMethods.title'));
  const resolvedDescription = $derived(description || i18n.t('unlock.importMethods.description'));
</script>

<div class="space-y-3">
  {#if onBack}
    <button
      type="button"
      class="text-muted-foreground hover:text-foreground inline-flex items-center gap-1.5 text-sm transition-colors"
      onclick={() => onBack?.()}
    >
      <ArrowLeftIcon class="size-4" />
      {i18n.t('unlock.importMethods.back')}
    </button>
  {/if}

  <div class="space-y-1">
    <h2 class="text-foreground text-base font-semibold">{resolvedTitle}</h2>
    <p class="text-muted-foreground text-sm">{resolvedDescription}</p>
  </div>

  <button
    type="button"
    class="border-input hover:bg-muted/60 w-full rounded-lg border p-4 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
    onclick={() => onSelect('seed24')}
  >
    <div class="flex items-start gap-3">
      <svg
        class="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path>
        <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2Z"></path>
      </svg>
      <div class="min-w-0">
        <p class="text-foreground text-sm font-semibold">{i18n.t('unlock.importMethods.seed24Title')}</p>
        <p class="text-muted-foreground mt-1 text-xs">
          {i18n.t('unlock.importMethods.seed24Description')}
        </p>
      </div>
    </div>
  </button>

  <button
    type="button"
    class="border-input hover:bg-muted/60 w-full rounded-lg border p-4 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
    onclick={() => onSelect('text')}
  >
    <div class="flex items-start gap-3">
      <svg
        class="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <path d="M15 7v10"></path>
        <path d="M9 7v10"></path>
        <path d="M5 4h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1Z"></path>
      </svg>
      <div class="min-w-0">
        <p class="text-foreground text-sm font-semibold">{i18n.t('unlock.importMethods.textTitle')}</p>
        <p class="text-muted-foreground mt-1 text-xs">
          {i18n.t('unlock.importMethods.textDescription')}
        </p>
      </div>
    </div>
  </button>
</div>
