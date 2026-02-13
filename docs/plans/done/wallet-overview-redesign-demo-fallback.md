---
owner: lite-wallet-team
last_reviewed: 2026-02-13
---

# Completed plan: Wallet overview redesign with dev demo fallback

## Final status

Completed.

## Date completed

2026-02-13

## What shipped

- Replaced the wallet overview right panel with a simplified layout:
  - large fiat hero value (top left),
  - primary VRSC/VRSCTEST amount under it,
  - equal `Receive`, `Send`, and `Convert` buttons on the right.
- Replaced the overview transaction block with a currency list view showing:
  - fiat value,
  - crypto amount,
  - optional unit fiat rate.
- Added live overview view-model helper:
  - `src/lib/utils/walletOverview.ts`
- Added dev-only demo snapshot helper for empty wallet states:
  - `src/lib/utils/walletOverviewDemo.ts`
- Added development-only fallback behavior:
  - use demo data only when no usable live data exists,
  - live-only rendering once balances are present,
  - no live/demo row mixing.
- Wired `Convert` action to the `Conversions` section from overview.
- Added locale key coverage for overview convert action in EN/NL.

## Core files

- `src/lib/components/wallet/sections/Overview.svelte`
- `src/lib/components/wallet/WalletLayout.svelte`
- `src/lib/utils/walletOverview.ts`
- `src/lib/utils/walletOverviewDemo.ts`
- `src/lib/i18n/locales/en.ts`
- `src/lib/i18n/locales/nl.ts`

## Validation notes

- `yarn lint`
- `yarn check`

## Follow-up items

- Replace placeholder/stub fiat rates with live backend rates to remove `—` in production wallets.
- Add per-asset detail navigation once wallet asset detail screens are available.
