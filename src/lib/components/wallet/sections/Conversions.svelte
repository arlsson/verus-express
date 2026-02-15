<!--
  Component: Conversions
  Purpose: Entry wrapper for unified transfer wizard (convert intent).
-->

<script lang="ts">
  import TransferWizard from './TransferWizard.svelte';

  type ConversionsProps = {
    onClose?: () => void;
  };

  const defaultCloseHandler = () => {};

  /* eslint-disable prefer-const */
  let { onClose = defaultCloseHandler }: ConversionsProps = $props();
  /* eslint-enable prefer-const */
</script>

<TransferWizard entryIntent="convert" {onClose} />
