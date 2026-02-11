<!-- 
  Component: CompleteStep
  Purpose: Complete success step with 2-column layout + bottom action
  Last Updated: Refactored from SuccessStep to new layout
  Security: No sensitive data - success confirmation only
-->

<script lang="ts">
  import { i18nStore, networkLocaleKey } from '$lib/i18n';

  // Props
  let {
    walletData = { name: '', emoji: '💰', color: 'blue', password: '', network: 'mainnet' },
    onFinish = () => {}
  } = $props();

  const i18n = $derived($i18nStore);

  // Color class lookup
  const colorOptions = [
    { name: 'blue', class: 'bg-blue-100 dark:bg-blue-900' },
    { name: 'green', class: 'bg-green-100 dark:bg-green-900' },
    { name: 'purple', class: 'bg-purple-100 dark:bg-purple-900' },
    { name: 'orange', class: 'bg-orange-100 dark:bg-orange-900' },
    { name: 'pink', class: 'bg-pink-100 dark:bg-pink-900' },
    { name: 'yellow', class: 'bg-yellow-100 dark:bg-yellow-900' }
  ];

  const selectedColorClass = $derived(
    colorOptions.find((c) => c.name === walletData.color)?.class || colorOptions[0].class
  );
</script>

<!-- Content only for complete step -->
<div class="space-y-5 max-w-md mx-auto text-center">
  <!-- Success Icon -->
  <div class="flex justify-center">
    <div class="w-16 h-16 bg-green-100 dark:bg-green-900/20 rounded-full flex items-center justify-center">
      <span class="text-3xl">✅</span>
    </div>
  </div>

  <!-- Wallet Preview -->
  <div class="flex justify-center">
    <div class="flex items-center gap-3 p-4 bg-muted/30 border border-border rounded-lg">
      <div class="{selectedColorClass} w-12 h-12 rounded-full flex items-center justify-center">
        <span class="text-xl" role="img">{walletData.emoji}</span>
      </div>
      <span class="text-card-foreground font-medium text-lg">
        {walletData.name}
      </span>
    </div>
  </div>

  <div class="text-xs text-muted-foreground">
    {i18n.t('walletCreation.complete.network', {
      network: i18n.t(networkLocaleKey(walletData.network))
    })}
  </div>

  <!-- Success Details -->
  <div class="bg-green-50 dark:bg-green-900/10 border border-green-200 dark:border-green-800 rounded-lg p-4 space-y-2">
    <h3 class="text-green-800 dark:text-green-400 font-semibold text-sm">
      {i18n.t('walletCreation.complete.nextTitle')}
    </h3>
    <div class="text-xs text-green-700 dark:text-green-300 space-y-1">
      <div class="flex items-center gap-2">
        <span>✓</span>
        <span>{i18n.t('walletCreation.complete.next1')}</span>
      </div>
      <div class="flex items-center gap-2">
        <span>✓</span>
        <span>{i18n.t('walletCreation.complete.next2')}</span>
      </div>
      <div class="flex items-center gap-2">
        <span>✓</span>
        <span>{i18n.t('walletCreation.complete.next3')}</span>
      </div>
    </div>
  </div>

  <!-- Security Reminder -->
  <div class="bg-muted/50 border border-border rounded-lg p-3">
    <h4 class="text-card-foreground font-semibold mb-1 text-sm">{i18n.t('walletCreation.complete.remember')}</h4>
    <div class="text-xs text-muted-foreground space-y-1">
      <p>{i18n.t('walletCreation.complete.reminder1')}</p>
      <p>{i18n.t('walletCreation.complete.reminder2')}</p>
      <p>{i18n.t('walletCreation.complete.reminder3')}</p>
    </div>
  </div>
</div>
