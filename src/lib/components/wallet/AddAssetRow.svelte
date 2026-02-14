<script lang="ts">
  import MinusIcon from '@lucide/svelte/icons/minus';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { Badge } from '$lib/components/ui/badge';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { AddAssetEntry } from '$lib/stores/addAssetCatalog.js';

  const noop = (asset: AddAssetEntry): void => {
    void asset;
  };

  type AddAssetRowProps = {
    entry: AddAssetEntry;
    busy?: boolean;
    onAction?: typeof noop;
  };

  /* eslint-disable prefer-const */
  let { entry, busy = false, onAction = noop }: AddAssetRowProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);

  const protocolLabel = $derived(entry.proto.toUpperCase());
</script>

<li class="flex items-center gap-3 rounded-md border border-border/60 bg-background px-3 py-2.5">
  <CoinIcon coinId={entry.id} coinName={entry.displayName} size={28} showBadge decorative />

  <div class="min-w-0 flex-1">
    <p class="truncate text-sm font-semibold text-foreground">{entry.displayName}</p>
    <p class="truncate text-xs text-muted-foreground">{entry.displayTicker}</p>
  </div>

  <div class="flex shrink-0 items-center gap-2">
    <Badge variant="outline" class="border-border/80 text-[10px] uppercase tracking-wide">{protocolLabel}</Badge>
    {#if entry.status === 'added'}
      <button
        type="button"
        class="text-destructive inline-flex h-8 w-8 items-center justify-center rounded-md bg-transparent transition-opacity hover:opacity-80 focus:bg-transparent active:bg-transparent disabled:opacity-45 dark:bg-transparent dark:hover:bg-transparent dark:focus:bg-transparent dark:active:bg-transparent"
        onclick={() => onAction(entry)}
        disabled={busy}
        aria-label={busy ? i18n.t('wallet.addAsset.disabling') : i18n.t('wallet.addAsset.disable')}
        title={busy ? i18n.t('wallet.addAsset.disabling') : i18n.t('wallet.addAsset.disable')}
      >
        <MinusIcon class="h-4 w-4" />
      </button>
    {:else}
      <button
        type="button"
        class="text-primary inline-flex h-8 w-8 items-center justify-center rounded-md bg-transparent transition-opacity hover:opacity-80 focus:bg-transparent active:bg-transparent disabled:opacity-45 dark:bg-transparent dark:hover:bg-transparent dark:focus:bg-transparent dark:active:bg-transparent"
        onclick={() => onAction(entry)}
        disabled={busy}
        aria-label={busy ? i18n.t('wallet.addAsset.adding') : i18n.t('wallet.addAsset.add')}
      >
        <PlusIcon class="h-4 w-4" />
      </button>
    {/if}
  </div>

</li>
