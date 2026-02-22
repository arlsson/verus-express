---
owner: lite-wallet-team
last_reviewed: 2026-02-22
---

# Settings scope (desktop)

This spec defines what belongs in the wallet `Settings` screen for lite-wallet
desktop and what does not.

## Scope decisions (explicit)

- Coin add/disable controls are out of scope for `Settings`.
- Coin add/disable remains in the `Wallet` screen flow as the single source of
  truth.
- Fiat display currency selection is required in `Settings` and must be easy to
  find.

## Goals

- Keep settings focused on account-level and app-level preferences.
- Keep one primary job per screen and avoid duplicate controls across screens.
- Make fiat display currency changes fast and predictable (for example:
  `USD`/`EUR`/`GBP`).

## Settings information architecture

### 1) Display and currency

In scope:

- **Display currency** selector (required).
- Selector supports fiat codes from app-supported fiat catalog.
- Current selection is always visible at rest (not hidden behind advanced
  disclosure).

Discoverability requirements:

- Category appears first in `Settings`.
- Display currency appears as the first interactive control in this category.
- User can reach and change display currency in two interactions or fewer after
  entering `Settings`.

Notes:

- `USD` default is acceptable for first run, but user choice must persist.
- Portfolio fiat value rendering should consistently follow this setting across
  overview/detail screens.

### 2) Security and private wallet

In scope:

- `Private Verus` setup and status controls (existing behavior).
- Setup paths: reuse primary seed, create new seed, import private seed/spending
  key.

Out of scope in this category (for now):

- Biometric toggle.
- Key derivation version switch.
- Profile deletion/reset actions.

### 3) Advanced wallet configuration

In scope now:

- No required v1 controls.

Deferred (advanced-only candidates, not default Settings surface):

- Custom VRPC endpoint overrides.
- Address blocklist source/manual editing.
- User-configurable ETH minimum gas floor.
- Per-coin verification-level controls.

## About and support

In scope:

- App version/build metadata.
- Useful links (privacy/license/support/community) if available in existing app
  nav model.

## Explicitly out of Settings

- Coin add/disable and asset catalog management (owner screen: `Wallet`
  overview/add-asset flow).
- Per-asset operational controls that already belong to asset/wallet flows.

## Implementation guardrails

- Avoid duplicate ownership: if a control already exists in a stronger task
  context (for example asset management in wallet overview), do not re-home it
  into `Settings`.
- Keep copy short and sentence case.
- Use translation keys for all user-facing text.
- Verify behavior in light and dark mode.

## Acceptance criteria for this scope

- `Settings` includes a first-class display currency selector that is easy to
  locate.
- User can switch `USD` to another fiat (for example `EUR` or `GBP`) from
  `Settings` without navigating to asset management.
- `Settings` does not include coin add/disable controls.
- `Private Verus` controls remain available in `Settings`.

## Cross-reference

- Current `Settings` UI: `src/lib/components/wallet/sections/Settings.svelte`
- Wallet route and active assets loading: `src/routes/wallet/+page.svelte`
- Current settings store placeholder: `src/lib/stores/settings.ts`
