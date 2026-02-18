<script lang="ts">
  import AlertTriangleIcon from '@lucide/svelte/icons/alert-triangle';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import CheckIcon from '@lucide/svelte/icons/check';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import Link2OffIcon from '@lucide/svelte/icons/link-2-off';
  import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
  import { toast } from 'svelte-sonner';
  import { Button } from '$lib/components/ui/button';
  import { i18nStore } from '$lib/i18n';
  import type { IdentityDetails } from '$lib/types/wallet.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';

  const noop = (): void => {};

  type IdentityDetailViewProps = {
    details: IdentityDetails;
    unlinking?: boolean;
    onBack?: () => void;
    onUnlink?: () => void;
  };

  /* eslint-disable prefer-const */
  let { details, unlinking = false, onBack = noop, onUnlink = noop }: IdentityDetailViewProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);

  let copiedKey = $state<string | null>(null);

  const displayName = $derived(formatIdentityDisplayName(details));

  const spendAndSignWarning = $derived(
    !details.ownedByPrimaryAddress || details.primaryAddresses.length > 1
  );
  function equalIgnoreCase(left: string, right: string): boolean {
    return left.toLowerCase() === right.toLowerCase();
  }

  function showValue(value: string | null | undefined): string {
    const normalized = value?.trim();
    if (!normalized) return i18n.t('wallet.identity.detail.notAvailable');
    return normalized;
  }

  async function copyValue(value: string | null | undefined, key: string) {
    if (!value || !value.trim()) return;

    try {
      await globalThis.navigator.clipboard.writeText(value);
      copiedKey = key;
      toast.success(i18n.t('wallet.identity.detail.copySuccess'));
      setTimeout(() => {
        if (copiedKey === key) copiedKey = null;
      }, 1500);
    } catch {
      toast.error(i18n.t('wallet.identity.detail.copyFailed'));
    }
  }

  const revokeAuthorityExternal = $derived(
    details.revocationAuthority
      ? !equalIgnoreCase(details.revocationAuthority, details.identityAddress)
      : false
  );
  const recoveryAuthorityExternal = $derived(
    details.recoveryAuthority
      ? !equalIgnoreCase(details.recoveryAuthority, details.identityAddress)
      : false
  );
</script>

