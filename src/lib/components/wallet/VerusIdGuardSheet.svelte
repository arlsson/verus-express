<script lang="ts">
  import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
  import ShieldXIcon from '@lucide/svelte/icons/shield-x';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { GuardFlowMode } from '$lib/components/flows/VerusIdGuard/types';

  const defaultHandler = (mode: GuardFlowMode) => {
    void mode;
  };

  type VerusIdGuardSheetProps = {
    isOpen?: boolean;
    onSelectMode?: typeof defaultHandler;
  };

  /* eslint-disable prefer-const */
  let { isOpen = $bindable(false), onSelectMode = defaultHandler }: VerusIdGuardSheetProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);

  function handleOpenAutoFocus(event: Event) {
    event.preventDefault();
  }

  function handleSelect(mode: GuardFlowMode) {
    isOpen = false;
    onSelectMode(mode);
  }
</script>

<StandardRightSheet
  bind:isOpen
  title={i18n.t('guard.sheet.title')}
  onOpenAutoFocus={handleOpenAutoFocus}
>
  <div class="space-y-3">
    <button
      type="button"
      class="group border-input hover:bg-muted/60 w-full rounded-lg border p-4 text-left transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
      onclick={() => handleSelect('revoke')}
    >
      <div class="flex items-start gap-3">
        <ShieldXIcon
          class="mt-0.5 h-7 w-7 shrink-0 text-foreground opacity-30 transition-[color,opacity] duration-150 group-hover:text-foreground group-hover:opacity-55 dark:opacity-45 dark:group-hover:text-white dark:group-hover:opacity-100"
          absoluteStrokeWidth
          stroke-linecap="butt"
          aria-hidden="true"
        />
        <div class="min-w-0">
          <p class="text-sm font-semibold text-foreground">{i18n.t('guard.sheet.revokeTitle')}</p>
          <p class="text-muted-foreground mt-1 text-xs">{i18n.t('guard.sheet.revokeDescription')}</p>
        </div>
      </div>
    </button>

    <button
      type="button"
      class="group border-input hover:bg-muted/60 w-full rounded-lg border p-4 text-left transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
      onclick={() => handleSelect('recover')}
    >
      <div class="flex items-start gap-3">
        <ShieldCheckIcon
          class="mt-0.5 h-7 w-7 shrink-0 text-foreground opacity-30 transition-[color,opacity] duration-150 group-hover:text-foreground group-hover:opacity-55 dark:opacity-45 dark:group-hover:text-white dark:group-hover:opacity-100"
          absoluteStrokeWidth
          stroke-linecap="butt"
          aria-hidden="true"
        />
        <div class="min-w-0">
          <p class="text-sm font-semibold text-foreground">{i18n.t('guard.sheet.recoverTitle')}</p>
          <p class="text-muted-foreground mt-1 text-xs">{i18n.t('guard.sheet.recoverDescription')}</p>
        </div>
      </div>
    </button>
  </div>
</StandardRightSheet>
