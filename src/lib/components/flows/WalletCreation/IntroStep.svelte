<!-- 
  Component: IntroStep
  Purpose: Content-only for introduction step
  Last Updated: Refactored to content-only (no layout)
  Security: No sensitive data - informational only
-->

<script lang="ts">
  import { i18nStore } from '$lib/i18n';

  const i18n = $derived($i18nStore);
</script>

<div class="space-y-5">
  <!-- Process Overview -->
  <div class="p-5 space-y-4 max-w-md">
    <h3 class="text-card-foreground font-semibold text-base">
      {i18n.t('walletCreation.intro.title')}
    </h3>

    <div class="space-y-3 text-left">
      <div class="flex items-start gap-3">
        <div class="w-5 h-5 bg-primary text-primary-foreground rounded-full flex items-center justify-center text-xs font-bold shrink-0 mt-0.5">
          1
        </div>
        <p class="text-sm text-card-foreground">{i18n.t('walletCreation.intro.item1')}</p>
      </div>

      <div class="flex items-start gap-3">
        <div class="w-5 h-5 bg-primary text-primary-foreground rounded-full flex items-center justify-center text-xs font-bold shrink-0 mt-0.5">
          2
        </div>
        <p class="text-sm text-card-foreground">{i18n.t('walletCreation.intro.item2')}</p>
      </div>

      <div class="flex items-start gap-3">
        <div class="w-5 h-5 bg-primary text-primary-foreground rounded-full flex items-center justify-center text-xs font-bold shrink-0 mt-0.5">
          3
        </div>
        <p class="text-sm text-card-foreground">{i18n.t('walletCreation.intro.item3')}</p>
      </div>

      <div class="flex items-start gap-3">
        <div class="w-5 h-5 bg-primary text-primary-foreground rounded-full flex items-center justify-center text-xs font-bold shrink-0 mt-0.5">
          4
        </div>
        <p class="text-sm text-card-foreground">{i18n.t('walletCreation.intro.item4')}</p>
      </div>

      <div class="flex items-start gap-3">
        <div class="w-5 h-5 bg-primary text-primary-foreground rounded-full flex items-center justify-center text-xs font-bold shrink-0 mt-0.5">
          5
        </div>
        <p class="text-sm text-card-foreground">{i18n.t('walletCreation.intro.item5')}</p>
      </div>
    </div>
  </div>
</div>
