<!-- 
  Root layout component for lite-wallet app
  Imports global styles, sets up theme detection, and renders page content
  Updated: Added mode-watcher for automatic light/dark theme detection
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import { ModeWatcher } from 'mode-watcher';
  import { Toaster } from 'svelte-sonner';
  import { initI18n } from '$lib/i18n';
  
  const { children } = $props();

  onMount(() => {
    initI18n();
  });
</script>

<!-- Theme detection and system preference monitoring -->
<ModeWatcher />
<Toaster theme="system" richColors position="bottom-right" />

<!-- Render page content -->
{@render children?.()}
