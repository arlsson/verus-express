<script lang="ts">
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { i18nStore } from '$lib/i18n';
  import type { GuardFlowMode } from './types';

  type GuardTargetStepProps = {
    mode: GuardFlowMode;
    targetIdentity: string;
    primaryAddress?: string;
    busy?: boolean;
    onTargetIdentityChange?: (value: string) => void;
    onPrimaryAddressChange?: (value: string) => void;
  };

  const defaultStringHandler = (value: string) => {
    void value;
  };

  /* eslint-disable prefer-const */
  let {
    mode,
    targetIdentity,
    primaryAddress = '',
    busy = false,
    onTargetIdentityChange = defaultStringHandler,
    onPrimaryAddressChange = defaultStringHandler
  }: GuardTargetStepProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);
  const isRecover = $derived(mode === 'recover');
</script>

<div class="mx-auto w-full max-w-[560px] space-y-6 py-4">
  <div class="space-y-2 text-center">
    <h2 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
      {i18n.t('guard.flow.target.title')}
    </h2>
    <p class="text-muted-foreground text-sm">
      {isRecover
        ? i18n.t('guard.flow.target.descriptionRecover')
        : i18n.t('guard.flow.target.descriptionRevoke')}
    </p>
  </div>

  <div class="bg-muted/20 border-border/70 space-y-5 rounded-xl border p-5">
    <div class="space-y-2">
      <Label for="guard-target-id">{i18n.t('guard.flow.target.identityLabel')}</Label>
      <Input
        id="guard-target-id"
        value={targetIdentity}
        oninput={(event) => onTargetIdentityChange((event.target as HTMLInputElement).value)}
        placeholder={i18n.t('guard.flow.target.identityPlaceholder')}
        autocapitalize="off"
        spellcheck="false"
      />
      <p class="text-muted-foreground text-xs">{i18n.t('guard.flow.target.identityHelp')}</p>
    </div>

    {#if isRecover}
      <div class="space-y-2">
        <Label for="guard-primary-address">{i18n.t('guard.flow.target.primaryLabel')}</Label>
        <Input
          id="guard-primary-address"
          value={primaryAddress}
          oninput={(event) => onPrimaryAddressChange((event.target as HTMLInputElement).value)}
          placeholder={i18n.t('guard.flow.target.primaryPlaceholder')}
          autocapitalize="off"
          spellcheck="false"
        />
        <p class="text-muted-foreground text-xs">{i18n.t('guard.flow.target.primaryHelp')}</p>
      </div>
    {/if}
  </div>
</div>
