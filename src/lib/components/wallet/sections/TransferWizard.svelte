<script lang="ts">
  import { onMount, tick } from 'svelte';
  import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import CheckCircle2Icon from '@lucide/svelte/icons/check-circle-2';
  import BookUserIcon from '@lucide/svelte/icons/book-user';
  import InfoIcon from '@lucide/svelte/icons/info';
  import PencilIcon from '@lucide/svelte/icons/pencil';
  import UserRoundIcon from '@lucide/svelte/icons/user-round';
  import XIcon from '@lucide/svelte/icons/x';
  import { Button } from '$lib/components/ui/button';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as Card from '$lib/components/ui/card';
  import * as ScrollArea from '$lib/components/ui/scroll-area';
  import * as Tabs from '$lib/components/ui/tabs';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import SearchInput from '$lib/components/common/SearchInput.svelte';
  import StandardRightSheet from '$lib/components/common/StandardRightSheet.svelte';
  import WalletTransferStepperShell from '$lib/components/shared/WalletTransferStepperShell.svelte';
  import CoinIcon from '$lib/components/wallet/CoinIcon.svelte';
  import TransferSummaryRail from './transfer-wizard/TransferSummaryRail.svelte';
  import { i18nStore } from '$lib/i18n';
  import { resolveCoinPresentation, resolveCoinPresentationById } from '$lib/coins/presentation.js';
  import { coinsStore } from '$lib/stores/coins.js';
  import { walletChannelsStore } from '$lib/stores/walletChannels.js';
  import { balanceStore, getBalance } from '$lib/stores/balances.js';
  import { ratesStore } from '$lib/stores/rates.js';
  import { transactionStore } from '$lib/stores/transactions.js';
  import { addressBookStore, upsertAddressBookContact } from '$lib/stores/addressBook.js';
  import { formatUsdAmount } from '$lib/utils/walletOverview.js';
  import * as addressBookService from '$lib/services/addressBookService.js';
  import {
    endpointKindForDestinationKind,
    findMatchingSavedEndpoint,
    sharesSuspiciousPrefixSuffix
  } from '$lib/address-book/utils';
  import { channelIdForCoin } from '$lib/utils/channelId.js';
  import * as walletService from '$lib/services/walletService.js';
  import { preflightSend, sendTransaction } from '$lib/services/txService.js';
  import {
    estimateBridgeConversion,
    getBridgeCapabilities,
    getBridgeConversionPaths,
    preflightBridgeTransfer
  } from '$lib/services/bridgeTransferService.js';
  import {
    getRecipientInputCopy,
    getTransferStepCopy,
    getTransferStepLabels
  } from '$lib/transfer/transferWizardCopy';
  import {
    buildReceiveAssetSections,
    filterReceiveAssetSectionsByQuery,
    type ReceiveAssetOption,
    type ReceiveAssetSections,
    type ViaRouteOption
  } from '$lib/transfer/convertTargetOptions';
  import type {
    BridgeCapabilitiesResult,
    BridgeConversionPathQuote,
    BridgeTransferPreflightResult,
    PreflightResult,
    SendResult
  } from '$lib/types/wallet.js';
  import type { AddressBookContact, AddressEndpointKind } from '$lib/types/addressBook';
  import type {
    DestinationAddressKind,
    TransferEntryContext,
    TransferStepId,
    TransferStepperStep,
    WizardOperationalStepId
  } from './transfer-wizard/types';

  type EntryIntent = 'send' | 'convert';

  type AddressBookEndpointOption = {
    contactId: string;
    contactName: string;
    endpointId: string;
    endpointKind: AddressEndpointKind;
    endpointLabel: string;
    endpointAddress: string;
    normalizedAddress: string;
    lastUsedAt: number | null;
  };

  type SameAssetOption = {
    id: string;
    label: string;
    destinationId: string;
    receiveLabel: string;
    ethDestination?: boolean;
    convertTo?: string | null;
    exportTo?: string | null;
    via?: string | null;
    mapTo?: string | null;
    price?: string | null;
  };

  type SummaryRow = {
    label: string;
    primary: string;
    secondary?: string;
    breakAll?: boolean;
    iconCoinId?: string;
    iconCoinName?: string;
  };

  type ConversionFeeInfo = {
    amount: string;
    currency: string;
    percentage: string;
  };

  type TransferWizardProps = {
    entryIntent: EntryIntent;
    entryContext?: TransferEntryContext | null;
    onClose?: () => void;
  };

  const defaultClose = () => {};
  const OPERATIONAL_STEPS: WizardOperationalStepId[] = ['details', 'recipient', 'review'];

  /* eslint-disable prefer-const */
  let { entryIntent, entryContext = null, onClose = defaultClose }: TransferWizardProps = $props();
  /* eslint-enable prefer-const */

  const i18n = $derived($i18nStore);
  const coins = $derived($coinsStore);
  const walletChannels = $derived($walletChannelsStore);
  const balances = $derived($balanceStore);
  const rates = $derived($ratesStore);
  const addressBookContacts = $derived($addressBookStore);
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
      const contextChannelId =
        entryContext &&
        !entryContext.readOnly &&
        entryContext.coinId === coin.id
          ? entryContext.channelId
          : null;
      const presentation = resolveCoinPresentation(coin);
      const channelId =
        contextChannelId ??
        walletChannels.byCoinId[coin.id] ??
        channelIdForCoin(coin, walletChannels.vrpcAddress ?? undefined);
      const balanceTotal = channelId ? getBalance(channelId, coin.id, balances)?.total ?? '0' : '0';
      return {
        coin,
        channelId,
        balanceTotal,
        balanceValue: toFiniteNumber(balanceTotal),
        displayName: presentation.displayName,
        displayTicker: presentation.displayTicker
      };
    })
  );

  const positiveSendableCoinOptions = $derived(
    sendableCoinOptions.filter((option) => option.channelId && option.balanceValue > 0)
  );

  const selectedCoinOption = $derived(
    positiveSendableCoinOptions.find((option) => option.coin.id === selectedCoinId) ?? null
  );

  let selectedCoinId = $state('');
  let currentStep = $state<TransferStepId>('details');
  let amount = $state('');
  let amountInputEl = $state<HTMLInputElement | null>(null);
  let destinationAddress = $state('');
  let conversionEnabled = $state(false);
  let conversionInitialized = $state(false);
  let selectedReceiveAssetId = $state('');
  let selectedExportSystemId = $state<string | null>(null);
  let selectedViaOptionId = $state('');
  let manualViaLocked = $state(false);
  let sourceCoinManuallyChosen = $state(false);
  let discoveredPathQuotes = $state<Record<string, BridgeConversionPathQuote[]>>({});
  let receiveSearchTerm = $state('');
  let pendingGroupedReceiveOption = $state<ReceiveAssetOption | null>(null);
  let pendingTargetOption = $state<ReceiveAssetOption | null>(null);
  let routeEstimateOutputs = $state<Record<string, string>>({});
  let bridgeCapabilities = $state<BridgeCapabilitiesResult | null>(null);

  let loadingTargets = $state(false);
  let preflighting = $state(false);
  let sending = $state(false);
  let targetsError = $state('');
  let transferError = $state('');

  let simplePreflightResult = $state<PreflightResult | null>(null);
  let bridgePreflightResult = $state<BridgeTransferPreflightResult | null>(null);
  let sendResult = $state<SendResult | null>(null);
  let addresses = $state<{ vrsc_address: string; eth_address: string; btc_address: string } | null>(null);

  let showSourceAssetSheet = $state(false);
  let showReceiveAssetSheet = $state(false);
  let showViaSheet = $state(false);
  let showNetworkSheet = $state(false);
  let showExportSheet = $state(false);
  let showAddressBookSheet = $state(false);
  let addressBookSearchTerm = $state('');
  let unsavedRecipientConfirmed = $state(false);
  let saveRecipientName = $state('');
  let saveRecipientError = $state('');
  let savingRecipient = $state(false);

  const selectedCoin = $derived(selectedCoinOption?.coin ?? null);

  const selectedCoinPresentation = $derived(
    selectedCoin ? resolveCoinPresentation(selectedCoin) : null
  );

  const selectedChannelId = $derived(selectedCoinOption?.channelId ?? null);

  const selectedChannelPrefix = $derived(selectedChannelId?.split('.')[0] ?? '');
  const sourceSupportsConversion = $derived(
    (() => {
      if (!selectedCoin || !selectedChannelId) return false;
      if (bridgeCapabilities) return bridgeCapabilities.conversionSupported;
      // Preserve existing behavior while capabilities are loading.
      return selectedChannelPrefix === 'vrpc';
    })()
  );

  const selectedBalance = $derived(selectedCoinOption?.balanceTotal ?? '0');
  const selectedBalanceValue = $derived(toFiniteNumber(selectedBalance));

  const selectedSourceAddress = $derived(
    !addresses
      ? ''
      : selectedChannelPrefix === 'vrpc'
        ? addresses.vrsc_address
        : selectedChannelPrefix === 'btc'
          ? addresses.btc_address
          : addresses.eth_address
  );

  const showChooseCurrencyCallToAction = $derived(
    (entryIntent === 'send' || entryIntent === 'convert') && !sourceCoinManuallyChosen
  );

  const receiveAssetSelectionEnabled = $derived(sourceCoinManuallyChosen && !!selectedCoin);

  const sameAssetOption = $derived<SameAssetOption | null>(
    selectedCoin && selectedCoinPresentation
      ? {
          id: `same-${selectedCoin.id}`,
          label: i18n.t('wallet.transfer.sameAssetOption', {
            ticker: selectedCoinPresentation.displayTicker
          }),
          destinationId: selectedCoin.id,
          receiveLabel: selectedCoinPresentation.displayTicker,
          ethDestination: false
        }
      : null
  );

  const rawReceiveAssetSections = $derived<ReceiveAssetSections>(
    buildReceiveAssetSections({
      paths: discoveredPathQuotes,
      sourceCurrencyId: selectedCoin?.currencyId || selectedCoin?.id || '',
      sourceCurrencyAliases: [selectedCoin?.currencyId, selectedCoin?.id].filter(
        (value): value is string => typeof value === 'string' && value.trim().length > 0
      )
    })
  );

  const receiveAssetSections = $derived<ReceiveAssetSections>(
    filterReceiveAssetSectionsByQuery(rawReceiveAssetSections, receiveSearchTerm)
  );

  const receiveAssetOptions = $derived(receiveAssetSections.allOptions);
  const popularReceiveAssetOptions = $derived(receiveAssetSections.popularOptions);
  const otherReceiveAssetOptions = $derived(receiveAssetSections.otherOptions);

  const selectableReceiveAssetOptions = $derived<ReceiveAssetOption[]>(
    rawReceiveAssetSections.allOptions.flatMap((option) =>
      option.isGrouped && option.networkOptions?.length ? option.networkOptions : [option]
    )
  );

  const selectedReceiveAssetOption = $derived(
    selectableReceiveAssetOptions.find((option) => option.id === selectedReceiveAssetId) ?? null
  );

  const selectedReceiveAssetViaOptions = $derived(
    selectedReceiveAssetOption
      ? filterViaOptionsByExport(selectedReceiveAssetOption.viaOptions, selectedExportSystemId)
      : []
  );

  const rankedViaOptions = $derived(
    sortViaOptionsByScore(selectedReceiveAssetViaOptions, amount, routeEstimateOutputs)
  );

  const bestViaOption = $derived(rankedViaOptions[0] ?? null);

  const selectedViaOption = $derived(
    selectedReceiveAssetViaOptions.find((option) => option.id === selectedViaOptionId) ?? null
  );

  const activeConvertRoute = $derived(
    isPositiveAmount(amount) ? selectedViaOption ?? bestViaOption : null
  );
  const activeTargetOption = $derived(conversionEnabled ? activeConvertRoute : sameAssetOption);

  const convertUnavailableForSource = $derived(
    !!selectedCoin && !!selectedChannelId && !!bridgeCapabilities && !bridgeCapabilities.conversionSupported
  );

  const showConvertUnavailable = $derived(
    convertUnavailableForSource && (entryIntent === 'convert' || conversionEnabled)
  );

  const convertUnavailableMessage = $derived(
    (() => {
      const reasonCode = bridgeCapabilities?.reasonCode ?? '';
      if (reasonCode === 'feature_disabled') {
        return i18n.t('wallet.transfer.convertUnavailableFeatureDisabled');
      }
      if (reasonCode === 'eth_not_configured') {
        return i18n.t('wallet.transfer.convertUnavailableEthNotConfigured');
      }
      if (reasonCode === 'unsupported_channel') {
        return i18n.t('wallet.transfer.convertUnavailableUnsupportedChannel');
      }
      return i18n.t('wallet.transfer.convertUnavailableEvm');
    })()
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

  const selfDestinationAddress = $derived(
    !addresses
      ? ''
      : destinationAddressKind === 'eth'
        ? addresses.eth_address
        : destinationAddressKind === 'btc'
          ? addresses.btc_address
          : addresses.vrsc_address
  );

  const destinationEndpointKind = $derived<AddressEndpointKind>(
    endpointKindForDestinationKind(destinationAddressKind)
  );

  const addressBookEndpointOptions = $derived<AddressBookEndpointOption[]>(
    (() => {
      const query = addressBookSearchTerm.trim().toLowerCase();
      const options = addressBookContacts.flatMap((contact: AddressBookContact) =>
        contact.endpoints
          .filter((endpoint) => endpoint.kind === destinationEndpointKind)
          .map((endpoint) => ({
            contactId: contact.id,
            contactName: contact.displayName,
            endpointId: endpoint.id,
            endpointKind: endpoint.kind,
            endpointLabel: endpoint.label,
            endpointAddress: endpoint.address,
            normalizedAddress: endpoint.normalizedAddress,
            lastUsedAt: endpoint.lastUsedAt
          }))
      );

      const filtered = query
        ? options.filter(
            (option) =>
              option.contactName.toLowerCase().includes(query) ||
              option.endpointLabel.toLowerCase().includes(query) ||
              option.endpointAddress.toLowerCase().includes(query)
          )
        : options;

      return filtered.sort((a, b) => {
        if ((a.lastUsedAt ?? 0) !== (b.lastUsedAt ?? 0)) {
          return (b.lastUsedAt ?? 0) - (a.lastUsedAt ?? 0);
        }
        return a.contactName.localeCompare(b.contactName);
      });
    })()
  );

  const matchedSavedRecipient = $derived(
    findMatchingSavedEndpoint(addressBookContacts, destinationEndpointKind, destinationAddress)
  );
  const isSavedRecipient = $derived(!!matchedSavedRecipient);
  const hasRecipientSimilarityWarning = $derived(
    !isSavedRecipient &&
      sharesSuspiciousPrefixSuffix(addressBookContacts, destinationEndpointKind, destinationAddress)
  );
  const activePreflight = $derived(simplePreflightResult ?? bridgePreflightResult);
  const requiresUnsavedRecipientAck = $derived(
    !!destinationAddress.trim() && !!activePreflight && !isSavedRecipient
  );

  const recipientInputCopy = $derived(getRecipientInputCopy(i18n.t, destinationAddressKind));
  const recipientValid = $derived(validateDestinationAddress(destinationAddress, destinationAddressKind));
  const amountValid = $derived(isPositiveAmount(amount));

  const estimatedConversionValue = $derived(
    (() => {
      if (!conversionEnabled || !amountValid || !activeConvertRoute) return null;

      const estimatedOutput = parseEstimatedOutput(routeEstimateOutputs[activeConvertRoute.id]);
      if (estimatedOutput !== null) {
        return estimatedOutput.toFixed(8);
      }

      const numericPrice = parsePrice(activeConvertRoute.price);
      if (numericPrice === null) return null;
      return (Number(amount) * numericPrice).toFixed(8);
    })()
  );

  const selectedReceiveAssetPresentation = $derived(
    selectedReceiveAssetOption
      ? resolveCoinPresentationById(selectedReceiveAssetOption.destinationId)
      : null
  );

  const sourceUsdRate = $derived(
    getUsdRateForCoinIds([selectedCoin?.id, selectedCoin?.currencyId, selectedCoin?.mappedTo])
  );

  const receiveUsdRate = $derived(
    getUsdRateForCoinIds([
      selectedReceiveAssetOption?.destinationId,
      selectedReceiveAssetPresentation?.currencyId,
      selectedReceiveAssetPresentation?.mappedTo
    ])
  );

  const sourceAmountFiatDisplay = $derived(formatFiatEstimate(amount, sourceUsdRate));
  const receiveAmountFiatDisplay = $derived(
    formatFiatEstimate(estimatedConversionValue ?? '0', receiveUsdRate)
  );

  const activeConvertRouteRate = $derived(formatRouteRateValue(activeConvertRoute?.price));

  const activeConvertRouteRateValueText = $derived(
    (() => {
      if (!selectedCoinPresentation || !selectedReceiveAssetOption || !activeConvertRouteRate) {
        return i18n.t('wallet.transfer.summary.notSet');
      }

      return i18n.t('wallet.transfer.ratePair', {
        from: selectedCoinPresentation.displayTicker,
        rate: activeConvertRouteRate,
        to: selectedReceiveAssetOption.label
      });
    })()
  );

  const conversionFeeInfo = $derived<ConversionFeeInfo | null>(
    (() => {
      if (!conversionEnabled || !amountValid || !selectedCoinPresentation) return null;
      const numericAmount = Number(amount.trim());
      if (!Number.isFinite(numericAmount) || numericAmount <= 0) return null;

      const hasVia = !!activeConvertRoute?.via;
      const multiplier = hasVia ? 0.0005 : 0.00025;
      const feeAmount = numericAmount * multiplier;
      if (!Number.isFinite(feeAmount) || feeAmount <= 0) return null;

      return {
        amount: formatDecimalTrimmed(feeAmount),
        currency: selectedCoinPresentation.displayTicker,
        percentage: hasVia ? '0.05%' : '0.025%'
      };
    })()
  );

  const reviewSendingValue = $derived(
    (() => {
      if (!selectedCoinPresentation) return '';
      const baseAmount = (activePreflight?.value ?? amount.trim()).trim();
      if (!baseAmount) return '';
      return `${baseAmount} ${selectedCoinPresentation.displayTicker}`;
    })()
  );

  const reviewReceivingValue = $derived(
    (() => {
      if (!conversionEnabled || !estimatedConversionValue) return '';
      const receiveLabel =
        selectedReceiveAssetOption?.fullyqualifiedname?.trim() ||
        selectedReceiveAssetOption?.ticker?.trim() ||
        selectedReceiveAssetOption?.label?.trim() ||
        '';
      return receiveLabel ? `~${estimatedConversionValue} ${receiveLabel}` : `~${estimatedConversionValue}`;
    })()
  );

  const reviewRouteValue = $derived(
    conversionEnabled && activeConvertRoute ? getViaOptionLabel(activeConvertRoute) : ''
  );

  const reviewDestinationNetworkValue = $derived(
    conversionEnabled
      ? (activeConvertRoute?.exportToLabel ?? activeConvertRoute?.exportTo ?? '')
      : ''
  );

  const reviewNetworkFeeValue = $derived(
    activePreflight ? `${activePreflight.fee} ${activePreflight.feeCurrency}` : ''
  );
  const reviewTotalFeesFiat = $derived(
    (() => {
      if (!activePreflight) return '≈ —';

      let totalFiat = 0;

      const networkFeeAmount = parseNonNegativeAmount(activePreflight.fee);
      const networkFeeRate = getUsdRateForCurrencyLabel(activePreflight.feeCurrency) ?? sourceUsdRate;
      if (networkFeeAmount !== null && networkFeeRate !== null) {
        totalFiat += networkFeeAmount * networkFeeRate;
      }

      if (conversionEnabled && conversionFeeInfo) {
        const conversionAmount = parseNonNegativeAmount(conversionFeeInfo.amount);
        const conversionRate = getUsdRateForCurrencyLabel(conversionFeeInfo.currency) ?? sourceUsdRate;
        if (conversionAmount !== null && conversionRate !== null) {
          totalFiat += conversionAmount * conversionRate;
        }
      }

      if (!Number.isFinite(totalFiat) || totalFiat <= 0) return '≈ —';
      return `≈ ${formatUsdAmountDynamic(totalFiat)}`;
    })()
  );
  const estimatedArrivalInfo = $derived(
    (() => {
      if (!activePreflight) return null;

      if (!conversionEnabled) {
        return {
          value: i18n.t('wallet.transfer.summary.estimatedTimeSimple'),
          tooltip: i18n.t('wallet.transfer.summary.estimatedTimeTooltipSimple')
        };
      }

      if (activeConvertRoute?.exportTo || selectedChannelPrefix === 'eth' || selectedChannelPrefix === 'erc20') {
        return {
          value: i18n.t('wallet.transfer.summary.estimatedTimeBridge'),
          tooltip: i18n.t('wallet.transfer.summary.estimatedTimeTooltipBridge')
        };
      }

      return {
        value: i18n.t('wallet.transfer.summary.estimatedTimePbaas'),
        tooltip: i18n.t('wallet.transfer.summary.estimatedTimeTooltipPbaas')
      };
    })()
  );

  const reviewRecipientName = $derived(matchedSavedRecipient?.contact.displayName?.trim() ?? '');
  const reviewRecipientAddress = $derived(truncateAddressMiddle(destinationAddress, 6, 6));

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
      (currentStep === 'review' && !activePreflight) ||
      (currentStep === 'review' && requiresUnsavedRecipientAck && !unsavedRecipientConfirmed)
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

  const showSummaryAside = $derived(currentStep !== 'review');

  const warningsSummary = $derived(activePreflight?.warnings.map((warning) => warning.message) ?? []);

  const summaryRows = $derived<SummaryRow[]>(
    (() => {
      const rows: SummaryRow[] = [];

      const sourcePrimary = selectedCoinPresentation?.displayName?.trim() ?? '';
      const sourceSecondary = normalizeSummarySecondary(
        sourcePrimary,
        selectedCoinPresentation?.displayTicker ?? ''
      );
      if (sourcePrimary) {
        rows.push({
          label: i18n.t('wallet.transfer.summary.from'),
          primary: sourcePrimary,
          secondary: sourceSecondary,
          iconCoinId: selectedCoin?.id,
          iconCoinName: sourcePrimary
        });
      }

      if (conversionEnabled) {
        const toDisplay = selectedReceiveAssetOption ? getReceiveOptionDisplay(selectedReceiveAssetOption) : null;
        const toPrimary = toDisplay?.primary?.trim() ?? '';
        const toSecondary = toDisplay?.secondary;
        if (toPrimary) {
          rows.push({
            label: i18n.t('wallet.transfer.summary.to'),
            primary: toPrimary,
            secondary: toSecondary,
            iconCoinId: selectedReceiveAssetOption?.destinationId,
            iconCoinName: toPrimary
          });
        }
      }

      if (conversionEnabled && amountValid && activeConvertRoute) {
        const routePrimary = getViaOptionLabel(activeConvertRoute).trim();
        const routeSecondary = normalizeRouteSummarySecondary(
          routePrimary,
          getRouteSubtitle(activeConvertRoute)
        );
        if (routePrimary) {
          rows.push({
            label: i18n.t('wallet.transfer.summary.route'),
            primary: routePrimary,
            secondary: routeSecondary
          });
        }
      }

      if (amountValid) {
        const amountValue = amount.trim();
        const amountFqn = selectedCoinPresentation?.displayTicker?.trim() ?? '';
        if (amountValue) {
          const amountPrimary = amountFqn ? `${amountValue} ${amountFqn}` : amountValue;
          rows.push({
            label: i18n.t('wallet.transfer.summary.amount'),
            primary: amountPrimary
          });
        }
      }

      if (conversionEnabled && estimatedConversionValue && selectedReceiveAssetOption) {
        const estimatedPrimaryValue = estimatedConversionValue.trim();
        const estimatedFqn =
          selectedReceiveAssetOption.fullyqualifiedname?.trim() ||
          selectedReceiveAssetOption.ticker?.trim() ||
          selectedReceiveAssetOption.label?.trim() ||
          '';
        if (estimatedPrimaryValue) {
          const estimatedPrimary = estimatedFqn
            ? `~${estimatedPrimaryValue} ${estimatedFqn}`
            : `~${estimatedPrimaryValue}`;
          rows.push({
            label: i18n.t('wallet.transfer.summary.estimatedReceive'),
            primary: estimatedPrimary
          });
        }
      }

      const recipientAddress = destinationAddress.trim();
      if (recipientAddress) {
        const truncatedRecipientAddress = truncateAddressMiddle(recipientAddress);
        const recipientName = matchedSavedRecipient?.contact.displayName?.trim() ?? '';
        const recipientPrimary = recipientName || truncatedRecipientAddress;
        const recipientSecondary = normalizeSummarySecondary(recipientPrimary, truncatedRecipientAddress);
        rows.push({
          label: i18n.t('wallet.transfer.summary.recipient'),
          primary: recipientPrimary,
          secondary: recipientSecondary
        });
      }

      if (activePreflight) {
        const feePrimary = activePreflight.fee.trim();
        const feeSecondary = normalizeSummarySecondary(feePrimary, activePreflight.feeCurrency);
        if (feePrimary) {
          rows.push({
            label: i18n.t('wallet.transfer.summary.networkFee'),
            primary: feePrimary,
            secondary: feeSecondary
          });
        }
      }

      return rows;
    })()
  );

  const preflightInputSignature = $derived(
    [
      selectedCoinId,
      selectedChannelId ?? '',
      conversionEnabled ? '1' : '0',
      selectedReceiveAssetId,
      selectedExportSystemId ?? '',
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
    const coin = selectedCoin;
    const channelId = selectedChannelId;
    let cancelled = false;

    bridgeCapabilities = null;

    if (!coin || !channelId) {
      return () => {
        cancelled = true;
      };
    }

    void (async () => {
      try {
        const capabilities = await getBridgeCapabilities({
          coinId: coin.id,
          channelId
        });
        if (cancelled) return;
        bridgeCapabilities = capabilities;
      } catch (error) {
        if (cancelled) return;
        bridgeCapabilities = {
          conversionSupported: false,
          executionEngine: 'unknown',
          reasonCode: extractWalletErrorType(error)
        };
      }
    })();

    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    if (positiveSendableCoinOptions.length === 0) {
      selectedCoinId = '';
      sourceCoinManuallyChosen = false;
      return;
    }

    const selectedStillAvailable = positiveSendableCoinOptions.some(
      (option) => option.coin.id === selectedCoinId
    );

    if (!selectedStillAvailable) {
      selectedCoinId = '';
      sourceCoinManuallyChosen = false;
    }
  });

  $effect(() => {
    const context = entryContext;
    if (!context || context.readOnly) return;

    const hasContextCoin = sendableCoinOptions.some((option) => option.coin.id === context.coinId);
    if (!hasContextCoin) return;

    selectedCoinId = context.coinId;
    sourceCoinManuallyChosen = true;
  });

  $effect(() => {
    if (!sourceSupportsConversion) {
      discoveredPathQuotes = {};
      loadingTargets = false;
      targetsError = '';
      selectedReceiveAssetId = '';
      selectedExportSystemId = null;
      selectedViaOptionId = '';
      manualViaLocked = false;
      receiveSearchTerm = '';
      pendingGroupedReceiveOption = null;
      pendingTargetOption = null;
      showReceiveAssetSheet = false;
      showNetworkSheet = false;
      showExportSheet = false;
    }
  });

  $effect(() => {
    if (showConvertUnavailable && conversionEnabled) {
      conversionEnabled = false;
      manualViaLocked = false;
      selectedViaOptionId = '';
    }
  });

  $effect(() => {
    if (showNetworkSheet) return;
    pendingGroupedReceiveOption = null;
  });

  $effect(() => {
    if (showExportSheet) return;
    pendingTargetOption = null;
  });

  $effect(() => {
    if (rawReceiveAssetSections.allOptions.length === 0) {
      selectedReceiveAssetId = '';
      selectedExportSystemId = null;
      selectedViaOptionId = '';
      manualViaLocked = false;
      pendingGroupedReceiveOption = null;
      pendingTargetOption = null;
      showReceiveAssetSheet = false;
      showNetworkSheet = false;
      showExportSheet = false;
      return;
    }

    if (!selectableReceiveAssetOptions.some((option) => option.id === selectedReceiveAssetId)) {
      selectedReceiveAssetId = '';
      selectedExportSystemId = null;
      selectedViaOptionId = '';
      manualViaLocked = false;
    }
  });

  $effect(() => {
    if (!selectedReceiveAssetOption) return;
    if (selectedExportSystemId === null) return;

    const exportStillValid = selectedReceiveAssetOption.exportOptions.some(
      (option) => option.exportTo === selectedExportSystemId
    );
    if (exportStillValid) return;

    selectedExportSystemId = selectedReceiveAssetOption.hasOnChainPath
      ? null
      : selectedReceiveAssetOption.exportOptions[0]?.exportTo ?? null;
    selectedViaOptionId = '';
    manualViaLocked = false;
  });

  $effect(() => {
    if (!conversionEnabled) return;
    if (!selectedReceiveAssetOption) return;
    if (!amountValid) {
      selectedViaOptionId = '';
      manualViaLocked = false;
      return;
    }

    const selectedStillValid = selectedReceiveAssetViaOptions.some((option) => option.id === selectedViaOptionId);

    if (manualViaLocked && selectedStillValid) return;

    const best = sortViaOptionsByScore(
      selectedReceiveAssetViaOptions,
      amount,
      routeEstimateOutputs
    )[0] ?? null;
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
    if (!selectedCoinOption) return;
    const trimmedAmount = amount.trim();
    if (!trimmedAmount) return;
    const numericAmount = Number(trimmedAmount);
    if (!Number.isFinite(numericAmount)) return;
    if (numericAmount <= selectedBalanceValue) return;
    amount = selectedBalance;
  });

  $effect(() => {
    const coin = selectedCoin;
    const channelId = selectedChannelId;
    const sourceCurrency = coin?.currencyId || coin?.id || '';
    const normalizedAmount = amount.trim();
    const numericAmount = Number(normalizedAmount);
    const viaOptions = selectedReceiveAssetViaOptions;
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | null = null;

    if (
      !conversionEnabled ||
      !coin ||
      !channelId ||
      !sourceCurrency ||
      !amountValid ||
      !Number.isFinite(numericAmount) ||
      numericAmount <= 0 ||
      viaOptions.length === 0
    ) {
      routeEstimateOutputs = {};
      return () => {
        cancelled = true;
      };
    }

    routeEstimateOutputs = {};

    const fallbackEstimateOutput = (option: ViaRouteOption): string | null => {
      const price = parsePrice(option.price);
      if (price === null) return null;
      return (numericAmount * price).toString();
    };

    timer = setTimeout(() => {
      void (async () => {
        const estimateCache = new Map<string, Promise<string | null>>();
        const nextOutputs: Record<string, string> = {};

      await Promise.all(
        viaOptions.map(async (option) => {
          const convertTo = option.convertTo?.trim();
          if (!convertTo) {
            const fallback = fallbackEstimateOutput(option);
            if (fallback) nextOutputs[option.id] = fallback;
            return;
          }

          const via = option.via?.trim() || null;
          const estimateKey = `${convertTo.toLowerCase()}|${(via ?? '').toLowerCase()}`;

          let estimatePromise = estimateCache.get(estimateKey);
          if (!estimatePromise) {
            estimatePromise = (async () => {
              try {
                const response = await estimateBridgeConversion({
                  coinId: coin.id,
                  channelId,
                  sourceCurrency,
                  convertTo,
                  amount: normalizedAmount,
                  via,
                  preconvert: false
                });
                const estimatedOut = parseEstimatedOutput(response.estimatedCurrencyOut ?? null);
                if (estimatedOut !== null) return estimatedOut.toString();

                const estimatedPrice = parsePrice(response.price ?? null);
                if (estimatedPrice !== null) return (numericAmount * estimatedPrice).toString();
              } catch {
                // Fall through to price-based fallback below.
              }
              return null;
            })();
            estimateCache.set(estimateKey, estimatePromise);
          }

          const estimateOutput = await estimatePromise;
          if (estimateOutput) {
            nextOutputs[option.id] = estimateOutput;
            return;
          }

          const fallback = fallbackEstimateOutput(option);
          if (fallback) {
            nextOutputs[option.id] = fallback;
          }
        })
      );

        if (cancelled) return;
        routeEstimateOutputs = nextOutputs;
      })();
    }, 350);

    return () => {
      cancelled = true;
      if (timer) {
        clearTimeout(timer);
      }
    };
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
        discoveredPathQuotes = response.paths;
      } catch (error) {
        if (cancelled) return;
        discoveredPathQuotes = {};
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
      } catch {
        addresses = null;
      }
    })();
  });

  $effect(() => {
    destinationAddress;
    unsavedRecipientConfirmed = false;
    saveRecipientError = '';
  });

  $effect(() => {
    if (currentStep !== 'details') return;
    void tick().then(() => amountInputEl?.focus());
  });

  function clearPreflightState() {
    simplePreflightResult = null;
    bridgePreflightResult = null;
    transferError = '';
  }

  function shortRecipientAddress(value: string): string {
    const trimmed = value.trim();
    if (trimmed.length <= 24) return trimmed;
    return `${trimmed.slice(0, 10)}...${trimmed.slice(-10)}`;
  }

  function endpointBadgeLabel(kind: AddressEndpointKind): string {
    if (kind === 'vrpc') return 'VERUS';
    return kind.toUpperCase();
  }

  function selectAddressBookRecipient(option: AddressBookEndpointOption) {
    destinationAddress = option.endpointAddress;
    showAddressBookSheet = false;
    addressBookSearchTerm = '';
    transferError = '';
    saveRecipientName = option.contactName;
  }

  function selectSelfRecipient() {
    if (!selfDestinationAddress) return;
    destinationAddress = selfDestinationAddress;
    transferError = '';
  }

  async function pasteRecipientAddress() {
    try {
      const pastedAddress = (await walletService.readClipboardText()).trim();
      if (!pastedAddress) return;
      destinationAddress = pastedAddress;
      transferError = '';
    } catch {
      // Ignore clipboard read errors and keep manual entry available.
    }
  }

  function mapAddressBookError(error: unknown): string {
    const errorType = extractWalletErrorType(error);
    if (errorType === 'AddressBookDuplicate') return i18n.t('wallet.transfer.saveRecipient.error.duplicate');
    if (errorType === 'AddressBookInvalidInput' || errorType === 'InvalidAddress') {
      return i18n.t('wallet.transfer.saveRecipient.error.invalid');
    }
    if (errorType === 'WalletLocked') return i18n.t('wallet.transfer.saveRecipient.error.walletLocked');
    if (error instanceof Error && error.message.trim()) return error.message;
    return i18n.t('wallet.transfer.saveRecipient.error.generic');
  }

  async function saveRecipientFromSuccess() {
    if (!sendResult || savingRecipient) return;

    const displayName = saveRecipientName.trim();
    if (!displayName) {
      saveRecipientError = i18n.t('wallet.transfer.saveRecipient.error.nameRequired');
      return;
    }

    savingRecipient = true;
    saveRecipientError = '';

    try {
      const validation = await addressBookService.validateDestinationAddress({
        kind: destinationEndpointKind,
        address: sendResult.toAddress
      });
      if (!validation.valid) {
        saveRecipientError = i18n.t('wallet.transfer.saveRecipient.error.invalid');
        return;
      }

      const saved = await addressBookService.saveAddressBookContact({
        displayName,
        note: null,
        endpoints: [
          {
            kind: destinationEndpointKind,
            label: i18n.t('wallet.transfer.saveRecipient.defaultEndpointLabel'),
            address: sendResult.toAddress
          }
        ]
      });
      upsertAddressBookContact(saved);
    } catch (error) {
      saveRecipientError = mapAddressBookError(error);
    } finally {
      savingRecipient = false;
    }
  }

  function filterViaOptionsByExport(
    options: ViaRouteOption[],
    exportSystemId: string | null
  ): ViaRouteOption[] {
    return options.filter((option) => {
      if (exportSystemId === null) {
        return option.exportTo === null || option.exportTo === undefined;
      }
      return option.exportTo === exportSystemId;
    });
  }

  function getRouteSubtitle(option: ViaRouteOption): string {
    const subtitleParts: string[] = [];
    if (option.exportTo) {
      subtitleParts.push(
        i18n.t('wallet.transfer.pathExportTo', { value: option.exportToLabel ?? option.exportTo })
      );
    }
    if (option.via) {
      subtitleParts.push(i18n.t('wallet.transfer.pathVia', { value: option.viaLabel ?? option.via }));
    }
    if (option.mapTo) {
      subtitleParts.push(i18n.t('wallet.transfer.pathMapTo', { value: option.mapTo }));
    }
    return subtitleParts.join(' • ');
  }

  function getViaSheetSubtitle(option: ViaRouteOption): string {
    const subtitleParts: string[] = [];
    if (option.exportTo) {
      subtitleParts.push(
        i18n.t('wallet.transfer.pathExportTo', { value: option.exportToLabel ?? option.exportTo })
      );
    }
    if (option.mapTo) {
      subtitleParts.push(i18n.t('wallet.transfer.pathMapTo', { value: option.mapTo }));
    }
    return subtitleParts.join(' • ');
  }

  function getViaOptionLabel(option: ViaRouteOption): string {
    return option.viaLabel ?? option.via ?? i18n.t('wallet.transfer.viaBest');
  }

  function isReceiveOptionSelected(option: ReceiveAssetOption): boolean {
    if (option.isGrouped && option.networkOptions?.length) {
      return option.networkOptions.some((networkOption) => networkOption.id === selectedReceiveAssetId);
    }
    return option.id === selectedReceiveAssetId;
  }

  function closeAssetSelectionSheets() {
    showReceiveAssetSheet = false;
    showNetworkSheet = false;
    showExportSheet = false;
  }

  function finalizeReceiveSelection(option: ReceiveAssetOption, exportSystemId: string | null) {
    selectedReceiveAssetId = option.id;
    selectedExportSystemId = exportSystemId;
    manualViaLocked = false;

    if (amountValid) {
      const viaCandidates = filterViaOptionsByExport(option.viaOptions, exportSystemId);
      const best = sortViaOptionsByScore(viaCandidates, amount, routeEstimateOutputs)[0] ?? null;
      selectedViaOptionId = best?.id ?? '';
    } else {
      selectedViaOptionId = '';
    }

    pendingGroupedReceiveOption = null;
    pendingTargetOption = null;
    closeAssetSelectionSheets();
    transferError = '';
  }

  function beginReceiveSelection(option: ReceiveAssetOption) {
    pendingTargetOption = option;
    showReceiveAssetSheet = false;
    showNetworkSheet = false;

    if (option.isCrossChain && option.exportOptions.length > 0) {
      showExportSheet = true;
      return;
    }

    finalizeReceiveSelection(option, null);
  }

  function isPositiveAmount(input: string): boolean {
    const value = Number(input);
    return Number.isFinite(value) && value > 0;
  }

  function toFiniteNumber(value: unknown): number {
    if (typeof value === 'number') {
      return Number.isFinite(value) ? value : 0;
    }

    if (typeof value === 'string') {
      const parsed = Number(value.trim());
      return Number.isFinite(parsed) ? parsed : 0;
    }

    return 0;
  }

  function formatSheetBalance(value: string): string {
    const numeric = toFiniteNumber(value);
    return i18n.formatNumber(numeric, {
      minimumFractionDigits: 0,
      maximumFractionDigits: 8
    });
  }

  function getReceiveOptionDisplay(option: ReceiveAssetOption): { primary: string; secondary?: string } {
    const presentation = resolveCoinPresentationById(option.destinationId);
    const primary =
      presentation?.displayName?.trim() ||
      option.subtitle?.trim() ||
      option.label.trim();
    const secondaryCandidate =
      presentation?.displayTicker?.trim() ||
      option.fullyqualifiedname?.trim() ||
      option.ticker?.trim();
    const secondary =
      secondaryCandidate && secondaryCandidate.toLowerCase() !== primary.toLowerCase()
        ? secondaryCandidate
        : undefined;

    return { primary, secondary };
  }

  function validateDestinationAddress(value: string, kind: DestinationAddressKind): boolean {
    const input = value.trim();
    if (!input) return false;

    if (kind === 'eth') {
      return /^0x[a-fA-F0-9]{40}$/.test(input);
    }
    if (kind === 'btc') {
      if (/^(bc1|tb1)[ac-hj-np-z02-9]{11,71}$/i.test(input)) {
        return true;
      }
      return /^[13mn2][a-km-zA-HJ-NP-Z1-9]{25,39}$/.test(input);
    }

    if (/[A-Za-z0-9._-]+@$/.test(input)) {
      return true;
    }

    return /^[Ri][a-km-zA-HJ-NP-Z1-9]{24,60}$/.test(input);
  }

  function viaLexicalKey(option: ViaRouteOption): string {
    return `${option.via ?? ''}|${option.exportTo ?? ''}|${option.mapTo ?? ''}`.toLowerCase();
  }

  function parseNonNegativeAmount(value?: string | null): number | null {
    if (typeof value !== 'string') return null;
    const trimmed = value.trim();
    if (!trimmed) return 0;
    const parsed = Number(trimmed);
    if (!Number.isFinite(parsed) || parsed < 0) return null;
    return parsed;
  }

  function getUsdRate(rateMap?: Record<string, number>): number | null {
    if (!rateMap) return null;
    const candidate = rateMap.USD ?? rateMap.usd;
    if (typeof candidate !== 'number' || !Number.isFinite(candidate) || candidate <= 0) {
      return null;
    }
    return candidate;
  }

  function getUsdRateForCoinIds(coinIds: Array<string | null | undefined>): number | null {
    const seen = new Set<string>();

    for (const rawCoinId of coinIds) {
      if (typeof rawCoinId !== 'string') continue;
      const coinId = rawCoinId.trim();
      if (!coinId || seen.has(coinId)) continue;
      seen.add(coinId);

      const usdRate = getUsdRate(rates[coinId]?.rates);
      if (usdRate !== null) return usdRate;
    }

    return null;
  }

  function getUsdRateForCurrencyLabel(label?: string | null): number | null {
    if (typeof label !== 'string') return null;
    const normalizedLabel = label.trim().toLowerCase();
    if (!normalizedLabel) return null;

    for (const coin of coins) {
      const presentation = resolveCoinPresentation(coin);
      if (
        coin.id.toLowerCase() === normalizedLabel ||
        coin.currencyId.toLowerCase() === normalizedLabel ||
        presentation.displayTicker.toLowerCase() === normalizedLabel ||
        presentation.displayName.toLowerCase() === normalizedLabel
      ) {
        return getUsdRateForCoinIds([coin.id, coin.currencyId, coin.mappedTo]);
      }
    }

    return null;
  }

  function formatFiatEstimate(amountValue: string | null | undefined, usdRate: number | null): string {
    const numericAmount = parseNonNegativeAmount(amountValue);
    if (numericAmount === null || usdRate === null) return '≈ —';
    return `≈ ${formatUsdAmount(numericAmount * usdRate, i18n.intlLocale)}`;
  }

  function formatUsdAmountDynamic(value: number): string {
    const absoluteValue = Math.abs(value);
    const maximumFractionDigits =
      absoluteValue < 0.0001 ? 8 : absoluteValue < 0.001 ? 7 : absoluteValue < 0.01 ? 6 : absoluteValue < 0.1 ? 4 : 2;

    return new Intl.NumberFormat(i18n.intlLocale, {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
      maximumFractionDigits
    }).format(value);
  }

  function parsePrice(value?: string | null): number | null {
    if (!value) return null;
    const parsed = Number(value);
    if (!Number.isFinite(parsed) || parsed <= 0) return null;
    return parsed;
  }

  function parseEstimatedOutput(value?: string | null): number | null {
    if (!value) return null;
    const parsed = Number(value);
    if (!Number.isFinite(parsed) || parsed <= 0) return null;
    return parsed;
  }

  function formatRouteRateValue(value?: string | null): string | null {
    const numeric = parsePrice(value);
    if (numeric === null) return null;
    return numeric.toFixed(8).replace(/\.?0+$/, '');
  }

  function formatDecimalTrimmed(value: number, digits = 8): string {
    return value.toFixed(digits).replace(/\.?0+$/, '');
  }

  function routeScore(
    option: ViaRouteOption,
    amountInput: string,
    estimatedOutputs: Record<string, string> = {}
  ): number | null {
    const estimatedOutput = parseEstimatedOutput(estimatedOutputs[option.id]);
    if (estimatedOutput !== null) {
      return estimatedOutput;
    }

    const price = parsePrice(option.price);
    if (price === null) return null;

    const numericAmount = Number(amountInput);
    if (Number.isFinite(numericAmount) && numericAmount > 0) {
      return price * numericAmount;
    }

    return price;
  }

  function sortViaOptionsByScore(
    options: ViaRouteOption[],
    amountInput: string,
    estimatedOutputs: Record<string, string> = {}
  ): ViaRouteOption[] {
    return [...options].sort((a, b) => {
      const scoreA = routeScore(a, amountInput, estimatedOutputs);
      const scoreB = routeScore(b, amountInput, estimatedOutputs);

      if (scoreA !== null && scoreB !== null && scoreA !== scoreB) {
        return scoreB - scoreA;
      }

      if (scoreA !== null && scoreB === null) return -1;
      if (scoreA === null && scoreB !== null) return 1;

      return viaLexicalKey(a).localeCompare(viaLexicalKey(b));
    });
  }

  function formatEstimatedReceive(option: ViaRouteOption): string | null {
    if (!amountValid) return null;
    const estimatedOutput = parseEstimatedOutput(routeEstimateOutputs[option.id]);
    if (estimatedOutput !== null) {
      return estimatedOutput.toFixed(8);
    }
    const price = parsePrice(option.price);
    if (price === null) return null;
    return (Number(amount) * price).toFixed(8);
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

  function extractWalletErrorMessage(error: unknown): string | null {
    if (!error || typeof error !== 'object') return null;
    const object = error as Record<string, unknown>;

    if (typeof object.message === 'string' && object.message.trim()) {
      return object.message.trim();
    }
    if (object.data && typeof object.data === 'object') {
      const data = object.data as Record<string, unknown>;
      if (typeof data.message === 'string' && data.message.trim()) {
        return data.message.trim();
      }
    }
    return null;
  }

  function mapWalletError(error: unknown): string {
    const errorType = extractWalletErrorType(error);
    const rawMessage = extractWalletErrorMessage(error);
    if (errorType === 'BridgeNotImplemented') return i18n.t('wallet.transfer.error.bridgeNotImplemented');
    if (errorType === 'BridgeRouteInvalid') return i18n.t('wallet.transfer.error.bridgeRouteInvalid');
    if (errorType === 'BridgeUnsupportedDestinationCombination') {
      return i18n.t('wallet.transfer.error.bridgeUnsupportedDestinationCombination');
    }
    if (errorType === 'BridgeApprovalFailed') return i18n.t('wallet.transfer.error.bridgeApprovalFailed');
    if (errorType === 'BridgeInsufficientEthFeeEnvelope') {
      return i18n.t('wallet.transfer.error.bridgeInsufficientEthFeeEnvelope');
    }
    if (errorType === 'BridgeGasDriftExceeded') return i18n.t('wallet.transfer.error.bridgeGasDriftExceeded');
    if (errorType === 'UnsupportedChannel') return i18n.t('wallet.transfer.error.unsupportedChannel');
    if (errorType === 'InvalidAddress') return i18n.t('wallet.transfer.error.invalidAddress');
    if (errorType === 'InsufficientFunds') return i18n.t('wallet.transfer.error.insufficientFunds');
    if (errorType === 'NetworkError') return i18n.t('wallet.transfer.error.network');
    if (errorType === 'OperationFailed') {
      if (rawMessage && rawMessage.toLowerCase() !== 'operation failed') return rawMessage;
      return i18n.t('wallet.transfer.error.operationFailed');
    }

    if (rawMessage) return rawMessage;
    if (error instanceof Error && error.message) return error.message;
    return i18n.t('common.unknownError');
  }

  function normalizeSummarySecondary(primary: string, secondary: string | null | undefined): string | undefined {
    const primaryTrimmed = primary.trim();
    const secondaryTrimmed = (secondary ?? '').trim();
    if (!secondaryTrimmed) return undefined;
    if (secondaryTrimmed.toLowerCase() === primaryTrimmed.toLowerCase()) return undefined;
    return secondaryTrimmed;
  }

  function normalizeRouteSummarySecondary(
    primary: string,
    secondary: string | null | undefined
  ): string | undefined {
    const normalized = normalizeSummarySecondary(primary, secondary);
    if (!normalized) return undefined;

    const viaOnly = i18n.t('wallet.transfer.pathVia', { value: primary }).trim();
    if (normalized.toLowerCase() === viaOnly.toLowerCase()) return undefined;

    return normalized;
  }

  function truncateAddressMiddle(value: string, startLength = 6, endLength = 6): string {
    const trimmed = value.trim();
    if (!trimmed) return '';
    const minimumVisibleLength = startLength + endLength + 3;
    if (trimmed.length <= minimumVisibleLength) return trimmed;
    return `${trimmed.slice(0, startLength)}...${trimmed.slice(-endLength)}`;
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
    if (!selectedCoin || !selectedChannelId || !activeTargetOption || !recipientValid) return;

    preflighting = true;
    transferError = '';

    try {
      if (conversionEnabled && activeConvertRoute) {
        bridgePreflightResult = await preflightBridgeTransfer({
          coinId: selectedCoin.id,
          channelId: selectedChannelId,
          sourceAddress: selectedSourceAddress || null,
          destination: destinationAddress.trim(),
          amount: amount.trim(),
          convertTo: activeConvertRoute.convertTo ?? null,
          exportTo: activeConvertRoute.exportTo ?? null,
          via: activeConvertRoute.via ?? null,
          mapTo: activeConvertRoute.mapTo ?? null,
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
      console.error('[TransferWizard] preflight failed', {
        type: extractWalletErrorType(error),
        message: extractWalletErrorMessage(error),
        error
      });
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
      if (matchedSavedRecipient) {
        void addressBookService.markAddressBookEndpointUsed(matchedSavedRecipient.endpoint.id);
      }
      await refreshTxHistory();
      currentStep = 'success';
    } catch (error) {
      console.error('[TransferWizard] broadcast failed', {
        type: extractWalletErrorType(error),
        message: extractWalletErrorMessage(error),
        error
      });
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

  function clearAmount() {
    amount = '';
    transferError = '';
  }

  function setTransferMode(mode: 'send' | 'convert') {
    if (mode === 'convert') {
      if (selectedCoin && !sourceSupportsConversion) return;
      conversionEnabled = true;
      manualViaLocked = false;
      return;
    }

    conversionEnabled = false;
    manualViaLocked = false;
    selectedViaOptionId = '';
  }

  function selectSourceCoin(coinId: string) {
    selectedCoinId = coinId;
    sourceCoinManuallyChosen = true;
    showSourceAssetSheet = false;
    transferError = '';
  }

  function selectReceiveAsset(optionId: string) {
    const selected = receiveAssetOptions.find((option) => option.id === optionId);
    if (!selected) return;

    if (selected.isGrouped && selected.networkOptions?.length) {
      pendingGroupedReceiveOption = selected;
      pendingTargetOption = null;
      showReceiveAssetSheet = false;
      showExportSheet = false;
      showNetworkSheet = true;
      transferError = '';
      return;
    }

    pendingGroupedReceiveOption = null;
    beginReceiveSelection(selected);
  }

  function selectReceiveNetworkOption(optionId: string) {
    const selected = pendingGroupedReceiveOption?.networkOptions?.find((option) => option.id === optionId);
    if (!selected) return;
    beginReceiveSelection(selected);
  }

  function selectExportOption(exportSystemId: string) {
    if (!pendingTargetOption) return;
    finalizeReceiveSelection(pendingTargetOption, exportSystemId);
  }

  function selectSameNetworkOption() {
    if (!pendingTargetOption) return;
    finalizeReceiveSelection(pendingTargetOption, null);
  }

  function selectViaOption(viaOptionId: string) {
    selectedViaOptionId = viaOptionId;
    manualViaLocked = true;
    showViaSheet = false;
    transferError = '';
  }

</script>

<WalletTransferStepperShell
  currentStep={stepNumber}
  totalSteps={OPERATIONAL_STEPS.length}
  steps={stepperSteps}
  onClose={onClose}
  closeDisabled={isBusy}
  dirty={isDirty}
  showAside={showSummaryAside}
  mobileAsideLabel={i18n.t('wallet.transfer.viewSummary')}
  mobileAsideTitle={i18n.t('wallet.transfer.summary.title')}
>
  {#snippet aside()}
    <TransferSummaryRail
      rows={summaryRows}
      warnings={warningsSummary}
      class="h-full"
    />
  {/snippet}

  {#snippet footer()}
    {#if currentStep === 'success'}
      <div class="flex justify-end md:hidden">
        <Button onclick={handleDone}>
          {i18n.t('common.done')}
        </Button>
      </div>
    {:else}
      <div class="flex items-center justify-between gap-3">
        <Button variant="secondary" onclick={goBack} disabled={isBusy}>
          {currentStep === 'details' ? i18n.t('common.cancel') : i18n.t('common.back')}
        </Button>
        {#if currentStep === 'review'}
          <div class="flex items-center gap-3">
            {#if requiresUnsavedRecipientAck}
              <div class="flex items-center gap-1.5">
                <Checkbox id="review-unsaved-recipient-footer" bind:checked={unsavedRecipientConfirmed} />
                <Label for="review-unsaved-recipient-footer" class="text-muted-foreground text-[11px] whitespace-nowrap">
                  {i18n.t('wallet.transfer.review.unsavedConfirmShort')}
                </Label>
              </div>
            {/if}
            <Button onclick={continueFlow} disabled={primaryDisabled}>
              {primaryLabel}
            </Button>
          </div>
        {:else}
          <Button class="md:hidden" onclick={continueFlow} disabled={primaryDisabled}>
            {primaryLabel}
          </Button>
        {/if}
      </div>
    {/if}
  {/snippet}

  {#snippet footerAside()}
    {#if currentStep === 'success'}
      <div class="hidden w-full justify-end md:flex">
        <Button onclick={handleDone}>
          {i18n.t('common.done')}
        </Button>
      </div>
    {:else}
      <div class="hidden w-full justify-end md:flex">
        <Button onclick={continueFlow} disabled={primaryDisabled}>
          {primaryLabel}
        </Button>
      </div>
    {/if}
  {/snippet}

  <div class={currentStep === 'details' ? 'space-y-4' : currentStep === 'review' ? 'space-y-3' : 'space-y-5'}>

    {#if transferError}
      <div class="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
        {transferError}
      </div>
    {/if}

    {#if currentStep === 'details'}
      <Card.Root class="border-0 bg-transparent py-0 shadow-none">
        <Card.Content class="space-y-4 px-0">
          <div class="-mt-1 flex w-full justify-center">
            <Tabs.Root value={conversionEnabled ? 'convert' : 'send'} class="w-auto">
              <Tabs.List class="mx-auto rounded-xl bg-muted/80 p-1 dark:bg-muted/55">
                <Tabs.Trigger
                  value="send"
                  class="h-8 min-w-[5.75rem] rounded-lg px-3 text-sm font-semibold data-[state=active]:shadow-none"
                  onclick={() => setTransferMode('send')}
                >
                  {i18n.t('wallet.overview.send')}
                </Tabs.Trigger>
                <Tabs.Trigger
                  value="convert"
                  class="h-8 min-w-[5.75rem] rounded-lg px-3 text-sm font-semibold data-[state=active]:shadow-none"
                  disabled={!!selectedCoin && !sourceSupportsConversion}
                  onclick={() => setTransferMode('convert')}
                >
                  {i18n.t('wallet.overview.convert')}
                </Tabs.Trigger>
              </Tabs.List>
            </Tabs.Root>
          </div>

          <div class="relative mx-auto w-full max-w-[560px] space-y-3.5">
            <div class="space-y-1.5">
              <p class="text-muted-foreground px-1 text-[10px] font-semibold tracking-[0.06em] uppercase">
                {i18n.t('wallet.transfer.youSend')}
              </p>
              <section class="rounded-[20px] bg-transparent p-4">
                <div class="flex items-start justify-between gap-3">
                  <div class="min-w-0 flex-1">
                    <Label for="transfer-amount" class="sr-only">{i18n.t('wallet.transfer.amountLabel')}</Label>
                    <div class="relative">
                      <Input
                        bind:ref={amountInputEl}
                        id="transfer-amount"
                        type="text"
                        inputmode="decimal"
                        placeholder={i18n.t('wallet.transfer.amountPlaceholder')}
                        bind:value={amount}
                        class="h-auto min-h-0 border-0 !bg-transparent dark:!bg-transparent px-0 py-0 pr-8 text-foreground placeholder:text-foreground dark:placeholder:text-foreground text-[2.5rem] md:text-[2.5rem] font-semibold leading-none tracking-tight focus-visible:ring-0"
                      />
                      {#if amount.trim()}
                        <button
                          type="button"
                          class="text-muted-foreground hover:text-foreground focus-visible:ring-ring/50 absolute top-1/2 right-0 -translate-y-1/2 rounded-sm p-1 transition-colors focus-visible:outline-none focus-visible:ring-2"
                          onclick={clearAmount}
                          aria-label={i18n.t('wallet.transfer.amountClear')}
                          title={i18n.t('wallet.transfer.amountClear')}
                        >
                          <XIcon class="size-3.5" />
                        </button>
                      {/if}
                    </div>
                    <p class="text-muted-foreground mt-1 px-0.5 text-xs tabular-nums">{sourceAmountFiatDisplay}</p>
                  </div>

                  <div class="flex h-12 max-w-[72%] shrink-0 flex-col items-end">
                    <Button
                      variant={showChooseCurrencyCallToAction ? 'default' : 'ghost'}
                      class="max-w-full
                        {showChooseCurrencyCallToAction
                          ? 'h-9 self-center rounded-md px-4 justify-center text-sm font-semibold'
                          : 'h-12 rounded-full px-3.5 justify-start gap-2.5 bg-muted/60 hover:bg-muted/70 dark:bg-muted/45 dark:hover:bg-muted/55'}"
                      onclick={() => (showSourceAssetSheet = true)}
                    >
                      {#if showChooseCurrencyCallToAction}
                        <span class="truncate text-sm font-semibold">{i18n.t('wallet.transfer.chooseCurrency')}</span>
                      {:else}
                        {#if selectedCoin}
                          <CoinIcon
                            coinId={selectedCoin.id}
                            coinName={selectedCoinPresentation?.displayName}
                            size={24}
                            decorative={true}
                          />
                        {/if}
                        <span class="min-w-0 text-left leading-tight">
                          <span class="block truncate text-base font-semibold">
                            {selectedCoinPresentation?.displayName || i18n.t('wallet.transfer.sourceAsset')}
                          </span>
                          {#if selectedCoinPresentation?.displayTicker}
                            <span class="text-muted-foreground block truncate text-xs">
                              {selectedCoinPresentation.displayTicker}
                            </span>
                          {/if}
                        </span>
                        <ChevronRightIcon class="text-foreground/45 ml-auto size-4 shrink-0" />
                      {/if}
                    </Button>
                  </div>
                </div>

                <div class="mt-2 min-h-6 flex items-center justify-between gap-2">
                  <div class="min-w-0 flex-1">
                    {#if !amountValid && amount.trim()}
                      <p class="text-destructive truncate text-xs">{i18n.t('wallet.transfer.amountInvalid')}</p>
                    {/if}
                  </div>

                  {#if selectedCoinOption}
                    <div class="flex shrink-0 items-center gap-2">
                      <p class="text-muted-foreground truncate text-xs">{formatSheetBalance(selectedBalance)}</p>
                      <Button variant="secondary" size="sm" class="h-6 rounded-full px-2.5 text-[11px]" onclick={setMaxAmount}>
                        {i18n.t('wallet.transfer.max')}
                      </Button>
                    </div>
                  {/if}
                </div>
              </section>
            </div>

            {#if conversionEnabled}
              <div class="-my-1 flex justify-center text-muted-foreground">
                <ArrowDownIcon class="size-4" />
              </div>

              <div class="space-y-1.5">
                <p class="text-muted-foreground px-1 text-[10px] font-semibold tracking-[0.06em] uppercase">
                  {i18n.t('wallet.transfer.youReceive')}
                </p>
                <section class="rounded-[20px] bg-transparent p-4">
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0 flex-1">
                      <p class="text-muted-foreground text-[2.5rem] md:text-[2.5rem] font-semibold leading-none tracking-tight">
                        {estimatedConversionValue || '0'}
                      </p>
                      <p class="text-muted-foreground mt-1 px-0.5 text-xs tabular-nums">{receiveAmountFiatDisplay}</p>
                    </div>
                    <Button
                      variant={selectedReceiveAssetOption ? 'ghost' : 'default'}
                      class="max-w-[72%] shrink-0
                        {selectedReceiveAssetOption
                          ? 'h-12 rounded-full px-3.5 justify-start gap-2.5 bg-muted/60 hover:bg-muted/70 dark:bg-muted/45 dark:hover:bg-muted/55'
                          : 'h-9 rounded-md px-4 justify-center text-sm font-semibold'}"
                      disabled={!receiveAssetSelectionEnabled}
                      onclick={() => {
                        if (!receiveAssetSelectionEnabled) return;
                        showNetworkSheet = false;
                        showExportSheet = false;
                        pendingGroupedReceiveOption = null;
                        pendingTargetOption = null;
                        showReceiveAssetSheet = true;
                      }}
                    >
                      {#if selectedReceiveAssetOption}
                        {@const receiveDisplay = getReceiveOptionDisplay(selectedReceiveAssetOption)}
                        <CoinIcon
                          coinId={selectedReceiveAssetOption.destinationId}
                          coinName={receiveDisplay.primary}
                          size={24}
                          decorative={true}
                        />
                        <span class="min-w-0 text-left leading-tight">
                          <span class="block truncate text-base font-semibold">{receiveDisplay.primary}</span>
                          {#if receiveDisplay.secondary}
                            <span class="text-muted-foreground block truncate text-xs">{receiveDisplay.secondary}</span>
                          {/if}
                        </span>
                        <ChevronRightIcon class="text-foreground/45 ml-auto size-4 shrink-0" />
                      {:else}
                        <span class="truncate text-sm font-semibold">{i18n.t('wallet.transfer.receiveAsset')}</span>
                      {/if}
                    </Button>
                  </div>

                  {#if selectedReceiveAssetOption && amountValid}
                    <div class="mt-8 mx-auto w-full max-w-[22rem] rounded-xl bg-muted/25 p-2.5">
                      <button
                        type="button"
                        class="focus-visible:ring-ring/60 hover:bg-muted/50 dark:hover:bg-muted/60 flex w-full items-center justify-between rounded-lg px-2 py-1.5 text-left outline-none focus-visible:ring-2"
                        onclick={() => (showViaSheet = true)}
                      >
                        <span class="text-muted-foreground text-[11px] font-medium">{i18n.t('wallet.transfer.conversionRoute')}</span>
                        <span class="flex items-center gap-1 text-sm font-semibold">
                          {activeConvertRoute ? getViaOptionLabel(activeConvertRoute) : i18n.t('wallet.transfer.viaBest')}
                          <ChevronRightIcon class="text-foreground/45 size-3.5 shrink-0" />
                        </span>
                      </button>

                      <div class="mt-1 flex items-start justify-between gap-3 px-2 py-1">
                        <span class="text-muted-foreground text-[11px] font-medium">{i18n.t('wallet.transfer.rateLabel')}</span>
                        <span class="text-foreground max-w-[70%] text-right text-xs font-medium tabular-nums">
                          {activeConvertRouteRateValueText}
                        </span>
                      </div>
                    </div>
                  {/if}

                  {#if targetsError}
                    <p class="text-destructive mt-2 text-xs">{targetsError}</p>
                  {/if}
                </section>
              </div>
            {/if}
          </div>

          {#if showConvertUnavailable}
            <div class="rounded-md border border-border/60 px-3 py-2 text-sm text-muted-foreground">
              {convertUnavailableMessage}
            </div>
          {/if}

          {#if !loadingTargets && conversionEnabled && rawReceiveAssetSections.allOptions.length === 0 && sourceSupportsConversion}
            <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.noRoutes')}</p>
          {/if}

          {#if positiveSendableCoinOptions.length === 0}
            <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.noAssets')}</p>
          {/if}
        </Card.Content>
      </Card.Root>
    {/if}

    {#if currentStep === 'recipient'}
      <div class="flex min-h-[52vh] items-center justify-center">
        <Card.Root class="w-full border-0 bg-transparent py-0 shadow-none">
          <Card.Header class="px-0 text-center">
            <Card.Title>{stepCopy.recipient.title}</Card.Title>
          </Card.Header>
          <Card.Content class="px-0">
            <div class="mx-auto flex w-full max-w-[560px] flex-col items-center gap-3 text-center">
              <Label for="transfer-recipient" class="sr-only">{i18n.t('wallet.transfer.recipientLabel')}</Label>
              <div class="relative w-full">
                <Input
                  id="transfer-recipient"
                  class="h-11 rounded-xl bg-muted/85 px-4 pr-14 text-center text-base font-medium dark:bg-muted/55 md:text-base"
                  bind:value={destinationAddress}
                  placeholder={recipientInputCopy.placeholder}
                />
                <button
                  type="button"
                  class="text-muted-foreground hover:text-foreground focus-visible:ring-ring/50 absolute top-1/2 right-2 -translate-y-1/2 rounded px-1.5 py-0.5 text-[10px] font-semibold leading-none transition-colors focus-visible:outline-none focus-visible:ring-2"
                  onclick={pasteRecipientAddress}
                  aria-label={i18n.t('wallet.transfer.recipient.paste')}
                  title={i18n.t('wallet.transfer.recipient.paste')}
                >
                  {i18n.t('wallet.transfer.recipient.paste')}
                </button>
              </div>
              {#if destinationAddress.trim() && !recipientValid}
                <p class="text-destructive text-xs">{i18n.t('wallet.transfer.recipientInvalid')}</p>
              {/if}

              <div class="mt-3 flex items-center justify-center gap-2">
                <Button
                  variant="secondary"
                  class="h-9 gap-1.5 px-4"
                  onclick={selectSelfRecipient}
                  disabled={!selfDestinationAddress}
                >
                  <UserRoundIcon class="size-4" />
                  {i18n.t('wallet.transfer.recipient.sendToSelf')}
                </Button>
                <Button variant="secondary" class="h-9 gap-1.5 px-4" onclick={() => (showAddressBookSheet = true)}>
                  <BookUserIcon class="size-4" />
                  {i18n.t('wallet.transfer.addressBook.open')}
                </Button>
              </div>

              <div class="min-h-5">
                {#if matchedSavedRecipient}
                  <p class="text-emerald-700 dark:text-emerald-300 text-xs">
                    {i18n.t('wallet.transfer.addressBook.savedMatch', {
                      contact: matchedSavedRecipient.contact.displayName,
                      endpoint: matchedSavedRecipient.endpoint.label
                    })}
                  </p>
                {:else if hasRecipientSimilarityWarning}
                  <p class="text-amber-700 dark:text-amber-300 text-xs">
                    {i18n.t('wallet.transfer.addressBook.similarWarning')}
                  </p>
                {/if}
              </div>
            </div>
          </Card.Content>
        </Card.Root>
      </div>
    {/if}

    {#if currentStep === 'review'}
      <Card.Root class="border-0 bg-transparent py-0 shadow-none">
        <Card.Content class="space-y-3 px-0 pt-0">
          {#if activePreflight}
            <div class="mx-auto w-full max-w-[430px] space-y-2">
              <div class="space-y-2">
                <div class="relative min-h-7 space-y-0.5 text-center">
                  <button
                    type="button"
                    class="text-muted-foreground hover:text-foreground focus-visible:ring-ring/50 absolute top-0 right-0 h-7 w-7 rounded-sm p-0 transition-colors focus-visible:outline-none focus-visible:ring-2"
                    onclick={() => jumpToStep('details')}
                    title={i18n.t('wallet.transfer.review.changeDetails')}
                    aria-label={i18n.t('wallet.transfer.review.changeDetails')}
                  >
                    <PencilIcon class="size-3.5" />
                  </button>
                  <p class="text-muted-foreground text-[10px] font-medium tracking-[0.05em] uppercase">
                    {i18n.t('wallet.transfer.review.sending')}
                  </p>
                  <div class="flex items-center justify-center gap-2">
                    {#if selectedCoin}
                      <CoinIcon
                        coinId={selectedCoin.id}
                        coinName={selectedCoinPresentation?.displayName}
                        size={20}
                        decorative={true}
                      />
                    {/if}
                    <p class="min-w-0 truncate text-[1.35rem] font-semibold tabular-nums">
                      {reviewSendingValue || i18n.t('wallet.transfer.summary.notSet')}
                    </p>
                  </div>
                  {#if sourceAmountFiatDisplay && sourceAmountFiatDisplay !== '≈ —'}
                    <p class="text-muted-foreground text-[11px] tabular-nums">{sourceAmountFiatDisplay}</p>
                  {/if}
                </div>

                {#if conversionEnabled}
                  <div class="text-muted-foreground/70 flex items-center justify-center">
                    <ArrowDownIcon class="size-3.5" />
                  </div>
                  <div class="space-y-0.5 text-center">
                    <p class="text-muted-foreground text-[10px] font-medium tracking-[0.05em] uppercase">
                      {i18n.t('wallet.transfer.review.receiving')}
                    </p>
                    <div class="flex items-center justify-center gap-2">
                      {#if selectedReceiveAssetOption?.destinationId}
                        <CoinIcon
                          coinId={selectedReceiveAssetOption.destinationId}
                          coinName={selectedReceiveAssetOption.label}
                          size={20}
                          decorative={true}
                        />
                      {/if}
                      <p class="min-w-0 truncate text-[1.35rem] font-semibold tabular-nums">
                        {reviewReceivingValue || i18n.t('wallet.transfer.summary.notSet')}
                      </p>
                    </div>
                    {#if receiveAmountFiatDisplay && receiveAmountFiatDisplay !== '≈ —'}
                      <p class="text-muted-foreground text-[11px] tabular-nums">{receiveAmountFiatDisplay}</p>
                    {/if}
                  </div>
                {/if}
              </div>

              <div class="space-y-1">
                <div class="space-y-1 rounded-lg bg-muted/35 px-2.5 py-2 dark:bg-muted/40">
                  <div class="flex items-start justify-between gap-3">
                    <p class="text-muted-foreground mt-0.5 text-[11px]">{i18n.t('wallet.transfer.summary.recipient')}</p>
                    <div class="flex min-w-0 items-start gap-1.5">
                      <div class="min-w-0 text-right">
                        <p class="truncate text-[13px] font-medium">
                          {reviewRecipientName || reviewRecipientAddress || i18n.t('wallet.transfer.summary.notSet')}
                        </p>
                        {#if reviewRecipientName && reviewRecipientAddress}
                          <p class="text-muted-foreground mt-0.5 font-mono text-[10px]">{reviewRecipientAddress}</p>
                        {/if}
                      </div>
                      <button
                        type="button"
                        class="text-muted-foreground hover:text-foreground focus-visible:ring-ring/50 h-7 w-7 shrink-0 rounded-sm p-0 transition-colors focus-visible:outline-none focus-visible:ring-2"
                        onclick={() => jumpToStep('recipient')}
                        title={i18n.t('wallet.transfer.review.changeRecipient')}
                        aria-label={i18n.t('wallet.transfer.review.changeRecipient')}
                      >
                        <PencilIcon class="size-3.5" />
                      </button>
                    </div>
                  </div>

                  <div class="mt-1 min-h-4">
                    {#if matchedSavedRecipient}
                      <p class="text-emerald-700 dark:text-emerald-300 text-[11px]">
                        {i18n.t('wallet.transfer.review.savedRecipient', {
                          contact: matchedSavedRecipient.contact.displayName,
                          endpoint: matchedSavedRecipient.endpoint.label
                        })}
                      </p>
                    {:else}
                      <div class="space-y-0.5">
                        <p class="text-amber-700 dark:text-amber-300 text-[11px]">
                          {i18n.t('wallet.transfer.review.unsavedRecipient')}
                        </p>
                        <p class="text-muted-foreground text-[11px]">
                          {i18n.t('wallet.transfer.review.unsavedSettingHint')}
                        </p>
                      </div>
                    {/if}
                  </div>
                </div>

                {#if conversionEnabled}
                  <div class="rounded-lg bg-muted/35 px-2.5 py-2 dark:bg-muted/40">
                    <div class="flex items-center justify-between gap-3">
                      <p class="text-muted-foreground text-[11px]">{i18n.t('wallet.transfer.summary.route')}</p>
                      <p class="text-right text-[13px] font-medium">
                        {reviewRouteValue || i18n.t('wallet.transfer.viaBest')}
                      </p>
                    </div>
                  </div>
                {/if}

                {#if reviewDestinationNetworkValue}
                  <div class="rounded-lg bg-muted/35 px-2.5 py-2 dark:bg-muted/40">
                    <div class="flex items-center justify-between gap-3">
                      <p class="text-muted-foreground text-[11px]">{i18n.t('wallet.transfer.summary.destinationNetwork')}</p>
                      <p class="text-right text-[13px] font-medium">{reviewDestinationNetworkValue}</p>
                    </div>
                  </div>
                {/if}

                <div class="space-y-1 rounded-lg bg-muted/35 px-2.5 py-2 dark:bg-muted/40">
                  <div class="flex items-start justify-between gap-3">
                    <p class="text-muted-foreground text-[11px]">{i18n.t('wallet.transfer.summary.networkFee')}</p>
                    <p class="text-[13px] font-medium tabular-nums">
                      {reviewNetworkFeeValue || i18n.t('wallet.transfer.summary.notSet')}
                    </p>
                  </div>

                  {#if conversionEnabled && conversionFeeInfo}
                    <div class="flex items-start justify-between gap-3">
                      <p class="text-muted-foreground text-[11px]">
                        {i18n.t('wallet.transfer.summary.conversionFeeWithRate', { rate: conversionFeeInfo.percentage })}
                      </p>
                      <p class="text-[13px] font-medium tabular-nums">
                        {conversionFeeInfo.amount} {conversionFeeInfo.currency}
                      </p>
                    </div>
                  {/if}

                  {#if reviewTotalFeesFiat !== '≈ —'}
                    <p class="text-muted-foreground text-right text-[11px] tabular-nums">{reviewTotalFeesFiat}</p>
                  {/if}
                </div>

                {#if estimatedArrivalInfo}
                  <div class="rounded-lg bg-muted/35 px-2.5 py-2 dark:bg-muted/40">
                    <div class="flex items-center justify-between gap-3">
                      <p class="text-muted-foreground text-[11px]">{i18n.t('wallet.transfer.summary.estimatedTime')}</p>
                      <div class="flex items-center gap-1.5">
                        <p class="text-[13px] font-medium">{estimatedArrivalInfo.value}</p>
                        <Tooltip.Root>
                          <Tooltip.Trigger
                            class="text-muted-foreground hover:text-foreground focus-visible:ring-ring/50 inline-flex h-7 w-7 items-center justify-center rounded-sm p-0 transition-colors focus-visible:outline-none focus-visible:ring-2"
                            aria-label={i18n.t('wallet.transfer.summary.estimatedTime')}
                          >
                            <InfoIcon class="size-3.5" />
                          </Tooltip.Trigger>
                          <Tooltip.Content side="top" align="end" class="max-w-72 text-xs leading-5">
                            {estimatedArrivalInfo.tooltip}
                          </Tooltip.Content>
                        </Tooltip.Root>
                      </div>
                    </div>
                  </div>
                {/if}
              </div>
            </div>
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

            {#if !isSavedRecipient}
              <div class="mt-4 rounded-md border border-border/70 p-3 text-left">
                <p class="text-sm font-medium">{i18n.t('wallet.transfer.saveRecipient.title')}</p>
                <p class="text-muted-foreground mt-1 text-xs">{i18n.t('wallet.transfer.saveRecipient.description')}</p>
                <Input
                  class="mt-3"
                  bind:value={saveRecipientName}
                  placeholder={i18n.t('wallet.transfer.saveRecipient.namePlaceholder')}
                />
                {#if saveRecipientError}
                  <p class="text-destructive mt-2 text-xs">{saveRecipientError}</p>
                {/if}
                <Button class="mt-3 h-8" onclick={saveRecipientFromSuccess} disabled={savingRecipient}>
                  {savingRecipient
                    ? i18n.t('wallet.transfer.saveRecipient.saving')
                    : i18n.t('wallet.transfer.saveRecipient.save')}
                </Button>
              </div>
            {/if}
          {/if}
        </Card.Content>
      </Card.Root>
    {/if}
  </div>
</WalletTransferStepperShell>

<StandardRightSheet bind:isOpen={showSourceAssetSheet} title={i18n.t('wallet.transfer.youSend')}>
  <div class="flex h-full min-h-0 flex-col">
    {#if positiveSendableCoinOptions.length === 0}
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.noAssets')}</p>
    {:else}
      <div class="min-h-0 flex-1 overflow-y-auto pr-1">
        <div class="space-y-2 pb-1">
          {#each positiveSendableCoinOptions as option}
            <button
              type="button"
              class="group flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors
                focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
                {selectedCoinId === option.coin.id
                  ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                  : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'}"
              onclick={() => selectSourceCoin(option.coin.id)}
            >
              <div class="flex min-w-0 items-center gap-2.5">
                <CoinIcon
                  coinId={option.coin.id}
                  coinName={option.displayName}
                  size={18}
                  decorative={true}
                />
                <div class="min-w-0">
                  <p class="truncate text-sm font-semibold">{option.displayName}</p>
                  <p class="text-muted-foreground truncate text-xs">{option.displayTicker}</p>
                </div>
              </div>
              <p class="ml-3 shrink-0 text-sm font-medium">{formatSheetBalance(option.balanceTotal)}</p>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet bind:isOpen={showAddressBookSheet} title={i18n.t('wallet.transfer.addressBook.sheetTitle')}>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <SearchInput
      bind:value={addressBookSearchTerm}
      placeholder={i18n.t('wallet.transfer.addressBook.searchPlaceholder')}
      inputClass="h-10"
    />

    {#if addressBookEndpointOptions.length === 0}
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.addressBook.empty')}</p>
    {:else}
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full pr-1">
          <div class="space-y-2 pb-1">
            {#each addressBookEndpointOptions as option}
              <button
                type="button"
                class="flex w-full items-start justify-between rounded-lg bg-muted/65 px-3.5 py-3 text-left transition-colors
                  hover:bg-muted/70 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring/60
                  dark:bg-muted/55 dark:hover:bg-muted/65"
                onclick={() => selectAddressBookRecipient(option)}
              >
                <div class="min-w-0">
                  <p class="truncate text-[15px] leading-tight font-medium">{option.contactName}</p>
                  <p class="text-muted-foreground mt-0.5 flex items-center gap-1.5 truncate text-sm">
                    <span
                      class="bg-background/60 text-muted-foreground inline-flex shrink-0 rounded-full px-2.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide dark:bg-background/45"
                    >
                      {endpointBadgeLabel(option.endpointKind)}
                    </span>
                    <span class="truncate">{shortRecipientAddress(option.endpointAddress)}</span>
                  </p>
                </div>
                {#if option.lastUsedAt}
                  <p class="text-muted-foreground ml-3 mt-0.5 text-xs">
                    {i18n.t('wallet.transfer.addressBook.recent')}
                  </p>
                {/if}
              </button>
            {/each}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet bind:isOpen={showReceiveAssetSheet} title={i18n.t('wallet.transfer.receiveSheetTitle')}>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <SearchInput
      bind:value={receiveSearchTerm}
      placeholder={i18n.t('wallet.transfer.receiveSearchPlaceholder')}
      inputClass="h-10"
    />

    {#if targetsError}
      <p class="text-destructive text-sm">{targetsError}</p>
    {/if}

    {#if !loadingTargets && rawReceiveAssetSections.allOptions.length === 0}
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.viaNoOptions')}</p>
    {:else if !loadingTargets && receiveAssetOptions.length === 0}
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.routeNoMatches')}</p>
    {:else}
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full pr-1">
          <div class="space-y-4 pb-1">
            {#if popularReceiveAssetOptions.length > 0}
              <section class="space-y-2">
                <p class="text-muted-foreground px-1 text-xs font-semibold">
                  {i18n.t('wallet.transfer.routeGroupPopular')}
                </p>
                {#each popularReceiveAssetOptions as option}
                  {@const display = getReceiveOptionDisplay(option)}
                  <button
                    type="button"
                    class="group flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors
                      focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
                      {isReceiveOptionSelected(option)
                        ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                        : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'}"
                    onclick={() => selectReceiveAsset(option.id)}
                  >
                    <div class="flex min-w-0 items-center gap-2.5">
                      <CoinIcon
                        coinId={option.destinationId}
                        coinName={display.primary}
                        size={18}
                        decorative={true}
                      />
                      <div class="min-w-0">
                        <p class="truncate text-sm font-semibold">{display.primary}</p>
                        {#if display.secondary}
                          <p class="text-muted-foreground truncate text-xs">{display.secondary}</p>
                        {/if}
                      </div>
                    </div>
                  </button>
                {/each}
              </section>
            {/if}

            {#if otherReceiveAssetOptions.length > 0}
              <section class="space-y-2">
                <p class="text-muted-foreground px-1 text-xs font-semibold">
                  {popularReceiveAssetOptions.length > 0
                    ? i18n.t('wallet.transfer.routeGroupMore')
                    : i18n.t('wallet.transfer.routeGroupConversions')}
                </p>
                {#each otherReceiveAssetOptions as option}
                  {@const display = getReceiveOptionDisplay(option)}
                  <button
                    type="button"
                    class="group flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors
                      focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
                      {isReceiveOptionSelected(option)
                        ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                        : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'}"
                    onclick={() => selectReceiveAsset(option.id)}
                  >
                    <div class="flex min-w-0 items-center gap-2.5">
                      <CoinIcon
                        coinId={option.destinationId}
                        coinName={display.primary}
                        size={18}
                        decorative={true}
                      />
                      <div class="min-w-0">
                        <p class="truncate text-sm font-semibold">{display.primary}</p>
                        {#if display.secondary}
                          <p class="text-muted-foreground truncate text-xs">{display.secondary}</p>
                        {/if}
                      </div>
                    </div>
                  </button>
                {/each}
              </section>
            {/if}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet bind:isOpen={showNetworkSheet} title={i18n.t('wallet.transfer.networkSheetTitle')}>
  <div class="flex h-full min-h-0 flex-col gap-3">
    {#if pendingGroupedReceiveOption}
      <p class="text-muted-foreground text-sm">
        {i18n.t('wallet.transfer.networkSheetDescription', { value: pendingGroupedReceiveOption.label })}
      </p>
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full pr-1">
          <div class="space-y-2 pb-1">
            {#each pendingGroupedReceiveOption.networkOptions ?? [] as option}
              <button
                type="button"
                class="flex w-full items-start justify-between rounded-md border px-3 py-2 text-left transition-colors
                  border-border/60 hover:bg-muted/30"
                onclick={() => selectReceiveNetworkOption(option.id)}
              >
                <div class="flex min-w-0 items-start gap-2">
                  <CoinIcon coinId={option.destinationId} coinName={option.label} size={18} decorative={true} />
                  <div class="min-w-0">
                    <div class="flex items-center gap-2">
                      <p class="truncate text-sm font-medium">{option.label}</p>
                      {#if option.hasOnChainPath}
                        <span class="rounded-full border border-border px-2 py-0.5 text-[10px] font-semibold">
                          {i18n.t('wallet.transfer.sameNetwork')}
                        </span>
                      {/if}
                    </div>
                    <p class="text-muted-foreground truncate text-xs">
                      {i18n.t('wallet.transfer.receiveAs', {
                        value: option.fullyqualifiedname || option.ticker || option.label
                      })}
                    </p>
                  </div>
                </div>
                <ChevronRightIcon class="text-foreground/45 ml-3 size-4 shrink-0" />
              </button>
            {/each}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    {:else}
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.viaNoOptions')}</p>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet bind:isOpen={showExportSheet} title={i18n.t('wallet.transfer.exportSheetTitle')}>
  <div class="flex h-full min-h-0 flex-col gap-3">
    {#if pendingTargetOption}
      <p class="text-muted-foreground text-sm">
        {#if !pendingTargetOption.hasOnChainPath && pendingTargetOption.exportOptions.length === 1}
          {i18n.t('wallet.transfer.onlyAvailableOnNetwork', {
            value: pendingTargetOption.exportOptions[0].exportToName
          })}
        {:else}
          {i18n.t('wallet.transfer.exportSheetDescription', { value: pendingTargetOption.label })}
        {/if}
      </p>
      <ScrollArea.Root class="min-h-0 flex-1">
        <ScrollArea.Viewport class="h-full pr-1">
          <div class="space-y-2 pb-1">
            {#if pendingTargetOption.hasOnChainPath}
              <button
                type="button"
                class="flex w-full items-start justify-between rounded-md border px-3 py-2 text-left transition-colors
                  {selectedReceiveAssetId === pendingTargetOption.id && selectedExportSystemId === null
                    ? 'border-primary/70 bg-primary/5'
                    : 'border-border/60 hover:bg-muted/30'}"
                onclick={selectSameNetworkOption}
              >
                <div class="flex min-w-0 items-start gap-2">
                  <CoinIcon
                    coinId={selectedCoin?.id || pendingTargetOption.destinationId}
                    coinName={pendingTargetOption.label}
                    size={18}
                    decorative={true}
                  />
                  <div class="min-w-0">
                    <p class="truncate text-sm font-medium">{i18n.t('wallet.transfer.sameNetwork')}</p>
                    <p class="text-muted-foreground truncate text-xs">
                      {i18n.t('wallet.transfer.receiveAs', {
                        value:
                          pendingTargetOption.fullyqualifiedname ||
                          pendingTargetOption.ticker ||
                          pendingTargetOption.label
                      })}
                    </p>
                  </div>
                </div>
              </button>
            {/if}

            {#each pendingTargetOption.exportOptions as option}
              <button
                type="button"
                class="flex w-full items-start justify-between rounded-md border px-3 py-2 text-left transition-colors
                  {selectedReceiveAssetId === pendingTargetOption.id &&
                  selectedExportSystemId === option.exportTo
                    ? 'border-primary/70 bg-primary/5'
                    : 'border-border/60 hover:bg-muted/30'}"
                onclick={() => selectExportOption(option.exportTo)}
              >
                <div class="flex min-w-0 items-start gap-2">
                  <CoinIcon
                    coinId={option.exportTo}
                    coinName={option.exportToName}
                    size={18}
                    decorative={true}
                  />
                  <div class="min-w-0">
                    <p class="truncate text-sm font-medium">{option.exportToName}</p>
                    <p class="text-muted-foreground truncate text-xs">
                      {i18n.t('wallet.transfer.receiveAs', {
                        value:
                          pendingTargetOption.fullyqualifiedname ||
                          pendingTargetOption.ticker ||
                          pendingTargetOption.label
                      })}
                    </p>
                  </div>
                </div>
                <ChevronRightIcon class="text-foreground/45 ml-3 size-4 shrink-0" />
              </button>
            {/each}
          </div>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar orientation="vertical" />
      </ScrollArea.Root>
    {:else}
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.viaNoOptions')}</p>
    {/if}
  </div>
</StandardRightSheet>

<StandardRightSheet bind:isOpen={showViaSheet} title={i18n.t('wallet.transfer.viaSheetTitle')}>
  <div class="space-y-3">
    {#if selectedReceiveAssetOption}
      {#if rankedViaOptions.length === 0}
        <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.viaNoOptions')}</p>
      {:else}
        <div class="space-y-2">
          {#each rankedViaOptions as option}
            {@const estimatedValue = formatEstimatedReceive(option)}
            {@const routeSubtitle = getViaSheetSubtitle(option)}
            <button
              type="button"
              class="group flex w-full items-center justify-between rounded-lg p-3 text-left transition-colors
                focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2
                {selectedViaOptionId === option.id
                  ? 'bg-primary/14 hover:bg-primary/20 dark:bg-primary/28 dark:hover:bg-primary/36'
                  : 'bg-muted/65 hover:bg-muted/70 dark:bg-muted/55 dark:hover:bg-muted/65'}"
              onclick={() => selectViaOption(option.id)}
            >
              <div class="flex min-w-0 items-start gap-2.5">
                <CoinIcon
                  coinId={option.via ?? option.id}
                  coinName={getViaOptionLabel(option)}
                  size={18}
                  decorative={true}
                />
                <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <p class="truncate text-sm font-semibold">{getViaOptionLabel(option)}</p>
                    {#if bestViaOption?.id === option.id}
                      <span
                        class="rounded-full bg-emerald-500/12 px-2 py-0.5 text-[10px] font-semibold text-emerald-700 dark:text-emerald-300"
                      >
                        {i18n.t('wallet.transfer.viaBest')}
                      </span>
                    {/if}
                  </div>
                  {#if routeSubtitle}
                    <p class="text-muted-foreground truncate text-xs">{routeSubtitle}</p>
                  {/if}
                </div>
              </div>
              {#if estimatedValue}
                <div class="ml-3 min-w-0 text-right">
                  <p class="text-muted-foreground inline-flex items-center gap-1 text-[11px] font-medium">
                    <ArrowDownIcon class="size-3" />
                    <span>{i18n.t('wallet.transfer.estimatedLabel')}</span>
                  </p>
                  <p class="text-foreground text-lg font-semibold leading-none tabular-nums">{estimatedValue}</p>
                </div>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    {:else}
      <p class="text-muted-foreground text-sm">{i18n.t('wallet.transfer.viaNoOptions')}</p>
    {/if}
  </div>
</StandardRightSheet>
