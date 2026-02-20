<script lang="ts">
  import { splitPrivateVerusWordmark } from '$lib/utils/privateVerusWordmark';

  type PrivateVerusWordmarkProps = {
    label: string;
  };

  const { label }: PrivateVerusWordmarkProps = $props();
  const parts = $derived(splitPrivateVerusWordmark(label));
</script>

<span class="inline-flex items-baseline">
  {#if parts.hasBadge}
    {#if parts.leading}
      <span>{parts.leading}</span>
    {/if}
    <span class={`inline-block text-[0.52em] leading-none font-semibold tracking-[0.14em] uppercase ${
      parts.leading ? 'ml-1 -translate-y-[0.55em]' : '-translate-y-[0.55em]'
    }`}>
      {parts.badge}
    </span>
    {#if parts.trailing}
      <span class="ml-1">{parts.trailing}</span>
    {/if}
  {:else}
    <span>{parts.leading}</span>
  {/if}
</span>
