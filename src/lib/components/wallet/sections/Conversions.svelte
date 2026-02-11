<!-- 
  Component: Conversions
  Purpose: Placeholder for Conversions section
  Last Updated: Initial creation
  Security: No sensitive operations - placeholder only
-->

<script lang="ts">
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import { i18nStore } from '$lib/i18n';

  const i18n = $derived($i18nStore);
</script>

<div class="flex flex-col items-center justify-center h-full py-12">
  <RefreshCwIcon class="h-12 w-12 text-muted-foreground mb-4" />
  <h2 class="text-2xl font-semibold mb-2">{i18n.t('wallet.conversions.title')}</h2>
  <p class="text-muted-foreground">{i18n.t('wallet.placeholder.comingSoon')}</p>
</div>
