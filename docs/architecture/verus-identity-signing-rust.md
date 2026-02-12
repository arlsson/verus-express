---
owner: lite-wallet-team
last_reviewed: 2026-02-12
---

# Verus identity signing (Rust native)

This backend path signs `update/revoke/recover` identity transactions without JS
runtime signing.

## Scope

- Commands unchanged:
  - `preflight_identity_update`
  - `send_identity_update`
  - `preflight_guard_identity_update`
  - `send_guard_identity_update`
- Signed-in and guard flows share the same Rust transaction/sighash/signing path.
- Transparent Overwinter/Sapling transactions only.

## Backend flow

1. Preflight resolves target identity and authority checks.
2. Preflight fetches funding UTXOs early and returns `InsufficientFunds` when fee
   coverage is impossible.
3. `updateidentity(..., true)` template is decoded through Rust Verus codec.
4. Identity input is verified, funding inputs are appended, and optional change
   output is added.
5. Internal preflight payload stores full signing metadata for every signable
   input:
   - input index
   - prevout script
   - prevout value
   - sign mode (`p2pkh` or `smart_transaction`)
6. Send path signs each signable input with Zcash-style transparent sighash and
   input-specific scriptSig encoding.
7. Signed tx hex is broadcast through `sendrawtransaction`.

## Module map

- `src-tauri/src/core/channels/vrpc/identity/verus_tx/model.rs`
- `src-tauri/src/core/channels/vrpc/identity/verus_tx/codec.rs`
- `src-tauri/src/core/channels/vrpc/identity/verus_tx/sighash.rs`
- `src-tauri/src/core/channels/vrpc/identity/verus_tx/script.rs`
- `src-tauri/src/core/channels/vrpc/identity/verus_tx/smart_sig.rs`

## Locked consensus defaults

- Version `3` branch id: `0x5ba81b19`
- Version `4` branch id: `0x76b809bb`
- Sapling group id: `0x892f2085`

## Out of scope

- Shielded spends/outputs and joinsplits.
- Multisig identity authorities (`minimumsignatures > 1`).
