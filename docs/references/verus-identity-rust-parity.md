---
owner: lite-wallet-team
last_reviewed: 2026-02-12
---

# Verus identity Rust parity report

This report tracks testnet parity scenarios for the Rust-native identity signing
engine.

## Scenario matrix

| Scenario ID | Operation | Mode | Status | Txid | Notes |
| --- | --- | --- | --- | --- | --- |
| ID-RUST-UPDATE-001 | update | signed-in | pending | - | Pending live testnet execution |
| ID-RUST-REVOKE-002 | revoke | signed-in | pending | - | Pending live testnet execution |
| ID-RUST-RECOVER-003 | recover (primary only) | signed-in | pending | - | Pending live testnet execution |
| ID-RUST-RECOVER-004 | recover (advanced patch) | signed-in | pending | - | Pending live testnet execution |
| ID-RUST-GUARD-REVOKE-005 | revoke | guard | pending | - | Pending live testnet execution |
| ID-RUST-GUARD-RECOVER-006 | recover (primary patch) | guard | broadcasted (awaiting confirmation) | a9a6980295946087131cefa796cdc93301b64479fe2ad0ce8d889993681bcae6 | 2026-02-12: `tryrevoke2@` with recovery authority `player6@`, patch set primary to `RBSLspANisgy5PJFKvKTdcWaArCUduLXTd` |

## Notes

- Unit vectors for codec/sighash/smart-signature encoding are in
  `src-tauri/src/core/channels/vrpc/identity/fixtures/verus_identity_vectors.json`.
- For recover/revoke flows, broadcast success (txid returned) may happen before
  `getidentity` reflects the new status. Confirmation is required for identity
  state transitions to appear in `getidentity`.
- The tx above was verified with `getrawtransaction(..., 1)` and includes the
  expected `identityprimary.primaryaddresses` patch in the identity output.
