export type TransferStepId = 'details' | 'recipient' | 'review' | 'success';

export type WizardOperationalStepId = Exclude<TransferStepId, 'success'>;

export type StepStatus = 'complete' | 'current' | 'upcoming';

export type TransferStepperStep = {
  id: WizardOperationalStepId;
  label: string;
  status: StepStatus;
};

export type DestinationAddressKind = 'vrpc' | 'btc' | 'eth';
