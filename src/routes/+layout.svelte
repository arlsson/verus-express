<!-- 
  Root layout component for lite-wallet app
  Imports global styles, sets up theme detection, and renders page content
  Updated: Added mode-watcher for automatic light/dark theme detection
-->

<script lang="ts">
  import '../app.css';
  import { ModeWatcher } from 'mode-watcher';
  
  const { children } = $props();
</script>

<!-- Theme detection and system preference monitoring -->
<ModeWatcher />

<!-- Render page content -->
{@render children?.()}
