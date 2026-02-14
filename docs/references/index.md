---
owner: lite-wallet-team
last_reviewed: 2026-02-12
---

# References index

External references and source repos used for parity or implementation checks.

## Repositories

- `valu-mobile` parity source: `/Users/maxtheyse/dev/valu-mobile` (branch:
  `newsend2`)
- Rust identity signing parity report:
  [`./verus-identity-rust-parity.md`](./verus-identity-rust-parity.md)
- Valu coin catalog parity and sync workflow:
  [`./valu-coin-catalog-parity.md`](./valu-coin-catalog-parity.md)

## Notes

- Prefer primary source code over stale copied notes.
- When parity behavior changes, update both references and local product-spec
  docs.
- Regenerate coin catalog artifacts with `yarn sync:valu-coins`.
- Verify catalog parity and drift with `yarn check:valu-coins`.
