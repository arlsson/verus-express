<script lang="ts">
  import { i18nStore } from '$lib/i18n';
  import { cn } from '$lib/utils.js';
  import { getTransferSummaryLabels } from '$lib/transfer/transferWizardCopy';

  type TransferSummaryRailProps = {
    sourceValue: string;
    toValue: string;
    routeValue: string;
    amountValue: string;
    recipientValue: string;
    estimatedReceiveValue: string;
    networkFeeValue: string;
    warnings?: string[];
    class?: string;
  };

  /* eslint-disable prefer-const */
  let {
    sourceValue,
    toValue,
    routeValue,
    amountValue,
    recipientValue,
    estimatedReceiveValue,
    networkFeeValue,
    warnings = [],
    class: className = ''
  }: TransferSummaryRailProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);
  const labels = $derived(getTransferSummaryLabels(i18n.t));

  function formatValue(value: string): string {
    const trimmed = value.trim();
    return trimmed ? trimmed : labels.notSet;
  }
</script>

<section class={cn('bg-[#EDEDED] dark:bg-[#28282B] h-full p-2', className)}>
  <div class="space-y-1">
    <h3 class="text-sm font-semibold">{labels.title}</h3>
  </div>

  <dl class="mt-3 space-y-2.5">
    <div>
      <dt class="text-muted-foreground text-xs">{labels.from}</dt>
      <dd class="mt-1 text-sm">{formatValue(sourceValue)}</dd>
    </div>
    <div>
      <dt class="text-muted-foreground text-xs">{labels.to}</dt>
      <dd class="mt-1 text-sm">{formatValue(toValue)}</dd>
    </div>
    <div>
      <dt class="text-muted-foreground text-xs">{labels.route}</dt>
      <dd class="mt-1 text-sm">{formatValue(routeValue)}</dd>
    </div>
    <div>
      <dt class="text-muted-foreground text-xs">{labels.amount}</dt>
      <dd class="mt-1 text-sm">{formatValue(amountValue)}</dd>
    </div>
    <div>
      <dt class="text-muted-foreground text-xs">{labels.recipient}</dt>
      <dd class="mt-1 break-all text-sm">{formatValue(recipientValue)}</dd>
    </div>
    <div>
      <dt class="text-muted-foreground text-xs">{labels.estimatedReceive}</dt>
      <dd class="mt-1 text-sm">{formatValue(estimatedReceiveValue)}</dd>
    </div>
    <div>
      <dt class="text-muted-foreground text-xs">{labels.networkFee}</dt>
      <dd class="mt-1 text-sm">{formatValue(networkFeeValue)}</dd>
    </div>
  </dl>

  {#if warnings.length > 0}
    <div class="mt-3 space-y-1">
      <p class="text-xs font-medium">{labels.warnings}</p>
      {#each warnings as warning}
        <p class="text-amber-600 dark:text-amber-400 text-xs">{warning}</p>
      {/each}
    </div>
  {/if}
</section>
