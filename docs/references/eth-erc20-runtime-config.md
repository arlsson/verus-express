---
owner: lite-wallet-team
last_reviewed: 2026-02-14
---

# ETH/ERC20 runtime config and parity notes

## Required runtime environment variables

- `INFURA_PROJECT_ID`
- `ETHERSCAN_API_KEY`

These are read at backend startup in `src-tauri/src/core/channels/eth/config.rs`.
Keys are not read from frontend state and are not persisted in repository files.
For desktop development, backend startup also attempts to load `.env` and `.env.local`
from both workspace root and `src-tauri/` into process env before config evaluation.

## Optional override variables

- `ETH_MAINNET_RPC_URL`
- `ETH_TESTNET_RPC_URL`
- `ETHERSCAN_MAINNET_URL`
- `ETHERSCAN_TESTNET_URL`

Default behavior without overrides:

- Mainnet RPC: `https://mainnet.infura.io/v3/{INFURA_PROJECT_ID}`
- Testnet RPC: `https://goerli.infura.io/v3/{INFURA_PROJECT_ID}`
- Etherscan mainnet: `https://api.etherscan.io/api`
- Etherscan testnet: `https://api-goerli.etherscan.io/api`

## Startup validation behavior

- If required env vars are missing or empty, ETH provider pool stays disabled.
- If configured URLs are invalid, ETH provider pool stays disabled.
- Disabled ETH provider pool does not panic app startup.
- ETH/ERC20 routes return deterministic `EthNotConfigured` errors.

## Security notes

- ETH private keys are accessed only from backend session state at send-time.
- Frontend never sends signed payloads or private key material.
- Preflight payloads remain backend-owned and single-use via `PreflightStore`.

## Phase-1 parity exclusions

The following mobile behaviors are intentionally out of scope for desktop core phase-1:

- Bridge convert/cross-chain flows (delegator contract, map-to/via/export-to semantics).
- Advanced approval edge-case workflows for conversion routes.
- Add-token ERC20 UX and persisted custom contract definitions.

These are tracked as phase-2 work once ETH/ERC20 core stability is validated.
