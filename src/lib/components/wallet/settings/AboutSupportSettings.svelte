<!--
  Component: AboutSupportSettings
  Purpose: Focused settings detail page for app metadata and support links.
-->

<script lang="ts">
  import { onMount } from 'svelte';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import CommunityHangoutButton from '$lib/components/common/CommunityHangoutButton.svelte';
  import { i18nStore } from '$lib/i18n';
  import { loadRuntimeAppInfo, type RuntimeAppInfo } from '$lib/utils/appInfo.js';

  type AboutSupportSettingsProps = {
    onBack: () => void;
  };

  const { onBack }: AboutSupportSettingsProps = $props();
  const i18n = $derived($i18nStore);

  let appInfo = $state<RuntimeAppInfo | null>(null);
  let loadingAppInfo = $state(true);

  async function loadAppInfo(): Promise<void> {
    loadingAppInfo = true;
    try {
      appInfo = await loadRuntimeAppInfo();
    } finally {
      loadingAppInfo = false;
    }
  }

  onMount(() => {
    void loadAppInfo();
  });
</script>

<div class="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col px-6 pb-6 pt-0 sm:px-8">
  <section class="flex min-h-0 flex-1 flex-col overflow-auto pt-3">
    <div class="space-y-4">
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground inline-flex w-fit items-center gap-1.5 text-sm"
        onclick={onBack}
      >
        <ArrowLeftIcon class="size-4" />
        {i18n.t('common.back')}
      </button>

      <section class="space-y-4">
        <div class="space-y-1">
          <h2 class="text-xl font-semibold">{i18n.t('wallet.settings.about.title')}</h2>
          <p class="text-muted-foreground text-sm">{i18n.t('wallet.settings.about.description')}</p>
        </div>

        <div class="space-y-2 p-1">
          {#if loadingAppInfo || !appInfo}
            <p class="text-muted-foreground text-sm">{i18n.t('common.loading')}</p>
          {:else}
            <dl class="space-y-2 text-sm">
              <div class="flex items-center justify-between gap-4">
                <dt class="text-muted-foreground">{i18n.t('wallet.settings.about.appName')}</dt>
                <dd class="text-right font-medium">{appInfo.name}</dd>
              </div>
              <div class="flex items-center justify-between gap-4">
                <dt class="text-muted-foreground">{i18n.t('wallet.settings.about.version')}</dt>
                <dd class="text-right font-medium">{appInfo.version}</dd>
              </div>
            </dl>
          {/if}
        </div>

        <CommunityHangoutButton label={i18n.t('wallet.settings.about.community')} />
      </section>
    </div>
  </section>
</div>
