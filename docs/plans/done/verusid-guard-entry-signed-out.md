---
owner: lite-wallet-team
last_reviewed: 2026-02-12
---

# Completed plan: VerusID guard signed-out entry and flows

## Final status

Completed.

## Date completed

2026-02-12

## What shipped

- Signed-out dock entry (`VerusID Guard`) on both welcome and unlock screens.
- Right-side choice sheet with `Revoke VerusID` and `Recover VerusID`.
- Full-screen signed-out guard flows for revoke and recover.
- Recover advanced optional patch fields (recovery/revocation/private address).
- Guard session lifecycle cleanup (`end_guard_session`) on close/unmount/finish.
- EN/NL translation coverage for all new guard UX strings.
- Theme-safe dock styling for both light and dark mode.

## Core files

- `src/lib/components/wallet/VerusIdGuardDock.svelte`
- `src/lib/components/wallet/VerusIdGuardSheet.svelte`
- `src/lib/components/flows/VerusIdGuard/GuardFlowHost.svelte`
- `src/lib/components/flows/VerusIdGuard/GuardSecretStep.svelte`
- `src/lib/components/flows/VerusIdGuard/GuardTargetStep.svelte`
- `src/lib/components/flows/VerusIdGuard/GuardRecoverPatchStep.svelte`
- `src/lib/components/flows/VerusIdGuard/GuardReviewStep.svelte`
- `src/lib/components/flows/VerusIdGuard/GuardResultStep.svelte`
- `src/lib/components/wallet/WelcomeScreen.svelte`
- `src/lib/components/wallet/UnlockScreen.svelte`
- `src/app.css`
- `src/lib/i18n/locales/en.ts`
- `src/lib/i18n/locales/nl.ts`

## Validation notes

- `yarn check` passed after implementation.
- Manual visual QA and testnet transaction QA should be run before release cut.

## Follow-up items

- Add automated UI tests for guard flow transitions and cleanup.
- Add a signed-in access path if product scope expands.
