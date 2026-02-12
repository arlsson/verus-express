<script lang="ts">
  import * as Sheet from '$lib/components/ui/sheet';
  import { i18nStore } from '$lib/i18n';
  import type { GuardFlowMode } from '$lib/components/flows/VerusIdGuard/types';

  type VerusIdGuardSheetProps = {
    isOpen?: boolean;
    onSelectMode?: (mode: GuardFlowMode) => void;
  };

  const defaultHandler = (mode: GuardFlowMode) => {
    void mode;
  };

  /* eslint-disable prefer-const */
  let { isOpen = $bindable(false), onSelectMode = defaultHandler }: VerusIdGuardSheetProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);

  function handleSelect(mode: GuardFlowMode) {
    isOpen = false;
    onSelectMode(mode);
  }
</script>

<Sheet.Root bind:open={isOpen}>
  <Sheet.Content side="right" class="w-[420px] max-w-[92vw] p-6">
    {#snippet children()}
      <div class="flex h-full flex-col">
        <Sheet.Header>
          <Sheet.Title>{i18n.t('guard.sheet.title')}</Sheet.Title>
          <Sheet.Description>{i18n.t('guard.sheet.description')}</Sheet.Description>
        </Sheet.Header>

        <div class="mt-5 space-y-3">
          <button
            type="button"
            class="border-input hover:bg-muted/60 w-full rounded-lg border p-4 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            onclick={() => handleSelect('revoke')}
          >
            <div class="flex items-start gap-3">
              <svg
                class="h-4 w-4 mt-0.5 shrink-0 text-muted-foreground"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
              >
                <path d="M12 3l7 4v5c0 5-3.5 8.5-7 9-3.5-.5-7-4-7-9V7l7-4z"></path>
                <path d="M9 9l6 6"></path>
                <path d="M15 9l-6 6"></path>
              </svg>
              <div class="min-w-0">
                <p class="text-sm font-semibold text-foreground">{i18n.t('guard.sheet.revokeTitle')}</p>
                <p class="text-muted-foreground mt-1 text-xs">{i18n.t('guard.sheet.revokeDescription')}</p>
              </div>
            </div>
          </button>

          <button
            type="button"
            class="border-input hover:bg-muted/60 w-full rounded-lg border p-4 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            onclick={() => handleSelect('recover')}
          >
            <div class="flex items-start gap-3">
              <svg
                class="h-4 w-4 mt-0.5 shrink-0 text-muted-foreground"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
              >
                <path d="M12 3l7 4v5c0 5-3.5 8.5-7 9-3.5-.5-7-4-7-9V7l7-4z"></path>
                <path d="M8 12l2.5 2.5L16 9"></path>
              </svg>
              <div class="min-w-0">
                <p class="text-sm font-semibold text-foreground">{i18n.t('guard.sheet.recoverTitle')}</p>
                <p class="text-muted-foreground mt-1 text-xs">{i18n.t('guard.sheet.recoverDescription')}</p>
              </div>
            </div>
          </button>
        </div>
      </div>
    {/snippet}
  </Sheet.Content>
</Sheet.Root>
