<script lang="ts">
  import { onMount } from 'svelte';
  import CheckCircle2Icon from '@lucide/svelte/icons/check-circle-2';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as Card from '$lib/components/ui/card';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import WalletTransferStepperShell from '$lib/components/shared/WalletTransferStepperShell.svelte';
  import TransferSummaryRail from './transfer-wizard/TransferSummaryRail.svelte';
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
  import {
    getRecipientInputCopy,
    getTransferStepCopy,
    getTransferStepLabels
  } from '$lib/transfer/transferWizardCopy';
  import type {
    BridgeConversionPathQuote,
    BridgeTransferPreflightResult,
    PreflightResult,
    SendResult
  } from '$lib/types/wallet.js';
  import type {
    DestinationAddressKind,
    TransferStepId,
    TransferStepperStep,
    WizardOperationalStepId
  } from './transfer-wizard/types';

  type EntryIntent = 'send' | 'convert';

  interface ViaRouteOption {
    id: string;
    kind: 'same' | 'path';
    receiveKey: string;
    receiveLabel: string;
    receiveSubtitle?: string;
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

  interface ReceiveAssetOption {
    key: string;
    label: string;
    subtitle?: string;
    destinationId: string;
    viaOptions: ViaRouteOption[];
  }

  type TransferWizardProps = {
    entryIntent: EntryIntent;
    onClose?: () => void;
  };

  const defaultClose = () => {};
  const OPERATIONAL_STEPS: WizardOperationalStepId[] = ['details', 'recipient', 'review'];

  /* eslint-disable prefer-const */
  let { entryIntent, onClose = defaultClose }: TransferWizardProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);
  const coins = $derived($coinsStore);
  const walletChannels = $derived($walletChannelsStore);
  const balances = $derived($balanceStore);
  const stepCopy = $derived(getTransferStepCopy(i18n.t));
  const stepLabels = $derived(getTransferStepLabels(i18n.t));

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
  let currentStep = $state<TransferStepId>('details');
  let amount = $state('');
  let destinationAddress = $state('');
  let conversionEnabled = $state(false);
  let conversionInitialized = $state(false);
  let selectedReceiveAssetKey = $state('');
  let selectedViaOptionId = $state('');
  let manualViaLocked = $state(false);
  let discoveredPathOptions = $state<ViaRouteOption[]>([]);

  let loadingTargets = $state(false);
  let preflighting = $state(false);
  let sending = $state(false);
  let targetsError = $state('');
  let transferError = $state('');
  let addressesError = $state('');

  let simplePreflightResult = $state<PreflightResult | null>(null);
  let bridgePreflightResult = $state<BridgeTransferPreflightResult | null>(null);
  let sendResult = $state<SendResult | null>(null);
  let addresses = $state<{ vrsc_address: string; eth_address: string; btc_address: string } | null>(null);

  let showSourceAssetSheet = $state(false);
  let showReceiveAssetSheet = $state(false);
  let showViaSheet = $state(false);

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
  const sourceSupportsConversion = $derived(selectedChannelPrefix === 'vrpc');

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

  const sameAssetOption = $derived<ViaRouteOption | null>(
    selectedCoin && selectedCoinPresentation
      ? {
          id: `same-${selectedCoin.id}`,
          kind: 'same',
          receiveKey: (selectedCoin.currencyId || selectedCoin.id).toLowerCase(),
          receiveLabel: selectedCoinPresentation.displayTicker,
          receiveSubtitle:
            selectedCoinPresentation.displayName !== selectedCoinPresentation.displayTicker
              ? selectedCoinPresentation.displayName
              : undefined,
          label: i18n.t('wallet.transfer.sameAssetOption', {
            ticker: selectedCoinPresentation.displayTicker
          }),
          destinationId: selectedCoin.id
        }
      : null
  );

  const receiveAssetOptions = $derived<ReceiveAssetOption[]>(
    (() => {
      const grouped = new Map<string, ReceiveAssetOption>();

      for (const option of discoveredPathOptions) {
        const existing = grouped.get(option.receiveKey);
        if (existing) {
          existing.viaOptions.push(option);
          continue;
        }

        grouped.set(option.receiveKey, {
          key: option.receiveKey,
          label: option.receiveLabel,
          subtitle: option.receiveSubtitle,
          destinationId: option.destinationId,
          viaOptions: [option]
        });
      }

      const groups = Array.from(grouped.values());
      for (const group of groups) {
        group.viaOptions.sort((a, b) => viaLexicalKey(a).localeCompare(viaLexicalKey(b)));
      }

      return groups.sort((a, b) => a.label.localeCompare(b.label));
    })()
  );

  const selectedReceiveAssetOption = $derived(
    receiveAssetOptions.find((option) => option.key === selectedReceiveAssetKey) ??
      receiveAssetOptions[0] ??
      null
  );

  const rankedViaOptions = $derived(
    selectedReceiveAssetOption
      ? sortViaOptionsByScore(selectedReceiveAssetOption.viaOptions, amount)
      : []
  );

  const bestViaOption = $derived(rankedViaOptions[0] ?? null);

  const selectedViaOption = $derived(
    selectedReceiveAssetOption?.viaOptions.find((option) => option.id === selectedViaOptionId) ?? null
  );

  const activeConvertRoute = $derived(selectedViaOption ?? bestViaOption);
  const activeTargetOption = $derived(
    conversionEnabled ? activeConvertRoute : sameAssetOption
  );

  const convertUnavailableForSource = $derived(
    selectedChannelPrefix === 'eth' || selectedChannelPrefix === 'erc20'
  );

  const showEvmConvertUnavailable = $derived(
    convertUnavailableForSource && (entryIntent === 'convert' || conversionEnabled)
  );

  const destinationAddressKind = $derived<DestinationAddressKind>(
    activeTargetOption?.ethDestination ||
      selectedChannelPrefix === 'eth' ||
      selectedChannelPrefix === 'erc20'
      ? 'eth'
      : selectedChannelPrefix === 'btc'
        ? 'btc'
        : 'vrpc'
  );

  const recipientInputCopy = $derived(getRecipientInputCopy(i18n.t, destinationAddressKind));
  const recipientValid = $derived(validateDestinationAddress(destinationAddress, destinationAddressKind));
  const amountValid = $derived(isPositiveAmount(amount));
  const activePreflight = $derived(simplePreflightResult ?? bridgePreflightResult);

  const estimatedConversionValue = $derived(
    (() => {
      if (!conversionEnabled || !amountValid || !activeConvertRoute?.price) return null;
      const numericAmount = Number(amount);
      const numericPrice = Number(activeConvertRoute.price);
      if (!Number.isFinite(numericAmount) || !Number.isFinite(numericPrice)) return null;
      return (numericAmount * numericPrice).toFixed(8);
    })()
  );

  const stepNumber = $derived(
    currentStep === 'success'
      ? OPERATIONAL_STEPS.length
      : OPERATIONAL_STEPS.indexOf(currentStep as WizardOperationalStepId) + 1
  );

  const stepperSteps = $derived<TransferStepperStep[]>(
    OPERATIONAL_STEPS.map((stepId, index) => {
      const currentIndex =
        currentStep === 'success'
          ? OPERATIONAL_STEPS.length - 1
          : OPERATIONAL_STEPS.indexOf(currentStep as WizardOperationalStepId);

      return {
        id: stepId,
        label: stepLabels[stepId],
        status:
          currentStep === 'success' || index < currentIndex
            ? 'complete'
            : index === currentIndex
              ? 'current'
              : 'upcoming'
      };
    })
  );

  const isBusy = $derived(loadingTargets || preflighting || sending);
  const isDirty = $derived(
    currentStep !== 'details' ||
      !!amount.trim() ||
      !!destinationAddress.trim() ||
      conversionEnabled !== (entryIntent === 'convert') ||
      !!simplePreflightResult ||
      !!bridgePreflightResult ||
      !!sendResult
  );

  const primaryDisabled = $derived(
    isBusy ||
      (currentStep === 'details' &&
        (!selectedCoin ||
          !selectedChannelId ||
          !amountValid ||
          !activeTargetOption ||
          (conversionEnabled && !selectedReceiveAssetOption))) ||
      (currentStep === 'recipient' && !recipientValid) ||
      (currentStep === 'review' && !activePreflight)
  );

  const primaryLabel = $derived(
    currentStep === 'recipient'
      ? preflighting
        ? i18n.t('wallet.transfer.preflighting')
        : i18n.t('wallet.transfer.prepareReview')
      : currentStep === 'review'
        ? sending
          ? i18n.t('wallet.transfer.sendingNow')
          : i18n.t('wallet.transfer.sendNow')
        : i18n.t('common.continue')
  );

  const viewTitle = $derived(
    entryIntent === 'convert' ? i18n.t('wallet.transfer.convertTitle') : i18n.t('wallet.transfer.sendTitle')
  );

  const sourceSummaryValue = $derived(
    selectedCoinPresentation
      ? `${selectedCoinPresentation.displayTicker} - ${selectedCoinPresentation.displayName}`
      : ''
  );

  const toSummaryValue = $derived(
    conversionEnabled
      ? selectedReceiveAssetOption?.label ?? ''
      : selectedCoinPresentation
        ? `${selectedCoinPresentation.displayTicker} - ${selectedCoinPresentation.displayName}`
        : ''
  );

  const routeSummaryValue = $derived(
    (() => {
      if (!activeTargetOption) return '';
      if (activeTargetOption.kind === 'same') return activeTargetOption.label;

      const parts = [
        activeTargetOption.receiveLabel,
        activeTargetOption.via ? i18n.t('wallet.transfer.pathVia', { value: activeTargetOption.via }) : '',
        activeTargetOption.exportTo
          ? i18n.t('wallet.transfer.pathExportTo', { value: activeTargetOption.exportTo })
          : ''
      ].filter((value) => !!value);

      return parts.join(' • ');
    })()
  );

  const amountSummaryValue = $derived(
    amountValid && selectedCoinPresentation
      ? `${amount.trim()} ${selectedCoinPresentation.displayTicker}`
      : ''
  );

  const recipientSummaryValue = $derived(destinationAddress.trim());

  const estimatedReceiveSummaryValue = $derived(
    conversionEnabled
      ? estimatedConversionValue
        ? `${estimatedConversionValue} ${selectedReceiveAssetOption?.label ?? ''}`
        : ''
      : amountValid && selectedCoinPresentation
        ? `${amount.trim()} ${selectedCoinPresentation.displayTicker}`
        : ''
  );

  const networkFeeSummaryValue = $derived(
    activePreflight ? `${activePreflight.fee} ${activePreflight.feeCurrency}` : ''
  );

  const warningsSummary = $derived(activePreflight?.warnings.map((warning) => warning.message) ?? []);

  const preflightInputSignature = $derived(
    [
      selectedCoinId,
      selectedChannelId ?? '',
      conversionEnabled ? '1' : '0',
      selectedReceiveAssetKey,
      activeTargetOption?.id ?? '',
      amount.trim(),
      destinationAddress.trim()
    ].join('|')
  );

  let previousPreflightInputSignature = $state<string | null>(null);

  $effect(() => {
    if (conversionInitialized) return;
    conversionEnabled = entryIntent === 'convert';
    conversionInitialized = true;
  });

  $effect(() => {
    if (sendableCoins.length === 0) return;
    if (!selectedCoinId || !sendableCoins.some((coin) => coin.id === selectedCoinId)) {
      selectedCoinId = sendableCoins[0].id;
    }
  });

  $effect(() => {
    if (!sourceSupportsConversion) {
      discoveredPathOptions = [];
      loadingTargets = false;
      targetsError = '';
      selectedReceiveAssetKey = '';
      selectedViaOptionId = '';
      manualViaLocked = false;
    }
  });

  $effect(() => {
    if (showEvmConvertUnavailable && conversionEnabled) {
      conversionEnabled = false;
      manualViaLocked = false;
      selectedViaOptionId = '';
    }
  });

  $effect(() => {
    if (receiveAssetOptions.length === 0) {
      selectedReceiveAssetKey = '';
      selectedViaOptionId = '';
      manualViaLocked = false;
      return;
    }

    if (!receiveAssetOptions.some((option) => option.key === selectedReceiveAssetKey)) {
      selectedReceiveAssetKey = receiveAssetOptions[0].key;
      manualViaLocked = false;
    }
  });

  $effect(() => {
    if (!conversionEnabled) return;
    if (!selectedReceiveAssetOption) return;

    const selectedStillValid = selectedReceiveAssetOption.viaOptions.some(
      (option) => option.id === selectedViaOptionId
    );

    if (manualViaLocked && selectedStillValid) return;

    const best = sortViaOptionsByScore(selectedReceiveAssetOption.viaOptions, amount)[0] ?? null;
    if (!best) {
      selectedViaOptionId = '';
      manualViaLocked = false;
      return;
    }

    if (selectedViaOptionId !== best.id) {
      selectedViaOptionId = best.id;
    }

    if (manualViaLocked && !selectedStillValid) {
      manualViaLocked = false;
    }
  });

  $effect(() => {
    const signature = preflightInputSignature;
    if (previousPreflightInputSignature === null) {
      previousPreflightInputSignature = signature;
      return;
    }

    if (signature === previousPreflightInputSignature) return;

    previousPreflightInputSignature = signature;
    clearPreflightState();
  });

  $effect(() => {
    const coin = selectedCoin;
    const channelId = selectedChannelId;
    const sourceCurrency = coin?.currencyId || coin?.id || '';
    let cancelled = false;

    if (
      currentStep !== 'details' ||
      !sourceSupportsConversion ||
      !coin ||
      !channelId ||
      !sourceCurrency
    ) {
      return () => {
        cancelled = true;
      };
    }

    loadingTargets = true;
    targetsError = '';

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
        if (!cancelled) loadingTargets = false;
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

  function clearPreflightState() {
    simplePreflightResult = null;
    bridgePreflightResult = null;
    transferError = '';
  }

  function buildPathOptions(paths: Record<string, BridgeConversionPathQuote[]>): ViaRouteOption[] {
    const dedupe = new Map<string, ViaRouteOption>();
    const sourceCurrencyId = (selectedCoin?.currencyId || selectedCoin?.id || '').toLowerCase();

    for (const quotes of Object.values(paths)) {
      for (const quote of quotes) {
        const convertTo = quote.convertTo ?? quote.destinationId;
        if (sourceCurrencyId && convertTo.toLowerCase() === sourceCurrencyId) continue;

        const receiveKey = (convertTo ?? quote.destinationId).toLowerCase();
        const dedupeKey = `${receiveKey}|${quote.exportTo ?? ''}|${quote.via ?? ''}|${quote.mapTo ?? ''}|${quote.ethDestination ? 'eth' : 'default'}`;
        if (dedupe.has(dedupeKey)) continue;

        const receiveLabel =
          quote.destinationDisplayTicker ?? quote.destinationDisplayName ?? quote.destinationId;

        const receiveSubtitle =
          quote.destinationDisplayName && quote.destinationDisplayName !== receiveLabel
            ? quote.destinationDisplayName
            : undefined;

        const subtitleParts: string[] = [];
        if (quote.exportTo) {
          subtitleParts.push(i18n.t('wallet.transfer.pathExportTo', { value: quote.exportTo }));
        }
        if (quote.via) {
          subtitleParts.push(i18n.t('wallet.transfer.pathVia', { value: quote.via }));
        }
        if (quote.mapTo) {
          subtitleParts.push(i18n.t('wallet.transfer.pathMapTo', { value: quote.mapTo }));
        }

        dedupe.set(dedupeKey, {
          id: `path-${dedupe.size + 1}`,
          kind: 'path',
          receiveKey,
          receiveLabel,
          receiveSubtitle,
          label: receiveLabel,
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

    return Array.from(dedupe.values());
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

  function viaLexicalKey(option: ViaRouteOption): string {
    return `${option.via ?? ''}|${option.exportTo ?? ''}|${option.mapTo ?? ''}`.toLowerCase();
  }

  function parsePrice(value?: string | null): number | null {
    if (!value) return null;
    const parsed = Number(value);
    if (!Number.isFinite(parsed) || parsed <= 0) return null;
    return parsed;
  }

  function routeScore(option: ViaRouteOption, amountInput: string): number | null {
    const price = parsePrice(option.price);
    if (price === null) return null;

    const numericAmount = Number(amountInput);
    if (Number.isFinite(numericAmount) && numericAmount > 0) {
      return price * numericAmount;
    }

    return price;
  }

  function sortViaOptionsByScore(options: ViaRouteOption[], amountInput: string): ViaRouteOption[] {
    return [...options].sort((a, b) => {
      const scoreA = routeScore(a, amountInput);
      const scoreB = routeScore(b, amountInput);

      if (scoreA !== null && scoreB !== null && scoreA !== scoreB) {
        return scoreB - scoreA;
      }

      if (scoreA !== null && scoreB === null) return -1;
      if (scoreA === null && scoreB !== null) return 1;

      return viaLexicalKey(a).localeCompare(viaLexicalKey(b));
    });
  }

  function formatEstimatedReceive(option: ViaRouteOption): string {
    if (!amountValid) return i18n.t('wallet.transfer.summary.notSet');
    const price = parsePrice(option.price);
    if (price === null) return i18n.t('wallet.transfer.summary.notSet');
    return `${(Number(amount) * price).toFixed(8)} ${option.receiveLabel}`;
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

    if (currentStep === 'details') {
      onClose();
      return;
    }

    if (currentStep === 'recipient') {
      currentStep = 'details';
      return;
    }

    if (currentStep === 'review') {
      clearPreflightState();
      currentStep = 'recipient';
    }
  }

  function continueFlow() {
    transferError = '';

    if (currentStep === 'details') {
      currentStep = 'recipient';
      return;
    }

    if (currentStep === 'recipient') {
      void runPreflight();
      return;
    }

    if (currentStep === 'review') {
      void broadcast();
    }
  }

  function jumpToStep(step: WizardOperationalStepId) {
    clearPreflightState();
    currentStep = step;
  }

  async function runPreflight() {
    if (!selectedCoin || !selectedChannelId || !activeTargetOption) return;

    preflighting = true;
    transferError = '';

    try {
      if (activeTargetOption.kind === 'path') {
        bridgePreflightResult = await preflightBridgeTransfer({
          coinId: selectedCoin.id,
          channelId: selectedChannelId,
          sourceAddress: selectedSourceAddress || null,
          destination: destinationAddress.trim(),
          amount: amount.trim(),
          convertTo: activeTargetOption.convertTo ?? null,
          exportTo: activeTargetOption.exportTo ?? null,
          via: activeTargetOption.via ?? null,
          mapTo: activeTargetOption.mapTo ?? null,
          preconvert: null
        });
        simplePreflightResult = null;
      } else {
        simplePreflightResult = await preflightSend({
          coinId: selectedCoin.id,
          channelId: selectedChannelId,
          toAddress: destinationAddress.trim(),
          amount: amount.trim()
        });
        bridgePreflightResult = null;
      }
      currentStep = 'review';
    } catch (error) {
      transferError = mapWalletError(error);
      currentStep = 'recipient';
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
      currentStep = 'success';
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

  function handleEnableConversion() {
    if (!sourceSupportsConversion || convertUnavailableForSource) return;
    conversionEnabled = true;
    manualViaLocked = false;
  }

  function handleDisableConversion() {
    conversionEnabled = false;
    manualViaLocked = false;
    selectedViaOptionId = '';
  }

  function selectSourceCoin(coinId: string) {
    selectedCoinId = coinId;
    showSourceAssetSheet = false;
    transferError = '';
  }

  function selectReceiveAsset(key: string) {
    selectedReceiveAssetKey = key;
    manualViaLocked = false;

    const selected = receiveAssetOptions.find((option) => option.key === key);
    const best = selected ? sortViaOptionsByScore(selected.viaOptions, amount)[0] ?? null : null;
    selectedViaOptionId = best?.id ?? '';

    showReceiveAssetSheet = false;
    transferError = '';
  }

  function selectViaOption(viaOptionId: string) {
    selectedViaOptionId = viaOptionId;
    manualViaLocked = true;
    showViaSheet = false;
    transferError = '';
  }

  function resetViaToBest() {
    if (!bestViaOption) return;
    selectedViaOptionId = bestViaOption.id;
    manualViaLocked = false;
  }
</script>

<WalletTransferStepperShell
  currentStep={stepNumber}
  totalSteps={OPERATIONAL_STEPS.length}
  steps={stepperSteps}
  onClose={onClose}
  closeDisabled={isBusy}
  dirty={isDirty}
  mobileAsideLabel={i18n.t('wallet.transfer.viewSummary')}
  mobileAsideTitle={i18n.t('wallet.transfer.summary.title')}
>
  {#snippet aside()}
    <TransferSummaryRail
      sourceValue={sourceSummaryValue}
      toValue={toSummaryValue}
      routeValue={routeSummaryValue}
      amountValue={amountSummaryValue}
      recipientValue={recipientSummaryValue}
      estimatedReceiveValue={estimatedReceiveSummaryValue}
      networkFeeValue={networkFeeSummaryValue}
      warnings={warningsSummary}
      class="h-full"
    />
  {/snippet}

  {#snippet footer()}
    {#if currentStep === 'success'}
      <div class="flex justify-end">
        <Button onclick={handleDone}>
          {i18n.t('common.done')}
        </Button>
      </div>
    {:else}
      <div class="flex items-center justify-between gap-3">
        <Button variant="secondary" onclick={goBack} disabled={isBusy}>
          {currentStep === 'details' ? i18n.t('common.cancel') : i18n.t('common.back')}
        </Button>
        <Button onclick={continueFlow} disabled={primaryDisabled}>
          {primaryLabel}
        </Button>
      </div>
    {/if}
  {/snippet}

  <div class="space-y-5">
    <div>
      <p class="text-lg font-semibold">{viewTitle}</p>
    </div>

    {#if transferError}
      <div class="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
        {transferError}
      </div>
    {/if}

    {#if currentStep === 'details'}
      <Card.Root class="border-0 bg-transparent py-0 shadow-none">
        <Card.Header class="px-0">
          <Card.Title>{stepCopy.details.title}</Card.Title>
        </Card.Header>
        <Card.Content class="space-y-4 px-0">
          <div class="space-y-3 rounded-lg border border-border/60 p-4">
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium">{i18n.t('wallet.transfer.youSend')}</p>
              <Button variant="outline" size="sm" onclick={() => (showSourceAssetSheet = true)}>
                {sourceSummaryValue || i18n.t('wallet.transfer.sourceAsset')}
              </Button>
            </div>

            <div class="space-y-2">
              <Label for="transfer-amount">{i18n.t('wallet.transfer.amountLabel')}</Label>
              <div class="flex items-center gap-2">
                <Input
                  id="transfer-amount"
                  placeholder={i18n.t('wallet.transfer.amountPlaceholder')}
                  bind:value={amount}
                />
                <Button variant="secondary" onclick={setMaxAmount}>
                  {i18n.t('wallet.transfer.max')}
                </Button>
              </div>
            </div>

            {#if !amountValid && amount.trim()}
              <p class="text-destructive text-xs">{i18n.t('wallet.transfer.amountInvalid')}</p>
            {/if}

            <p class="text-muted-foreground text-xs">
              {i18n.t('wallet.transfer.sourceBalance', { value: selectedBalance })}
            </p>
          </div>

          {#if showEvmConvertUnavailable}
            <div class="rounded-md border border-border/60 px-3 py-2 text-sm text-muted-foreground">
              {i18n.t('wallet.transfer.convertUnavailableEvm')}
            </div>
          {/if}

          {#if !conversionEnabled}
            {#if sourceSupportsConversion && !convertUnavailableForSource}
              <div>
                <Button variant="ghost" size="sm" onclick={handleEnableConversion}>
                  {i18n.t('wallet.transfer.doConversion')}
                </Button>
              </div>
            {/if}
          {:else}
            <div class="space-y-3 rounded-lg border border-border/60 p-4">
              <div class="flex items-center justify-between gap-3">
                <p class="text-sm font-medium">{i18n.t('wallet.transfer.youReceive')}</p>
                <Button variant="outline" size="sm" onclick={() => (showReceiveAssetSheet = true)}>
                  {selectedReceiveAssetOption?.label || i18n.t('wallet.transfer.receiveAsset')}
                </Button>
              </div>

              <p class="text-base font-medium">
                {estimatedReceiveSummaryValue || i18n.t('wallet.transfer.summary.notSet')}
              </p>

              <div class="flex flex-wrap items-center gap-2">
                <Button variant="ghost" size="sm" onclick={() => (showViaSheet = true)}>
                  {i18n.t('wallet.transfer.via')}: {activeConvertRoute?.via || i18n.t('wallet.transfer.viaBest')}
                </Button>
                {#if manualViaLocked}
                  <Button variant="ghost" size="sm" onclick={resetViaToBest}>
                    {i18n.t('wallet.transfer.viaResetBest')}
                  </Button>
                {/if}
                <Button variant="ghost" size="sm" onclick={handleDisableConversion}>
                  {i18n.t('wallet.transfer.routeGroupSend')}
                </Button>
              </div>

              {#if targetsError}
                <p class="text-destructive text-xs">{targetsError}</p>
              {/if}
            </div>
          {/if}

          {#if loadingTargets && sourceSupportsConversion}
            <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.loadingTargets')}</p>
          {/if}

          {#if !loadingTargets && conversionEnabled && receiveAssetOptions.length === 0 && sourceSupportsConversion}
            <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.noRoutes')}</p>
          {/if}

          {#if sendableCoinOptions.length === 0}
            <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.noAssets')}</p>
          {/if}
        </Card.Content>
      </Card.Root>
    {/if}

    {#if currentStep === 'recipient'}
      <Card.Root class="border-0 bg-transparent py-0 shadow-none">
        <Card.Header class="px-0">
          <Card.Title>{stepCopy.recipient.title}</Card.Title>
        </Card.Header>
        <Card.Content class="space-y-3 px-0">
          <div>
            <Label for="transfer-recipient">{i18n.t('wallet.transfer.recipientLabel')}</Label>
            <Input
              id="transfer-recipient"
              class="mt-2"
              bind:value={destinationAddress}
              placeholder={recipientInputCopy.placeholder}
            />
            <p class="text-muted-foreground mt-1 text-xs">{recipientInputCopy.hint}</p>
            {#if destinationAddress.trim() && !recipientValid}
              <p class="text-destructive mt-1 text-xs">{i18n.t('wallet.transfer.recipientInvalid')}</p>
            {/if}
          </div>
        </Card.Content>
      </Card.Root>
    {/if}

    {#if currentStep === 'review'}
      <Card.Root class="border-0 bg-transparent py-0 shadow-none">
        <Card.Header class="px-0">
          <Card.Title>{stepCopy.review.title}</Card.Title>
        </Card.Header>
        <Card.Content class="space-y-3 px-0">
          {#if activePreflight}
            <div class="space-y-2 rounded-md border border-border/70 p-3">
              <div class="flex items-center justify-between gap-3">
                <p class="text-sm font-medium">{stepCopy.details.title}</p>
                <Button variant="ghost" size="sm" onclick={() => jumpToStep('details')}>
                  {i18n.t('wallet.transfer.review.changeDetails')}
                </Button>
              </div>
              <p class="text-sm">{sourceSummaryValue || i18n.t('wallet.transfer.summary.notSet')}</p>
              <p class="text-sm">{toSummaryValue || i18n.t('wallet.transfer.summary.notSet')}</p>
              <p class="text-sm">{amountSummaryValue || i18n.t('wallet.transfer.summary.notSet')}</p>
              {#if conversionEnabled}
                <p class="text-muted-foreground text-xs">{routeSummaryValue || i18n.t('wallet.transfer.summary.notSet')}</p>
              {/if}
            </div>

            <div class="space-y-2 rounded-md border border-border/70 p-3">
              <div class="flex items-center justify-between gap-3">
                <p class="text-sm font-medium">{i18n.t('wallet.transfer.summary.recipient')}</p>
                <Button variant="ghost" size="sm" onclick={() => jumpToStep('recipient')}>
                  {i18n.t('wallet.transfer.review.changeRecipient')}
                </Button>
              </div>
              <p class="break-all text-sm">{activePreflight.toAddress}</p>
            </div>

            <div class="space-y-1 rounded-md border border-border/70 p-3">
              <p class="text-sm font-medium">{i18n.t('wallet.transfer.summary.networkFee')}</p>
              <p class="text-sm">{activePreflight.fee} {activePreflight.feeCurrency}</p>
              {#if bridgePreflightResult}
                <div class="text-muted-foreground mt-2 space-y-1 text-xs">
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
              {/if}
            </div>

            {#if activePreflight.warnings.length > 0}
              <div class="space-y-1 rounded-md border border-border/70 p-3">
                <p class="text-sm font-medium">{i18n.t('wallet.transfer.warningsTitle')}</p>
                {#each activePreflight.warnings as warning}
                  <p class="text-amber-600 dark:text-amber-400 text-xs">{warning.message}</p>
                {/each}
              </div>
            {/if}
          {:else}
            <div class="space-y-3">
              <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.reviewUnavailable')}</p>
              <Button variant="outline" onclick={runPreflight} disabled={preflighting || !recipientValid}>
                {i18n.t('wallet.transfer.review.refresh')}
              </Button>
            </div>
          {/if}
        </Card.Content>
      </Card.Root>
    {/if}

    {#if currentStep === 'success'}
      <Card.Root class="border-0 bg-transparent py-0 shadow-none">
        <Card.Content class="px-0 py-6 text-center">
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

<StandardRightSheet bind:isOpen={showSourceAssetSheet} title={i18n.t('wallet.transfer.sourceSheetTitle')}>
  <div class="space-y-3">
    {#if sendableCoinOptions.length === 0}
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.noAssets')}</p>
    {:else}
      <div class="space-y-2">
        {#each sendableCoinOptions as option}
          <button
            type="button"
            class="flex w-full items-center justify-between rounded-md border px-3 py-2 text-left transition-colors
              {selectedCoinId === option.coin.id
                ? 'border-primary/70 bg-primary/5'
                : 'border-border/60 hover:bg-muted/30'}"
            onclick={() => selectSourceCoin(option.coin.id)}
          >
            <div class="min-w-0">
              <p class="truncate text-sm font-medium">{option.displayTicker} - {option.displayName}</p>
            </div>
          </button>
        {/each}
      </div>
    {/if}

    <div class="rounded-md border border-border/60 p-3 text-sm">
      <p class="text-muted-foreground text-xs">{i18n.t('wallet.transfer.sourceChannelLabel')}</p>
      <p class="break-all">{selectedChannelId ?? '—'}</p>
    </div>
    <div class="rounded-md border border-border/60 p-3 text-sm">
      <p class="text-muted-foreground text-xs">{i18n.t('wallet.transfer.sourceAddressLabel')}</p>
      <p class="break-all">{selectedSourceAddress || '—'}</p>
    </div>
    {#if addressesError}
      <p class="text-destructive text-xs">{addressesError}</p>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet bind:isOpen={showReceiveAssetSheet} title={i18n.t('wallet.transfer.receiveSheetTitle')}>
  <div class="space-y-3">
    {#if loadingTargets}
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.loadingTargets')}</p>
    {/if}

    {#if targetsError}
      <p class="text-destructive text-sm">{targetsError}</p>
    {/if}

    {#if !loadingTargets && receiveAssetOptions.length === 0}
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.viaNoOptions')}</p>
    {:else}
      <div class="space-y-2">
        {#each receiveAssetOptions as option}
          {@const bestRoute = sortViaOptionsByScore(option.viaOptions, amount)[0] ?? null}
          <button
            type="button"
            class="flex w-full items-start justify-between rounded-md border px-3 py-2 text-left transition-colors
              {selectedReceiveAssetKey === option.key
                ? 'border-primary/70 bg-primary/5'
                : 'border-border/60 hover:bg-muted/30'}"
            onclick={() => selectReceiveAsset(option.key)}
          >
            <div class="min-w-0">
              <p class="truncate text-sm font-medium">{option.label}</p>
              {#if option.subtitle}
                <p class="text-muted-foreground truncate text-xs">{option.subtitle}</p>
              {/if}
            </div>
            {#if bestRoute}
              <p class="text-muted-foreground ml-3 text-xs">{formatEstimatedReceive(bestRoute)}</p>
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet bind:isOpen={showViaSheet} title={i18n.t('wallet.transfer.viaSheetTitle')}>
  <div class="space-y-3">
    {#if selectedReceiveAssetOption}
      <div class="flex items-center justify-between gap-2">
        <p class="text-sm font-medium">{selectedReceiveAssetOption.label}</p>
        <Button variant="ghost" size="sm" onclick={resetViaToBest}>
          {i18n.t('wallet.transfer.viaResetBest')}
        </Button>
      </div>

      {#if rankedViaOptions.length === 0}
        <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.viaNoOptions')}</p>
      {:else}
        <div class="space-y-2">
          {#each rankedViaOptions as option}
            <button
              type="button"
              class="flex w-full items-start justify-between rounded-md border px-3 py-2 text-left transition-colors
                {selectedViaOptionId === option.id
                  ? 'border-primary/70 bg-primary/5'
                  : 'border-border/60 hover:bg-muted/30'}"
              onclick={() => selectViaOption(option.id)}
            >
              <div class="min-w-0">
                <div class="flex items-center gap-2">
                  <p class="truncate text-sm font-medium">{option.via || i18n.t('wallet.transfer.viaBest')}</p>
                  {#if bestViaOption?.id === option.id}
                    <span class="rounded-full bg-primary/10 px-2 py-0.5 text-[10px] font-semibold text-primary">
                      {i18n.t('wallet.transfer.viaBest')}
                    </span>
                  {/if}
                </div>
                {#if option.subtitle}
                  <p class="text-muted-foreground truncate text-xs">{option.subtitle}</p>
                {/if}
                {#if option.price}
                  <p class="text-muted-foreground text-xs">{i18n.t('wallet.transfer.rate', { value: option.price })}</p>
                {/if}
              </div>
              <p class="text-muted-foreground ml-3 text-xs">
                {i18n.t('wallet.transfer.estimatedForAmount', { value: formatEstimatedReceive(option) })}
              </p>
            </button>
          {/each}
        </div>
      {/if}
    {:else}
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.viaNoOptions')}</p>
    {/if}
  </div>
</StandardRightSheet>
