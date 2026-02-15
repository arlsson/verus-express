<!--
  Component: Send
  Purpose: Entry wrapper for unified transfer wizard (send intent).
-->

<script lang="ts">
  import TransferWizard from './TransferWizard.svelte';

  type SendProps = {
    onClose?: () => void;
  };

  const defaultCloseHandler = () => {};

  /* eslint-disable prefer-const */
  let { onClose = defaultCloseHandler }: SendProps = $props();
  /* eslint-enable prefer-const */
</script>

<TransferWizard entryIntent="send" {onClose} />
