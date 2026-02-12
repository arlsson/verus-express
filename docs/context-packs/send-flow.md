---
owner: lite-wallet-team
last_reviewed: 2026-02-12
---

# Context pack: send flow

Read this before touching transaction send behavior.

## Invariants

- UI sends preflight params only.
- Send uses `preflight_id` only.
- Preflight payload is backend-owned and session-scoped.
- Preflight records are single-use and must expire/clear correctly.

## Open these files first

- `src/lib/components/wallet/sections/Send.svelte`
- `src/lib/machines/txMachine.ts`
- `src/lib/services/txService.ts`
- `src-tauri/src/commands/transaction.rs`
- `src-tauri/src/core/channels/mod.rs`
- `src-tauri/src/core/channels/store.rs`
- `src-tauri/src/core/channels/vrpc/preflight.rs`
- `src-tauri/src/core/channels/vrpc/send.rs`
- `src-tauri/src/core/channels/btc/preflight.rs`
- `src-tauri/src/core/channels/btc/send.rs`

## Verification checklist

- Confirm flow transitions still cover
  `idle -> preflighting -> confirming -> sending -> success|error`.
- Confirm no UI pathway can submit tx hex/signing material.
- Confirm lock still clears preflight state.
- Confirm user-facing errors remain safe.
