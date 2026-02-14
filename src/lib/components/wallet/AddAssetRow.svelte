<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { AddAssetEntry } from '$lib/stores/addAssetCatalog.js';

  const noop = (asset: AddAssetEntry): void => {
    void asset;
  };

  type AddAssetRowProps = {
    entry: AddAssetEntry;
    busy?: boolean;
    onAdd?: typeof noop;
  };

  /* eslint-disable prefer-const */
  let { entry, busy = false, onAdd = noop }: AddAssetRowProps = $props();
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
      <span
        class="text-emerald-700 dark:text-emerald-300"
        aria-label={i18n.t('wallet.addAsset.stateAdded')}
        title={i18n.t('wallet.addAsset.stateAdded')}
      >
        <CheckIcon class="h-4 w-4" />
      </span>
    {:else}
      <Button
        variant="ghost"
        size="icon"
        onclick={() => onAdd(entry)}
        disabled={busy}
        class="text-primary h-8 w-8 rounded-md bg-transparent hover:bg-transparent"
        aria-label={busy ? i18n.t('wallet.addAsset.adding') : i18n.t('wallet.addAsset.add')}
      >
        <PlusIcon class="h-4 w-4" />
      </Button>
    {/if}
  </div>

</li>
