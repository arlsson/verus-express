---
owner: lite-wallet-team
last_reviewed: 2026-02-14
---

# Completed plan: Fiat rates via CoinPaprika + PBaaS derivation

## Final status

Completed.

## Date completed

2026-02-14

## What shipped

- Added optional coin metadata field for CoinPaprika slug overrides:
  - `coin_paprika_id` / `coinPaprikaId` on `CoinDefinition`.
- Added rates domain module:
  - direct CoinPaprika USD fetch (`ohlcv/latest` close),
  - ECB FX parsing and USD normalization,
  - PBaaS fallback derivation from `bestcurrencystate.currencies[*].lastconversionprice`.
- Integrated rates polling into update engine:
  - refresh cadence aligned to `RATES_REFRESH_SECS`,
  - per-coin `wallet://rates-updated` emission,
  - in-memory latest rates cache used as PBaaS reserve anchors.
- Kept rates failures non-fatal and non-user-blocking.
- Added wallet lifecycle hygiene:
  - clear `ratesStore` on wallet mount/unmount.
- Updated architecture docs with new fiat rates module entrypoint.

## Core files

- `src-tauri/src/core/rates/mod.rs`
- `src-tauri/src/core/rates/coinpaprika.rs`
- `src-tauri/src/core/rates/ecb.rs`
- `src-tauri/src/core/rates/pbaas.rs`
- `src-tauri/src/core/updates/engine.rs`
- `src-tauri/src/core/coins/types.rs`
- `src-tauri/src/core/coins/registry.rs`
- `src/lib/types/wallet.ts`
- `src/routes/wallet/+page.svelte`

## Validation notes

- `cargo test` (pass)
- `yarn check` (pass)
- `yarn lint` (fails due pre-existing unrelated lint errors in existing files)

## Follow-up items

- Add per-coin source metadata plumbing (currently source URL is internal only).
- Consider persisting last successful rates across app restarts.
- Add per-system VRPC endpoint routing for PBaaS derivation if/when dynamic systems need it.
