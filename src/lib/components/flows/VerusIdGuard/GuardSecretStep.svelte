<script lang="ts">
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { i18nStore } from '$lib/i18n';
  import type { GuardFlowMode } from './types';

  type GuardSecretStepProps = {
    mode: GuardFlowMode;
    importText: string;
    // eslint-disable-next-line no-unused-vars
    onImportTextChange?: (value: string) => void;
  };

  const defaultStringHandler = (value: string) => {
    void value;
  };

  /* eslint-disable prefer-const */
  let {
    mode,
    importText,
    onImportTextChange = defaultStringHandler
  }: GuardSecretStepProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);
  const actionLabel = $derived(mode === 'revoke' ? i18n.t('guard.mode.revoke') : i18n.t('guard.mode.recover'));
</script>

<div class="mx-auto w-full max-w-[560px] space-y-6 py-4">
  <div class="space-y-2 text-center">
    <h2 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
      {i18n.t('guard.flow.secret.title')}
    </h2>
    <p class="text-muted-foreground text-sm">
      {i18n.t('guard.flow.secret.description', { action: actionLabel })}
    </p>
  </div>

  <div class="bg-muted/20 border-border/70 space-y-5 rounded-xl border p-5">
    <div class="space-y-2">
      <Label for="guard-import-text">{i18n.t('guard.flow.secret.inputLabel')}</Label>
      <Input
        id="guard-import-text"
        type="password"
        value={importText}
        oninput={(event) => onImportTextChange((event.target as HTMLInputElement).value)}
        placeholder={i18n.t('guard.flow.secret.inputPlaceholder')}
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
      />
      <p class="text-muted-foreground text-xs">{i18n.t('guard.flow.secret.inputHelp')}</p>
    </div>
  </div>
</div>
