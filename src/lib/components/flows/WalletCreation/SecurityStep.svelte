<!-- 
  Component: SecurityStep
  Purpose: Security guidelines step before showing seed phrase
  Last Updated: Simplified to content only, checkbox moved to main WalletCreation component
  Security: Ensures users read security guidelines before accessing sensitive data
-->

<script lang="ts">
  import { i18nStore } from '$lib/i18n';

  const i18n = $derived($i18nStore);
</script>

<!-- Security Guidelines Content -->
<div class="w-full max-w-md mx-auto">
  <!-- Security Guidelines -->
  <div class="p-6 border-l-4 border-amber-500 bg-amber-50 dark:bg-amber-950/20 rounded-r-lg">
    <h3 class="text-foreground font-semibold text-lg mb-4">{i18n.t('walletCreation.security.title')}</h3>

    <div class="space-y-3 text-foreground text-sm">
      <p>• {i18n.t('walletCreation.security.item1')}</p>
      <p>• {i18n.t('walletCreation.security.item2')}</p>
      <p>• {i18n.t('walletCreation.security.item3')}</p>
      <p>• {i18n.t('walletCreation.security.item4')}</p>
    </div>
  </div>
</div>
