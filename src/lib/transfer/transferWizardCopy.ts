import type { TranslationParams } from '$lib/i18n';
import type {
  DestinationAddressKind,
  WizardOperationalStepId,
} from '$lib/components/wallet/sections/transfer-wizard/types';

type TranslateFn = (key: string, params?: TranslationParams) => string;

export function getTransferStepLabels(t: TranslateFn): Record<WizardOperationalStepId, string> {
  return {
    details: t('wallet.transfer.stepLabel.details'),
    recipient: t('wallet.transfer.stepLabel.recipient'),
    review: t('wallet.transfer.stepLabel.review'),
  };
}

export function getTransferStepCopy(
  t: TranslateFn
): Record<WizardOperationalStepId, { title: string; description: string }> {
  return {
    details: {
      title: t('wallet.transfer.step.details.title'),
      description: t('wallet.transfer.step.details.description'),
    },
    recipient: {
      title: t('wallet.transfer.step.recipient.title'),
      description: t('wallet.transfer.step.recipient.description'),
    },
    review: {
      title: t('wallet.transfer.step.review.title'),
      description: t('wallet.transfer.step.review.description'),
    },
  };
}

export function getRecipientInputCopy(
  t: TranslateFn,
  kind: DestinationAddressKind
): { placeholder: string; hint: string } {
  if (kind === 'eth') {
    return {
      placeholder: t('wallet.transfer.recipientPlaceholderEth'),
      hint: t('wallet.transfer.recipientHintEth'),
    };
  }

  if (kind === 'btc') {
    return {
      placeholder: t('wallet.transfer.recipientPlaceholderBtc'),
      hint: t('wallet.transfer.recipientHintBtc'),
    };
  }

  if (kind === 'dlight') {
    return {
      placeholder: t('wallet.transfer.recipientPlaceholderDlight'),
      hint: t('wallet.transfer.recipientHintDlight'),
    };
  }

  return {
    placeholder: t('wallet.transfer.recipientPlaceholderVrpc'),
    hint: t('wallet.transfer.recipientHintVrpc'),
  };
}

export function getTransferSummaryLabels(t: TranslateFn) {
  return {
    title: t('wallet.transfer.summary.title'),
    from: t('wallet.transfer.summary.from'),
    to: t('wallet.transfer.summary.to'),
    route: t('wallet.transfer.summary.route'),
    amount: t('wallet.transfer.summary.amount'),
    recipient: t('wallet.transfer.summary.recipient'),
    estimatedReceive: t('wallet.transfer.summary.estimatedReceive'),
    networkFee: t('wallet.transfer.summary.networkFee'),
    warnings: t('wallet.transfer.warningsTitle'),
    notSet: t('wallet.transfer.summary.notSet'),
  };
}
