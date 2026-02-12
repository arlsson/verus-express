<script lang="ts">
  import { Input } from '$lib/components/ui/input';
  import { i18nStore } from '$lib/i18n';

  const defaultOnInputChanged = (value: string) => {
    void value;
  };

  const defaultOnValidityChanged = (valid: boolean) => {
    void valid;
  };

  type TextImportStepProps = {
    importTextInput?: string;
    onInputChanged?: typeof defaultOnInputChanged;
    onValidityChanged?: typeof defaultOnValidityChanged;
  };

  /* eslint-disable prefer-const */
  let {
    importTextInput = '',
    onInputChanged = defaultOnInputChanged,
    onValidityChanged = defaultOnValidityChanged
  }: TextImportStepProps = $props();
  /* eslint-enable prefer-const */

  let localValue = $state('');
  const i18n = $derived($i18nStore);

  $effect(() => {
    localValue = importTextInput;
  });

  $effect(() => {
    onValidityChanged(localValue.trim().length > 0);
  });
</script>

<div class="mx-auto w-full max-w-[560px] space-y-3">
  <Input
    id="wallet-import-text-input"
    type="password"
    value={localValue}
    oninput={(event) => {
      localValue = (event.target as HTMLInputElement).value;
      onInputChanged(localValue);
    }}
    placeholder={i18n.t('walletImport.text.placeholder')}
    autocomplete="off"
    autocapitalize="off"
    spellcheck="false"
  />
  <p class="text-muted-foreground text-xs">{i18n.t('walletImport.text.helper')}</p>
</div>
