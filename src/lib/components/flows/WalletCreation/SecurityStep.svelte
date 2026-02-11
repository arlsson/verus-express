<!-- 
  Component: SecurityStep
  Purpose: Security guidelines step before showing seed phrase
  Last Updated: Includes acknowledgment control within step content for cleaner footer alignment
  Security: Ensures users read security guidelines before accessing sensitive data
-->

<script lang="ts">
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { i18nStore } from '$lib/i18n';

  let { securityAccepted = $bindable(false) }: { securityAccepted: boolean } = $props();
  const i18n = $derived($i18nStore);
</script>

<!-- Security Guidelines Content -->
<div class="w-full space-y-5">
  <div class="rounded-xl border border-amber-500/30 bg-amber-500/10 p-6">
    <p class="mb-4 text-sm font-medium text-amber-200">
      {i18n.t('walletCreation.security.warning')}
    </p>

    <ul class="space-y-2 text-sm text-foreground">
      <li class="flex items-start gap-2">
        <span class="text-amber-500">•</span>
        <span>{i18n.t('walletCreation.security.item1')}</span>
      </li>
      <li class="flex items-start gap-2">
        <span class="text-amber-500">•</span>
        <span>{i18n.t('walletCreation.security.item2')}</span>
      </li>
      <li class="flex items-start gap-2">
        <span class="text-amber-500">•</span>
        <span>{i18n.t('walletCreation.security.item3')}</span>
      </li>
      <li class="flex items-start gap-2">
        <span class="text-amber-500">•</span>
        <span>{i18n.t('walletCreation.security.item4')}</span>
      </li>
    </ul>
  </div>

  <label
    for="security-acceptance-main"
    class="flex cursor-pointer items-start gap-3 rounded-lg border border-border/60 bg-muted/20 px-4 py-3"
  >
    <Checkbox id="security-acceptance-main" bind:checked={securityAccepted} class="mt-0.5" />
    <span class="text-sm leading-5 text-foreground">
      {i18n.t('walletCreation.step3.checkbox')}
    </span>
  </label>
</div>
