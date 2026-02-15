# Desktop transfer wizard redesign v2

## Scope

- Move to 3 operational steps: `details`, `recipient`, `review`.
- Keep `success` as terminal state outside the operational step count.
- Put amount entry in the first step (`You send`).
- For convert mode, show a second card (`You receive`) in the first step.
- Keep right sheets for source asset, receive asset, and via selection.
- Keep desktop summary rail + mobile summary sheet fallback.

## Implemented architecture

- `TransferWizard.svelte`
  - Reworked to step IDs: `details | recipient | review | success`.
  - Removed memo from UI and preflight payload usage.
  - Added conversion toggle flow for Send entry (`Do conversion`).
  - Added route normalization and deduped receive assets.
  - Added best-via auto selection + manual via lock and reset.
- `transfer-wizard/types.ts`
  - Updated `TransferStepId` to new step model.
- `transferWizardCopy.ts`
  - Updated step copy and labels to details-first model.
- `TransferSummaryRail.svelte`
  - Added `To` summary field.
  - Keeps warnings hidden when none.

## Route model behavior

- Bridge paths are normalized into via options.
- Receive-asset sheet dedupes by receive key (`convertTo ?? destinationId`).
- Selecting a receive asset picks best via for current amount.
- Via sheet shows all variants for selected receive asset with estimated
  receive.
- Manual via selection is sticky until reset or invalidated by upstream changes.

## State and safety rules

- Preflight is invalidated whenever source, conversion mode, receive asset, via,
  amount, or recipient changes.
- Send action uses current `activePreflight.preflightId` only.
- Backend transaction and preflight APIs are unchanged.

## Localization updates

- Added v2 keys for details-first flow:
  - step labels and titles
  - `You send` / `You receive`
  - conversion action
  - receive and via sheet labels/actions
  - review `Change details`
  - via rate and estimated text
- Updated both locale files:
  - `src/lib/i18n/locales/en.ts`
  - `src/lib/i18n/locales/nl.ts`
