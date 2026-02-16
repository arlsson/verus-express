<script lang="ts">
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import { i18nStore } from '$lib/i18n';
  import { cn } from '$lib/utils.js';
  import { getTransferSummaryLabels } from '$lib/transfer/transferWizardCopy';

  type SummaryRow = {
    label: string;
    primary: string;
    secondary?: string;
    breakAll?: boolean;
    iconCoinId?: string;
    iconCoinName?: string;
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

<section class={cn('h-full p-1', className)}>
  <div class="space-y-1">
    <h3 class="text-base leading-tight font-semibold">{labels.title}</h3>
  </div>

  {#if rows.length > 0}
    <dl class="mt-3 space-y-3.5">
      {#each rows as row}
        <div>
          <dt class="text-muted-foreground text-xs leading-tight">{row.label}</dt>
          {#if row.iconCoinId}
            <dd class="mt-1.5">
              <div class="flex items-center gap-2.5">
                <CoinIcon
                  coinId={row.iconCoinId}
                  coinName={row.iconCoinName ?? row.primary}
                  size={18}
                  decorative={true}
                />
                <div class="min-w-0">
                  <p class={cn('truncate text-sm leading-tight font-semibold', row.breakAll ? 'break-all' : '')}>{row.primary}</p>
                  {#if row.secondary}
                    <p class="text-muted-foreground mt-0.5 truncate text-xs">{row.secondary}</p>
                  {/if}
                </div>
              </div>
            </dd>
          {:else}
            <dd class={cn('mt-1.5 text-sm leading-tight font-medium', row.breakAll ? 'break-all' : '')}>
              <p>{row.primary}</p>
              {#if row.secondary}
                <p class="text-muted-foreground mt-0.5 text-xs">{row.secondary}</p>
              {/if}
            </dd>
          {/if}
        </div>
      {/each}
    </dl>
  {/if}

  {#if warnings.length > 0}
    <div class="mt-4 space-y-1.5">
      <p class="text-xs font-medium">{labels.warnings}</p>
      {#each warnings as warning}
        <p class="text-amber-600 dark:text-amber-400 text-xs">{warning}</p>
      {/each}
    </div>
  {/if}
</section>
