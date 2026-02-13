<!--
  Component: Apps
  Purpose: Placeholder for Apps section
  Last Updated: Added for simplified sidebar navigation
  Security: No sensitive operations - placeholder only
-->

<script lang="ts">
  import AppWindowIcon from '@lucide/svelte/icons/app-window';
  import { i18nStore } from '$lib/i18n';

  const i18n = $derived($i18nStore);
</script>

<div class="flex h-full flex-col items-center justify-center py-12">
  <AppWindowIcon class="mb-4 h-12 w-12 text-muted-foreground" />
  <h2 class="mb-2 text-2xl font-semibold">{i18n.t('wallet.sidebar.apps')}</h2>
  <p class="text-muted-foreground">{i18n.t('wallet.placeholder.comingSoon')}</p>
</div>