<div class="mx-auto flex w-full max-w-4xl flex-col gap-5 p-6">
  <div class="flex items-center justify-between gap-3">
    <button
      type="button"
      class="text-muted-foreground hover:text-foreground inline-flex items-center gap-1.5 text-sm transition-colors"
      onclick={onBack}
    >
      <ArrowLeftIcon class="size-4" />
      {i18n.t('wallet.identity.detail.back')}
    </button>

    <Button
      variant="destructive"
      size="sm"
      onclick={onUnlink}
      disabled={unlinking}
      class="h-8"
    >
      <Link2OffIcon class="size-4" />
      {unlinking ? i18n.t('wallet.identity.detail.unlinking') : i18n.t('wallet.identity.detail.unlink')}
    </Button>
  </div>

  <div class="rounded-xl bg-muted/20 p-4">
    <p class="text-sm font-semibold text-foreground">{displayName}</p>
    <p class="mt-1 font-mono text-xs text-muted-foreground">{details.identityAddress}</p>
  </div>

  <section class="space-y-2 rounded-xl bg-muted/20 p-4">
    <h3 class="text-sm font-semibold text-foreground">{i18n.t('wallet.identity.detail.sections.base')}</h3>

    <div class="space-y-2 text-sm">
      <div class="flex items-center justify-between gap-3 rounded-md bg-background/55 px-3 py-2 dark:bg-background/40">
        <div class="min-w-0">
          <p class="text-xs text-muted-foreground">{i18n.t('wallet.identity.detail.fields.name')}</p>
          <p class="truncate text-foreground">{showValue(details.name)}</p>
        </div>
      </div>

      <div class="flex items-center justify-between gap-3 rounded-md bg-background/55 px-3 py-2 dark:bg-background/40">
        <div class="min-w-0">
          <p class="text-xs text-muted-foreground">{i18n.t('wallet.identity.detail.fields.iAddress')}</p>
          <p class="truncate font-mono text-foreground">{details.identityAddress}</p>
        </div>
        <button
          type="button"
          class="text-muted-foreground hover:text-foreground inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors"
          onclick={() => copyValue(details.identityAddress, 'iAddress')}
          aria-label={i18n.t('wallet.identity.detail.copy')}
        >
          {#if copiedKey === 'iAddress'}
            <CheckIcon class="size-4 text-emerald-600 dark:text-emerald-300" />
          {:else}
            <CopyIcon class="size-4" />
          {/if}
        </button>
      </div>

      <div class="flex items-center justify-between gap-3 rounded-md bg-background/55 px-3 py-2 dark:bg-background/40">
        <div class="min-w-0">
          <p class="text-xs text-muted-foreground">{i18n.t('wallet.identity.detail.fields.status')}</p>
          <p class="truncate text-foreground">{showValue(details.status)}</p>
        </div>
      </div>

      <div class="flex items-center justify-between gap-3 rounded-md bg-background/55 px-3 py-2 dark:bg-background/40">
        <div class="min-w-0">
          <p class="text-xs text-muted-foreground">{i18n.t('wallet.identity.detail.fields.system')}</p>
          <p class="truncate text-foreground">{showValue(details.system)}</p>
        </div>
      </div>
    </div>
  </section>

  <section class="space-y-2 rounded-xl bg-muted/20 p-4">
    <h3 class="text-sm font-semibold text-foreground">{i18n.t('wallet.identity.detail.sections.authorities')}</h3>

    <div class="space-y-2 text-sm">
      <div class="flex items-center justify-between gap-3 rounded-md bg-background/55 px-3 py-2 dark:bg-background/40">
        <div class="min-w-0">
          <p class="text-xs text-muted-foreground">{i18n.t('wallet.identity.detail.fields.revocationAuthority')}</p>
          <p class="truncate text-foreground">{showValue(details.revocationAuthority)}</p>
        </div>
        {#if details.revocationAuthority}
          <button
            type="button"
            class="text-muted-foreground hover:text-foreground inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors"
            onclick={() => copyValue(details.revocationAuthority, 'revocationAuthority')}
            aria-label={i18n.t('wallet.identity.detail.copy')}
          >
            {#if copiedKey === 'revocationAuthority'}
              <CheckIcon class="size-4 text-emerald-600 dark:text-emerald-300" />
            {:else}
              <CopyIcon class="size-4" />
            {/if}
          </button>
        {/if}
      </div>

      <div class="flex items-center justify-between gap-3 rounded-md bg-background/55 px-3 py-2 dark:bg-background/40">
        <div class="min-w-0">
          <p class="text-xs text-muted-foreground">{i18n.t('wallet.identity.detail.fields.recoveryAuthority')}</p>
          <p class="truncate text-foreground">{showValue(details.recoveryAuthority)}</p>
        </div>
        {#if details.recoveryAuthority}
          <button
            type="button"
            class="text-muted-foreground hover:text-foreground inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors"
            onclick={() => copyValue(details.recoveryAuthority, 'recoveryAuthority')}
            aria-label={i18n.t('wallet.identity.detail.copy')}
          >
            {#if copiedKey === 'recoveryAuthority'}
              <CheckIcon class="size-4 text-emerald-600 dark:text-emerald-300" />
            {:else}
              <CopyIcon class="size-4" />
            {/if}
          </button>
        {/if}
      </div>
    </div>
  </section>

  <section class="space-y-2 rounded-xl bg-muted/20 p-4">
    <h3 class="text-sm font-semibold text-foreground">{i18n.t('wallet.identity.detail.sections.primaryAddresses')}</h3>

    {#if details.primaryAddresses.length === 0}
      <p class="rounded-md bg-background/55 px-3 py-2 text-sm text-muted-foreground dark:bg-background/40">
        {i18n.t('wallet.identity.detail.noPrimaryAddresses')}
      </p>
    {:else}
      <div class="space-y-2">
        {#each details.primaryAddresses as address, index (address)}
          <div class="flex items-center justify-between gap-3 rounded-md bg-background/55 px-3 py-2 dark:bg-background/40">
            <p class="truncate font-mono text-sm text-foreground">{address}</p>
            <button
              type="button"
              class="text-muted-foreground hover:text-foreground inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors"
              onclick={() => copyValue(address, `primary-${index}`)}
              aria-label={i18n.t('wallet.identity.detail.copy')}
            >
              {#if copiedKey === `primary-${index}`}
                <CheckIcon class="size-4 text-emerald-600 dark:text-emerald-300" />
              {:else}
                <CopyIcon class="size-4" />
              {/if}
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  {#if details.privateAddress}
    <section class="space-y-2 rounded-xl bg-muted/20 p-4">
      <h3 class="text-sm font-semibold text-foreground">{i18n.t('wallet.identity.detail.sections.privateAddress')}</h3>
      <div class="flex items-center justify-between gap-3 rounded-md bg-background/55 px-3 py-2 dark:bg-background/40">
        <p class="truncate font-mono text-sm text-foreground">{details.privateAddress}</p>
        <button
          type="button"
          class="text-muted-foreground hover:text-foreground inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors"
          onclick={() => copyValue(details.privateAddress, 'privateAddress')}
          aria-label={i18n.t('wallet.identity.detail.copy')}
        >
          {#if copiedKey === 'privateAddress'}
            <CheckIcon class="size-4 text-emerald-600 dark:text-emerald-300" />
          {:else}
            <CopyIcon class="size-4" />
          {/if}
        </button>
      </div>
    </section>
  {/if}

  <section class="space-y-2 rounded-xl bg-muted/20 p-4">
    <h3 class="text-sm font-semibold text-foreground">{i18n.t('wallet.identity.detail.sections.warnings')}</h3>

    <div class="grid gap-2 md:grid-cols-3">
      <div class={`rounded-md px-3 py-2 text-xs ${spendAndSignWarning ? 'bg-amber-500/10 text-amber-700 dark:text-amber-200' : 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-200'}`}>
        <p class="font-semibold">{i18n.t('wallet.identity.detail.warningCards.spendAndSign.title')}</p>
        <p class="mt-1 leading-relaxed">
          {spendAndSignWarning
            ? i18n.t('wallet.identity.detail.warningCards.spendAndSign.warning')
            : i18n.t('wallet.identity.detail.warningCards.spendAndSign.safe')}
        </p>
      </div>

      <div class={`rounded-md px-3 py-2 text-xs ${revokeAuthorityExternal ? 'bg-amber-500/10 text-amber-700 dark:text-amber-200' : 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-200'}`}>
        <p class="font-semibold">{i18n.t('wallet.identity.detail.warningCards.revoke.title')}</p>
        <p class="mt-1 leading-relaxed">
          {revokeAuthorityExternal
            ? i18n.t('wallet.identity.detail.warningCards.revoke.warning')
            : i18n.t('wallet.identity.detail.warningCards.revoke.safe')}
        </p>
      </div>

      <div class={`rounded-md px-3 py-2 text-xs ${recoveryAuthorityExternal ? 'bg-amber-500/10 text-amber-700 dark:text-amber-200' : 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-200'}`}>
        <p class="font-semibold">{i18n.t('wallet.identity.detail.warningCards.recover.title')}</p>
        <p class="mt-1 leading-relaxed">
          {recoveryAuthorityExternal
            ? i18n.t('wallet.identity.detail.warningCards.recover.warning')
            : i18n.t('wallet.identity.detail.warningCards.recover.safe')}
        </p>
      </div>
    </div>

    {#if details.warnings.length > 0}
      <div class="space-y-1 rounded-md bg-amber-500/10 px-3 py-2 text-xs text-amber-700 dark:text-amber-200">
        {#each details.warnings as warning}
          <p class="flex items-start gap-2 leading-relaxed">
            <AlertTriangleIcon class="mt-0.5 size-3.5 shrink-0" />
            <span>{warning.message}</span>
          </p>
        {/each}
      </div>
    {:else}
      <p class="flex items-center gap-2 rounded-md bg-emerald-500/10 px-3 py-2 text-xs text-emerald-700 dark:text-emerald-200">
        <ShieldCheckIcon class="size-4 shrink-0" />
        <span>{i18n.t('wallet.identity.detail.noWarnings')}</span>
      </p>
    {/if}
  </section>
</div>
