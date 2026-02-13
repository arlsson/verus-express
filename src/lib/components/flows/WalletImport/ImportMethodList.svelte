<script lang="ts">
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import BookOpenIcon from '@lucide/svelte/icons/book-open';
  import SquarePenIcon from '@lucide/svelte/icons/square-pen';
  import * as Sheet from '$lib/components/ui/sheet';
  import { i18nStore } from '$lib/i18n';
  import type { ImportMethod } from './types';

  const defaultOnSelect = (method: ImportMethod) => {
    void method;
  };

  type ImportMethodListProps = {
    title?: string;
    showHeader?: boolean;
    onSelect?: typeof defaultOnSelect;
    onBack?: (() => void) | null;
  };

  /* eslint-disable prefer-const */
  let {
    title = '',
    showHeader = true,
    onSelect = defaultOnSelect,
    onBack = null
  }: ImportMethodListProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);
  const resolvedTitle = $derived(title || i18n.t('unlock.importMethods.title'));
</script>

<div>
  {#if showHeader && onBack}
    <button
      type="button"
      class="text-muted-foreground hover:text-foreground mb-3 inline-flex items-center gap-1.5 text-sm transition-colors"
      onclick={() => onBack?.()}
    >
      <ArrowLeftIcon class="size-4" />
      {i18n.t('unlock.importMethods.back')}
    </button>
  {/if}

  {#if showHeader}
    <Sheet.Header class="gap-1 p-0 pr-8 pt-4">
      <Sheet.Title class="text-base">{resolvedTitle}</Sheet.Title>
    </Sheet.Header>
  {/if}

  <div class="{showHeader ? 'mt-5' : ''} space-y-3">
    <button
      type="button"
      class="group w-full rounded-lg bg-muted/65 p-4 text-left transition-colors hover:bg-muted/65 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 dark:bg-muted/55 dark:hover:bg-muted/65"
      onclick={() => onSelect('seed24')}
    >
      <div class="flex items-start gap-3">
        <BookOpenIcon
          class="mt-0.5 h-6 w-6 shrink-0 text-foreground opacity-30 transition-opacity duration-150 group-hover:opacity-100 dark:opacity-45 dark:group-hover:opacity-100"
          absoluteStrokeWidth
          stroke-linecap="butt"
          aria-hidden="true"
        />
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
      class="group w-full rounded-lg bg-muted/65 p-4 text-left transition-colors hover:bg-muted/65 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 dark:bg-muted/55 dark:hover:bg-muted/65"
      onclick={() => onSelect('text')}
    >
      <div class="flex items-start gap-3">
        <SquarePenIcon
          class="mt-0.5 h-6 w-6 shrink-0 text-foreground opacity-30 transition-opacity duration-150 group-hover:opacity-100 dark:opacity-45 dark:group-hover:opacity-100"
          absoluteStrokeWidth
          stroke-linecap="butt"
          aria-hidden="true"
        />
        <div class="min-w-0">
          <p class="text-foreground text-sm font-semibold">{i18n.t('unlock.importMethods.textTitle')}</p>
          <p class="text-muted-foreground mt-1 text-xs">
            {i18n.t('unlock.importMethods.textDescription')}
          </p>
        </div>
      </div>
    </button>
  </div>
</div>
