---
owner: lite-wallet-team
last_reviewed: 2026-02-21
---

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
  - Memo is shown conditionally for `dlight_private` routes when destination is shielded (`zs`).
  - dlight recipient validation uses route-aware policy (`zs | R | i` only).
  - Added conversion toggle flow for Send entry (`Do conversion`).
  - Added route normalization and deduped receive assets.
  - Added best-via auto selection + manual via lock and reset.
  - Added SendWizard parity shaping for receive assets (prelaunch filtering,
    canonical grouping, popular/more sections, and search).
  - Added network + destination-network picker sheets for grouped/cross-chain
    receive selection.
- `transfer-wizard/types.ts`
  - Updated `TransferStepId` to new step model.
- `transferWizardCopy.ts`
  - Updated step copy and labels to details-first model.
- `TransferSummaryRail.svelte`
  - Added `To` summary field.
  - Keeps warnings hidden when none.

## Route model behavior

- Bridge paths are normalized into via options.
- Receive-asset sheet uses mobile parity shaping from
  `valu-mobile` branch `newsend3`:
  - prelaunch destinations hidden
  - options grouped by canonical asset keys
  - fixed popular ordering (`VRSC`, `USDC`, `ETH`, `TBTC`, `DAI`)
  - searchable popular + more sections
- Selecting a receive asset picks best via for current amount.
- Grouped receive assets open a network picker before final target selection.
- Cross-chain targets open a destination-network picker; same-network can be
  hidden when no on-chain route exists.
- Via sheet shows all variants for selected receive asset with estimated
  receive.
- Manual via selection is sticky until reset or invalidated by upstream changes.

## State and safety rules

- Preflight is invalidated whenever source, conversion mode, receive asset, via,
  amount, or recipient changes.
- Send action uses current `activePreflight.preflightId` only.
- Backend transaction and preflight APIs are unchanged.
- For `dlight_private`, backend preflight/send now route through the spend engine
  and preserve the same IPC contract (`preflight_send`, `send_transaction`).

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
