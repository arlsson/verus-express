<script lang="ts">
  import { onMount } from 'svelte';
  import CheckCircle2Icon from '@lucide/svelte/icons/check-circle-2';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import InfoIcon from '@lucide/svelte/icons/info';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as Card from '$lib/components/ui/card';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import WalletTransferStepperShell from '$lib/components/shared/WalletTransferStepperShell.svelte';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import { i18nStore } from '$lib/i18n';
  import { resolveCoinPresentation } from '$lib/coins/presentation.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { walletChannelsStore } from '$lib/stores/walletChannels.js';
  import { balanceStore, getBalance } from '$lib/stores/balances.js';
  import { transactionStore } from '$lib/stores/transactions.js';
  import { channelIdForCoin } from '$lib/utils/channelId.js';
  import * as walletService from '$lib/services/walletService.js';
  import { preflightSend, sendTransaction } from '$lib/services/txService.js';
  import { getBridgeConversionPaths, preflightBridgeTransfer } from '$lib/services/bridgeTransferService.js';
  import type {
    BridgeConversionPathQuote,
    BridgeTransferPreflightResult,
    CoinDefinition,
    PreflightResult,
    SendResult
  } from '$lib/types/wallet.js';

  type EntryIntent = 'send' | 'convert';
  type DestinationAddressKind = 'vrpc' | 'btc' | 'eth';

  interface TargetOption {
    id: string;
    kind: 'same' | 'path';
    label: string;
    subtitle?: string;
    destinationId: string;
    convertTo?: string | null;
    exportTo?: string | null;
    via?: string | null;
    mapTo?: string | null;
    price?: string | null;
    gateway?: boolean;
    mapping?: boolean;
    bounceback?: boolean;
    ethDestination?: boolean;
  }

  type TransferWizardProps = {
    entryIntent: EntryIntent;
    onClose?: () => void;
  };

  const defaultClose = () => {};
  const TOTAL_STEPS = 6;

  /* eslint-disable prefer-const */
  let { entryIntent, onClose = defaultClose }: TransferWizardProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);
  const coins = $derived($coinsStore);
  const walletChannels = $derived($walletChannelsStore);
  const balances = $derived($balanceStore);

  const sendableCoins = $derived(
    coins.filter(
      (coin) =>
        coin.compatibleChannels.includes('vrpc') ||
        coin.compatibleChannels.includes('btc') ||
        coin.compatibleChannels.includes('eth') ||
        coin.compatibleChannels.includes('erc20')
    )
  );
  const sendableCoinOptions = $derived(
    sendableCoins.map((coin) => {
      const presentation = resolveCoinPresentation(coin);
      return {
        coin,
        displayName: presentation.displayName,
        displayTicker: presentation.displayTicker
      };
    })
  );

  let selectedCoinId = $state('');
  let selectedTargetOptionId = $state('');
  let currentStep = $state(1);
  let amount = $state('');
  let memo = $state('');
  let destinationAddress = $state('');
  let discoveredPathOptions = $state<TargetOption[]>([]);

  let loadingTargets = $state(false);
  let preflighting = $state(false);
  let sending = $state(false);
  let targetsError = $state('');
  let transferError = $state('');
  let addressesError = $state('');

  let simplePreflightResult = $state<PreflightResult | null>(null);
  let bridgePreflightResult = $state<BridgeTransferPreflightResult | null>(null);
  let sendResult = $state<SendResult | null>(null);
  let addresses = $state<{ vrsc_address: string; eth_address: string; btc_address: string } | null>(
    null
  );

  let showSourceSheet = $state(false);
  let showTargetDetailsSheet = $state(false);

  const selectedCoin = $derived(
    sendableCoins.find((coin) => coin.id === selectedCoinId) ?? sendableCoins[0] ?? null
  );
  const selectedCoinPresentation = $derived(
    selectedCoin ? resolveCoinPresentation(selectedCoin) : null
  );
  const selectedChannelId = $derived(
    selectedCoin
      ? walletChannels.byCoinId[selectedCoin.id] ??
          channelIdForCoin(selectedCoin, walletChannels.vrpcAddress ?? undefined)
      : null
  );
  const selectedChannelPrefix = $derived(selectedChannelId?.split('.')[0] ?? '');
  const selectedBalance = $derived(
    selectedChannelId && selectedCoin
      ? getBalance(selectedChannelId, selectedCoin.id, balances)?.total ?? '0'
      : '0'
  );
  const selectedSourceAddress = $derived(
    !addresses
      ? ''
      : selectedChannelPrefix === 'vrpc'
        ? addresses.vrsc_address
        : selectedChannelPrefix === 'btc'
          ? addresses.btc_address
          : addresses.eth_address
  );

  const sameAssetOption = $derived<TargetOption | null>(
    selectedCoin && selectedCoinPresentation
      ? {
          id: `same-${selectedCoin.id}`,
          kind: 'same',
          label: i18n.t('wallet.transfer.sameAssetOption', {
            ticker: selectedCoinPresentation.displayTicker
          }),
          subtitle: selectedCoinPresentation.displayName,
          destinationId: selectedCoin.id
        }
      : null
  );
  const targetOptions = $derived([
    ...(sameAssetOption ? [sameAssetOption] : []),
    ...discoveredPathOptions
  ]);
  const selectedTargetOption = $derived(
    targetOptions.find((option) => option.id === selectedTargetOptionId) ?? targetOptions[0] ?? null
  );

  const destinationAddressKind = $derived<DestinationAddressKind>(
    selectedTargetOption?.ethDestination || selectedChannelPrefix === 'eth' || selectedChannelPrefix === 'erc20'
      ? 'eth'
      : selectedChannelPrefix === 'btc'
        ? 'btc'
        : 'vrpc'
  );
  const recipientValid = $derived(validateDestinationAddress(destinationAddress, destinationAddressKind));
  const amountValid = $derived(isPositiveAmount(amount));
  const activePreflight = $derived(simplePreflightResult ?? bridgePreflightResult);
  const showEvmConvertUnavailable = $derived(
    entryIntent === 'convert' && (selectedChannelPrefix === 'eth' || selectedChannelPrefix === 'erc20')
  );
  const estimatedConversionValue = $derived((() => {
    if (!amountValid || selectedTargetOption?.kind !== 'path' || !selectedTargetOption.price) return null;
    const numericAmount = Number(amount);
    const numericPrice = Number(selectedTargetOption.price);
    if (!Number.isFinite(numericAmount) || !Number.isFinite(numericPrice)) return null;
    return (numericAmount * numericPrice).toFixed(8);
  })());
  const isBusy = $derived(loadingTargets || preflighting || sending);
  const isDirty = $derived(
    currentStep > 1 ||
      !!amount.trim() ||
      !!memo.trim() ||
      !!destinationAddress.trim() ||
      selectedTargetOption?.kind === 'path' ||
      !!simplePreflightResult ||
      !!bridgePreflightResult ||
      !!sendResult
  );
  const primaryDisabled = $derived(
    isBusy ||
      (currentStep === 1 && (!selectedCoin || !selectedChannelId)) ||
      (currentStep === 2 && !selectedTargetOption) ||
      (currentStep === 3 && !amountValid) ||
      (currentStep === 4 && !recipientValid) ||
      (currentStep === 5 && !activePreflight)
  );
  const primaryLabel = $derived(
    currentStep === 4
      ? preflighting
        ? i18n.t('wallet.transfer.preflighting')
        : i18n.t('wallet.transfer.prepareReview')
      : currentStep === 5
        ? sending
          ? i18n.t('wallet.transfer.sendingNow')
          : i18n.t('wallet.transfer.sendNow')
        : i18n.t('common.continue')
  );

  const viewTitle = $derived(
    entryIntent === 'convert'
      ? i18n.t('wallet.transfer.convertTitle')
      : i18n.t('wallet.transfer.sendTitle')
  );

  $effect(() => {
    if (sendableCoins.length === 0) return;
    if (!selectedCoinId || !sendableCoins.some((coin) => coin.id === selectedCoinId)) {
      selectedCoinId = sendableCoins[0].id;
    }
  });

  $effect(() => {
    targetOptions.length;
    if (targetOptions.length === 0) {
      selectedTargetOptionId = '';
      return;
    }
    if (!targetOptions.some((option) => option.id === selectedTargetOptionId)) {
      const preferred =
        entryIntent === 'convert'
          ? targetOptions.find((option) => option.kind === 'path')
          : targetOptions[0];
      selectedTargetOptionId = preferred ? preferred.id : targetOptions[0].id;
    }
  });

  $effect(() => {
    if (selectedChannelPrefix === 'vrpc') return;
    discoveredPathOptions = [];
    loadingTargets = false;
    targetsError = '';
  });

  $effect(() => {
    const coin = selectedCoin;
    const channelId = selectedChannelId;
    const sourceCurrency = coin?.currencyId || coin?.id || '';
    if (currentStep !== 2 || selectedChannelPrefix !== 'vrpc' || !coin || !channelId || !sourceCurrency) return;

    let cancelled = false;
    loadingTargets = true;
    targetsError = '';
    transferError = '';

    void (async () => {
      try {
        const response = await getBridgeConversionPaths({
          coinId: coin.id,
          channelId,
          sourceCurrency
        });
        if (cancelled) return;
        discoveredPathOptions = buildPathOptions(response.paths);
      } catch (error) {
        if (cancelled) return;
        discoveredPathOptions = [];
        targetsError = mapWalletError(error);
      } finally {
        if (!cancelled) {
          loadingTargets = false;
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  });

  onMount(() => {
    void (async () => {
      try {
        addresses = await walletService.getAddresses();
        addressesError = '';
      } catch {
        addresses = null;
        addressesError = i18n.t('wallet.receive.errorLoad');
      }
    })();
  });

  function buildPathOptions(paths: Record<string, BridgeConversionPathQuote[]>): TargetOption[] {
    const dedupe = new Map<string, TargetOption>();
    const sourceCurrencyId = (selectedCoin?.currencyId || selectedCoin?.id || '').toLowerCase();

    for (const quotes of Object.values(paths)) {
      for (const quote of quotes) {
        const convertTo = quote.convertTo ?? quote.destinationId;
        if (sourceCurrencyId && convertTo.toLowerCase() === sourceCurrencyId) continue;

        const key = `${convertTo ?? ''}|${quote.exportTo ?? ''}|${quote.via ?? ''}|${quote.mapTo ?? ''}|${quote.ethDestination ? 'eth' : 'default'}`;
        if (dedupe.has(key)) continue;

        const label =
          quote.destinationDisplayTicker ?? quote.destinationDisplayName ?? quote.destinationId;

        const subtitleParts: string[] = [];
        if (
          quote.destinationDisplayName &&
          quote.destinationDisplayTicker &&
          quote.destinationDisplayName !== quote.destinationDisplayTicker
        ) {
          subtitleParts.push(quote.destinationDisplayName);
        }
        if (quote.exportTo) {
          subtitleParts.push(i18n.t('wallet.transfer.pathExportTo', { value: quote.exportTo }));
        }
        if (quote.via) {
          subtitleParts.push(i18n.t('wallet.transfer.pathVia', { value: quote.via }));
        }

        dedupe.set(key, {
          id: `path-${dedupe.size + 1}`,
          kind: 'path',
          label,
          subtitle: subtitleParts.join(' • '),
          destinationId: quote.destinationId,
          convertTo,
          exportTo: quote.exportTo,
          via: quote.via,
          mapTo: quote.mapTo,
          price: quote.price,
          gateway: quote.gateway,
          mapping: quote.mapping,
          bounceback: quote.bounceback,
          ethDestination: quote.ethDestination
        });
      }
    }

    return Array.from(dedupe.values()).sort((a, b) => a.label.localeCompare(b.label));
  }

  function isPositiveAmount(input: string): boolean {
    const value = Number(input);
    return Number.isFinite(value) && value > 0;
  }

  function validateDestinationAddress(value: string, kind: DestinationAddressKind): boolean {
    const input = value.trim();
    if (!input) return false;

    if (kind === 'eth') {
      return /^0x[a-fA-F0-9]{40}$/.test(input);
    }
    if (kind === 'btc') {
      return /^(bc1|tb1|[13mn2])[a-zA-HJ-NP-Z0-9]{20,}$/.test(input);
    }
    return /(^[Ri][a-km-zA-HJ-NP-Z1-9]{24,60}$)|(^[A-Za-z0-9._-]+@$)/.test(input);
  }

  function recipientPlaceholder(kind: DestinationAddressKind): string {
    if (kind === 'eth') return i18n.t('wallet.transfer.recipientPlaceholderEth');
    if (kind === 'btc') return i18n.t('wallet.transfer.recipientPlaceholderBtc');
    return i18n.t('wallet.transfer.recipientPlaceholderVrpc');
  }

  function recipientHint(kind: DestinationAddressKind): string {
    if (kind === 'eth') return i18n.t('wallet.transfer.recipientHintEth');
    if (kind === 'btc') return i18n.t('wallet.transfer.recipientHintBtc');
    return i18n.t('wallet.transfer.recipientHintVrpc');
  }

  function extractWalletErrorType(error: unknown): string | null {
    if (!error || typeof error !== 'object') return null;
    const object = error as Record<string, unknown>;

    if (typeof object.type === 'string') return object.type;
    if (object.data && typeof object.data === 'object') {
      const data = object.data as Record<string, unknown>;
      if (typeof data.type === 'string') return data.type;
    }
    return null;
  }

  function mapWalletError(error: unknown): string {
    const errorType = extractWalletErrorType(error);
    if (errorType === 'BridgeNotImplemented') return i18n.t('wallet.transfer.error.bridgeNotImplemented');
    if (errorType === 'UnsupportedChannel') return i18n.t('wallet.transfer.error.unsupportedChannel');
    if (errorType === 'InvalidAddress') return i18n.t('wallet.transfer.error.invalidAddress');
    if (errorType === 'InsufficientFunds') return i18n.t('wallet.transfer.error.insufficientFunds');
    if (errorType === 'NetworkError') return i18n.t('wallet.transfer.error.network');
    if (errorType === 'OperationFailed') return i18n.t('wallet.transfer.error.operationFailed');

    if (error instanceof Error && error.message) return error.message;
    return i18n.t('common.unknownError');
  }

  function goBack() {
    transferError = '';
    if (currentStep === 1) {
      onClose();
      return;
    }
    if (currentStep === 5) {
      simplePreflightResult = null;
      bridgePreflightResult = null;
    }
    currentStep = Math.max(1, currentStep - 1);
  }

  function continueFlow() {
    transferError = '';
    if (currentStep === 1) {
      currentStep = 2;
      return;
    }
    if (currentStep === 2) {
      currentStep = 3;
      return;
    }
    if (currentStep === 3) {
      currentStep = 4;
      return;
    }
    if (currentStep === 4) {
      void runPreflight();
      return;
    }
    if (currentStep === 5) {
      void broadcast();
    }
  }

  async function runPreflight() {
    if (!selectedCoin || !selectedChannelId || !selectedTargetOption) return;

    preflighting = true;
    transferError = '';

    try {
      if (selectedTargetOption.kind === 'path') {
        bridgePreflightResult = await preflightBridgeTransfer({
          coinId: selectedCoin.id,
          channelId: selectedChannelId,
          sourceAddress: selectedSourceAddress || null,
          destination: destinationAddress.trim(),
          amount: amount.trim(),
          convertTo: selectedTargetOption.convertTo ?? null,
          exportTo: selectedTargetOption.exportTo ?? null,
          via: selectedTargetOption.via ?? null,
          mapTo: selectedTargetOption.mapTo ?? null,
          preconvert: null,
          memo: memo.trim() || null
        });
        simplePreflightResult = null;
      } else {
        simplePreflightResult = await preflightSend({
          coinId: selectedCoin.id,
          channelId: selectedChannelId,
          toAddress: destinationAddress.trim(),
          amount: amount.trim(),
          memo: memo.trim() || null
        });
        bridgePreflightResult = null;
      }
      currentStep = 5;
    } catch (error) {
      transferError = mapWalletError(error);
    } finally {
      preflighting = false;
    }
  }

  async function broadcast() {
    if (!activePreflight) return;

    sending = true;
    transferError = '';

    try {
      sendResult = await sendTransaction({ preflightId: activePreflight.preflightId });
      await refreshTxHistory();
      currentStep = 6;
    } catch (error) {
      transferError = mapWalletError(error);
    } finally {
      sending = false;
    }
  }

  async function refreshTxHistory() {
    if (!selectedCoin || !selectedChannelId) return;
    try {
      const transactions = await walletService.getTransactionHistory(selectedChannelId, selectedCoin.id);
      transactionStore.update((state) => ({
        ...state,
        [selectedChannelId]: {
          ...(state[selectedChannelId] ?? {}),
          [selectedCoin.id]: transactions
        }
      }));
    } catch {
      // Best effort refresh only.
    }
  }

  function handleDone() {
    onClose();
  }

  function setMaxAmount() {
    amount = selectedBalance;
  }
</script>

<WalletTransferStepperShell
  currentStep={currentStep}
  totalSteps={TOTAL_STEPS}
  onClose={onClose}
  closeDisabled={isBusy}
  dirty={isDirty}
>
  {#snippet footer()}
    {#if currentStep === 6}
      <div class="flex justify-end">
        <Button onclick={handleDone}>
          {i18n.t('common.done')}
        </Button>
      </div>
    {:else}
      <div class="flex items-center justify-between gap-3">
        <Button variant="outline" onclick={goBack} disabled={isBusy}>
          {currentStep === 1 ? i18n.t('common.cancel') : i18n.t('common.back')}
        </Button>
        <Button onclick={continueFlow} disabled={primaryDisabled}>
          {primaryLabel}
        </Button>
      </div>
    {/if}
  {/snippet}

  <div class="space-y-6">
    <div class="space-y-1">
      <p class="text-lg font-semibold">{viewTitle}</p>
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.subtitle')}</p>
    </div>

    {#if transferError}
      <div class="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
        {transferError}
      </div>
    {/if}

    {#if currentStep === 1}
      <Card.Root>
        <Card.Header>
          <Card.Title>{i18n.t('wallet.transfer.step.source.title')}</Card.Title>
          <Card.Description>{i18n.t('wallet.transfer.step.source.description')}</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-4">
          <div>
            <Label for="transfer-source-coin">{i18n.t('wallet.transfer.sourceAsset')}</Label>
            <select
              id="transfer-source-coin"
              class="mt-2 flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
              bind:value={selectedCoinId}
            >
              {#each sendableCoinOptions as option}
                <option value={option.coin.id}>
                  {option.displayTicker} - {option.displayName}
                </option>
              {/each}
            </select>
          </div>

          {#if selectedCoin && selectedCoinPresentation}
            <div class="rounded-md border border-border/70 px-3 py-2">
              <div class="flex items-center gap-2">
                <CoinIcon
                  coinId={selectedCoin.id}
                  coinName={selectedCoinPresentation.displayName}
                  proto={selectedCoin.proto}
                  size={22}
                  showBadge
                  decorative
                />
                <p class="text-sm font-medium">
                  {selectedCoinPresentation.displayTicker} - {selectedCoinPresentation.displayName}
                </p>
              </div>
              <p class="text-muted-foreground mt-2 text-xs">
                {i18n.t('wallet.transfer.sourceBalance', { value: selectedBalance })}
              </p>
              <p class="text-muted-foreground mt-1 text-xs">
                {i18n.t('wallet.transfer.sourceChannel', { value: selectedChannelId ?? '—' })}
              </p>
            </div>
          {/if}

          <div class="flex justify-end">
            <Button variant="ghost" onclick={() => (showSourceSheet = true)}>
              {i18n.t('wallet.transfer.sourceSheetOpen')}
            </Button>
          </div>

          {#if sendableCoinOptions.length === 0}
            <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.noAssets')}</p>
          {/if}
        </Card.Content>
      </Card.Root>
    {/if}

    {#if currentStep === 2}
      <Card.Root>
        <Card.Header>
          <Card.Title>{i18n.t('wallet.transfer.step.target.title')}</Card.Title>
          <Card.Description>{i18n.t('wallet.transfer.step.target.description')}</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-3">
          {#if loadingTargets}
            <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.loadingTargets')}</p>
          {/if}

          {#if targetsError}
            <p class="text-destructive text-sm">{targetsError}</p>
          {/if}

          {#if showEvmConvertUnavailable}
            <div class="rounded-md border border-border/60 bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
              {i18n.t('wallet.transfer.convertUnavailableEvm')}
            </div>
          {/if}

          {#if targetOptions.length === 0}
            <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.noRoutes')}</p>
          {:else}
            <div class="space-y-2">
              {#each targetOptions as option}
                <button
                  type="button"
                  class="flex w-full items-center justify-between rounded-md border px-3 py-2 text-left transition-colors
                    {selectedTargetOptionId === option.id
                      ? 'border-primary/70 bg-primary/5'
                      : 'border-border/60 hover:bg-muted/40'}"
                  onclick={() => {
                    selectedTargetOptionId = option.id;
                    transferError = '';
                  }}
                >
                  <div class="min-w-0">
                    <p class="truncate text-sm font-medium">{option.label}</p>
                    {#if option.subtitle}
                      <p class="text-muted-foreground truncate text-xs">{option.subtitle}</p>
                    {/if}
                  </div>
                  <div class="ml-3 flex shrink-0 items-center gap-2">
                    {#if option.price}
                      <span class="text-muted-foreground text-xs">~{option.price}</span>
                    {/if}
                    <ChevronRightIcon class="text-muted-foreground size-4" />
                  </div>
                </button>
              {/each}
            </div>
          {/if}

          {#if selectedTargetOption?.kind === 'path'}
            <div class="flex justify-end">
              <Button
                variant="ghost"
                class="gap-2"
                onclick={() => (showTargetDetailsSheet = true)}
              >
                <InfoIcon class="size-4" />
                {i18n.t('wallet.transfer.routeDetails')}
              </Button>
            </div>
          {/if}
        </Card.Content>
      </Card.Root>
    {/if}

    {#if currentStep === 3}
      <Card.Root>
        <Card.Header>
          <Card.Title>{i18n.t('wallet.transfer.step.amount.title')}</Card.Title>
          <Card.Description>{i18n.t('wallet.transfer.step.amount.description')}</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-4">
          <div>
            <Label for="transfer-amount">{i18n.t('wallet.transfer.amountLabel')}</Label>
            <div class="mt-2 flex items-center gap-2">
              <Input
                id="transfer-amount"
                placeholder={i18n.t('wallet.transfer.amountPlaceholder')}
                bind:value={amount}
              />
              <Button variant="outline" onclick={setMaxAmount}>
                {i18n.t('wallet.transfer.max')}
              </Button>
            </div>
            {#if !amountValid && amount.trim()}
              <p class="text-destructive mt-1 text-xs">{i18n.t('wallet.transfer.amountInvalid')}</p>
            {/if}
          </div>

          {#if estimatedConversionValue}
            <p class="text-muted-foreground text-sm">
              {i18n.t('wallet.transfer.estimatedReceive', {
                value: estimatedConversionValue,
                ticker: selectedTargetOption?.label ?? ''
              })}
            </p>
          {/if}

          <div>
            <Label for="transfer-memo">{i18n.t('wallet.send.memoLabel')}</Label>
            <Input
              id="transfer-memo"
              class="mt-2"
              placeholder={i18n.t('wallet.send.memoPlaceholder')}
              bind:value={memo}
            />
          </div>
        </Card.Content>
      </Card.Root>
    {/if}

    {#if currentStep === 4}
      <Card.Root>
        <Card.Header>
          <Card.Title>{i18n.t('wallet.transfer.step.recipient.title')}</Card.Title>
          <Card.Description>{i18n.t('wallet.transfer.step.recipient.description')}</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-3">
          <div>
            <Label for="transfer-recipient">{i18n.t('wallet.transfer.recipientLabel')}</Label>
            <Input
              id="transfer-recipient"
              class="mt-2"
              bind:value={destinationAddress}
              placeholder={recipientPlaceholder(destinationAddressKind)}
            />
            <p class="text-muted-foreground mt-1 text-xs">{recipientHint(destinationAddressKind)}</p>
            {#if destinationAddress.trim() && !recipientValid}
              <p class="text-destructive mt-1 text-xs">{i18n.t('wallet.transfer.recipientInvalid')}</p>
            {/if}
          </div>
        </Card.Content>
      </Card.Root>
    {/if}

    {#if currentStep === 5}
      <Card.Root>
        <Card.Header>
          <Card.Title>{i18n.t('wallet.transfer.step.review.title')}</Card.Title>
          <Card.Description>{i18n.t('wallet.transfer.step.review.description')}</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-3">
          {#if activePreflight}
            <div class="space-y-1 text-sm">
              <p>
                <span class="text-muted-foreground">{i18n.t('wallet.send.to')}</span>
                {activePreflight.toAddress}
              </p>
              <p>
                <span class="text-muted-foreground">{i18n.t('wallet.send.amount')}</span>
                {activePreflight.value}
              </p>
              <p>
                <span class="text-muted-foreground">{i18n.t('wallet.send.fee')}</span>
                {activePreflight.fee} {activePreflight.feeCurrency}
              </p>
            </div>

            {#if bridgePreflightResult}
              <div class="rounded-md border border-border/60 bg-muted/30 px-3 py-2 text-xs">
                <p class="mb-1 font-medium">{i18n.t('wallet.transfer.routeSectionTitle')}</p>
                <div class="space-y-1 text-muted-foreground">
                  {#if bridgePreflightResult.route.convertTo}
                    <p>{i18n.t('wallet.transfer.pathConvertTo', { value: bridgePreflightResult.route.convertTo })}</p>
                  {/if}
                  {#if bridgePreflightResult.route.exportTo}
                    <p>{i18n.t('wallet.transfer.pathExportTo', { value: bridgePreflightResult.route.exportTo })}</p>
                  {/if}
                  {#if bridgePreflightResult.route.via}
                    <p>{i18n.t('wallet.transfer.pathVia', { value: bridgePreflightResult.route.via })}</p>
                  {/if}
                  {#if bridgePreflightResult.route.mapTo}
                    <p>{i18n.t('wallet.transfer.pathMapTo', { value: bridgePreflightResult.route.mapTo })}</p>
                  {/if}
                </div>
              </div>
            {/if}

            <div class="space-y-1">
              <p class="text-xs font-medium">{i18n.t('wallet.transfer.warningsTitle')}</p>
              {#if activePreflight.warnings.length === 0}
                <p class="text-muted-foreground text-xs">{i18n.t('wallet.transfer.noWarnings')}</p>
              {:else}
                {#each activePreflight.warnings as warning}
                  <p class="text-amber-600 dark:text-amber-400 text-xs">{warning.message}</p>
                {/each}
              {/if}
            </div>
          {:else}
            <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.reviewUnavailable')}</p>
          {/if}
        </Card.Content>
      </Card.Root>
    {/if}

    {#if currentStep === 6}
      <Card.Root>
        <Card.Content class="py-8 text-center">
          <CheckCircle2Icon class="mx-auto mb-4 h-12 w-12 text-emerald-600 dark:text-emerald-400" />
          <h3 class="text-lg font-semibold">{i18n.t('wallet.transfer.step.success.title')}</h3>
          <p class="text-muted-foreground mt-1 text-sm">{i18n.t('wallet.transfer.step.success.description')}</p>
          {#if sendResult}
            <p class="mt-3 break-all font-mono text-xs">{sendResult.txid}</p>
            <p class="text-muted-foreground mt-2 text-sm">
              {i18n.t('wallet.send.sentSummary', { value: sendResult.value, address: sendResult.toAddress })}
            </p>
          {/if}
        </Card.Content>
      </Card.Root>
    {/if}
  </div>
</WalletTransferStepperShell>

<StandardRightSheet bind:isOpen={showSourceSheet} title={i18n.t('wallet.transfer.sourceSheetTitle')}>
  <div class="space-y-3">
    <p class="text-sm font-medium">{i18n.t('wallet.transfer.sourceSummaryTitle')}</p>
    <div class="rounded-md border border-border/70 p-3 text-sm">
      <p class="text-muted-foreground text-xs">{i18n.t('wallet.transfer.sourceChannelLabel')}</p>
      <p class="break-all">{selectedChannelId ?? '—'}</p>
    </div>
    <div class="rounded-md border border-border/70 p-3 text-sm">
      <p class="text-muted-foreground text-xs">{i18n.t('wallet.transfer.sourceAddressLabel')}</p>
      <p class="break-all">{selectedSourceAddress || '—'}</p>
    </div>
    {#if addressesError}
      <p class="text-destructive text-xs">{addressesError}</p>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet bind:isOpen={showTargetDetailsSheet} title={i18n.t('wallet.transfer.routeDetailsTitle')}>
  {#if selectedTargetOption?.kind === 'path'}
    <div class="space-y-3 text-sm">
      <div class="rounded-md border border-border/70 p-3">
        <p class="font-medium">{selectedTargetOption.label}</p>
        {#if selectedTargetOption.subtitle}
          <p class="text-muted-foreground mt-1 text-xs">{selectedTargetOption.subtitle}</p>
        {/if}
      </div>
      <div class="space-y-1 text-muted-foreground">
        {#if selectedTargetOption.convertTo}
          <p>{i18n.t('wallet.transfer.pathConvertTo', { value: selectedTargetOption.convertTo })}</p>
        {/if}
        {#if selectedTargetOption.exportTo}
          <p>{i18n.t('wallet.transfer.pathExportTo', { value: selectedTargetOption.exportTo })}</p>
        {/if}
        {#if selectedTargetOption.via}
          <p>{i18n.t('wallet.transfer.pathVia', { value: selectedTargetOption.via })}</p>
        {/if}
        {#if selectedTargetOption.mapTo}
          <p>{i18n.t('wallet.transfer.pathMapTo', { value: selectedTargetOption.mapTo })}</p>
        {/if}
        {#if selectedTargetOption.price}
          <p>{i18n.t('wallet.transfer.pathPrice', { value: selectedTargetOption.price })}</p>
        {/if}
      </div>
    </div>
  {:else}
    <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.noRouteDetails')}</p>
  {/if}
</StandardRightSheet>
