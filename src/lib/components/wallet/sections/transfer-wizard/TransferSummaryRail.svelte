<script lang="ts">
  import { i18nStore } from '$lib/i18n';
  import { cn } from '$lib/utils.js';
  import { getTransferSummaryLabels } from '$lib/transfer/transferWizardCopy';

  type SummaryRow = {
    label: string;
    primary: string;
    secondary?: string;
    breakAll?: boolean;
  };

  type TransferSummaryRailProps = {
    rows: SummaryRow[];
    warnings?: string[];
    class?: string;
  };

  /* eslint-disable prefer-const */
  let {
    rows,
    warnings = [],
    class: className = ''
  }: TransferSummaryRailProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);
  const labels = $derived(getTransferSummaryLabels(i18n.t));
</script>

<section class={cn('bg-[#EDEDED] dark:bg-[#28282B] h-full p-2', className)}>
  <div class="space-y-1">
    <h3 class="text-sm font-semibold">{labels.title}</h3>
  </div>

  {#if rows.length > 0}
    <dl class="mt-3 space-y-2.5">
      {#each rows as row}
        <div>
          <dt class="text-muted-foreground text-xs">{row.label}</dt>
          <dd class={cn('mt-1 text-sm', row.breakAll ? 'break-all' : '')}>
            <p>{row.primary}</p>
            {#if row.secondary}
              <p class="text-muted-foreground mt-0.5 text-[10px]">{row.secondary}</p>
            {/if}
          </dd>
        </div>
      {/each}
    </dl>
  {/if}

  {#if warnings.length > 0}
    <div class="mt-3 space-y-1">
      <p class="text-xs font-medium">{labels.warnings}</p>
      {#each warnings as warning}
        <p class="text-amber-600 dark:text-amber-400 text-xs">{warning}</p>
      {/each}
    </div>
  {/if}
</section>
