---
owner: lite-wallet-team
last_reviewed: 2026-02-26
---

# Settings scope (desktop)

This spec defines the `Settings` information architecture and ownership for
lite-wallet desktop.

## Scope decisions (explicit)

1. `Settings` uses a **hub + detail** structure.
2. Coin add/disable controls are out of scope for `Settings`; they stay in the
   `Wallet` screen flow.
3. Fiat display currency and language are required, first-class controls in
   `Settings`.
4. First-launch language gate remains in place when no wallet exists.
5. Recovery scope is account-level **seed + derived keys** (not per-scope or
   per-address bulk export).
6. Recovery lives under **Profile & security**.
7. Sensitive recovery reveal is gated by password re-check on each access.
8. Private Verus setup actions are hidden by default when already configured,
   but reconfiguration remains available in advanced detail.

## Goals

- Keep settings focused on account-level and app-level controls.
- Keep one primary task per detail page.
- Improve discoverability with clear category rows and summaries.
- Preserve valu-mobile parity intent for recovery while keeping desktop security
  guardrails explicit.

## Settings information architecture

### 1) Settings hub (home)

In scope:

- Category rows with drill-down navigation and summaries:
  - Display and language (summary: selected fiat + language).
  - Profile & security (summary: recovery and keys).
  - Private Verus (summary: configured/not configured).
  - About and support (summary: app version).

Design constraints:

- Desktop-first list layout with clear click targets.
- Consistent back/header pattern in each detail page.
- Keep detail views focused; avoid mixed multi-purpose forms.

### 2) Display and language detail

In scope:

- Display currency selector with quick picks, search, and full list.
- Language selector using the same locale set as onboarding.
- Immediate app-wide language update and persistent storage.
- Persistent display currency used consistently across overview/details/send.

### 3) Profile & security detail

In scope:

- Entry row to **Recovery and keys** detail.
- Minimal v1 surface (no extra toggles added in this iteration).

### 4) Recovery and keys detail

In scope:

- Password-gated reveal flow with re-authentication for each access.
- Seed/primary secret and derived keys for account-level addresses.
- Optional dlight recovery material only when Private Verus is configured.
- Per-item reveal/hide and copy actions.
- Clear in-memory secret data when leaving the screen.

Explicit v1 exclusions:

- QR rendering for secrets.
- Per-scope/per-address private key export across all derived variants.
- Biometric gating.

### 5) Private Verus detail

In scope:

- Status-first screen.
- When configured:
  - Show configured status and full shielded address.
  - Hide setup actions by default.
  - Show advanced disclosure before reconfiguration actions.
- When not configured:
  - Show setup actions directly (reuse primary, create new, import).

### 6) About and support

In scope:

- App version/build metadata.
- Existing support/community links.

## Explicitly out of Settings

- Coin add/disable and asset catalog management.
- Per-asset controls that belong to wallet/asset flows.
- Runtime endpoint or blockchain provider configuration.
- Any background export/backup automation for secrets in v1.

## Implementation guardrails

- Avoid duplicate ownership across Wallet and Settings.
- Keep copy short and sentence case.
- Use translation keys for all user-facing text.
- Verify behavior in light and dark mode.
- Never persist recovery secrets in localStorage or logs.

## Acceptance criteria for this scope

1. `Settings` opens to a category hub (not a long mixed control page).
2. Each category row navigates to a dedicated detail screen and back.
3. Display currency and language are both editable in Display and language.
4. Currency and language changes persist and apply across the app.
5. Private Verus configured state shows status + full shielded address and hides
   setup actions by default.
6. Recovery and keys requires password confirmation each reveal access.
7. Recovery secret material is cleared when leaving recovery detail.
8. Coin add/disable does not appear in `Settings`.

## Cross-reference

- Current `Settings` UI: `src/lib/components/wallet/sections/Settings.svelte`
- Settings detail components: `src/lib/components/wallet/settings/`
- Wallet route and active assets loading: `src/routes/wallet/+page.svelte`
- Current settings store: `src/lib/stores/settings.ts`
