---
owner: lite-wallet-team
last_reviewed: 2026-02-25
---

# Blockchain runtime config and ETH/ERC20 parity notes

## Runtime environment variables

### Endpoint variables

- `VRPC_MAINNET_URL`
- `VRPC_TESTNET_URL`
- `VRPC_VARRR_MAINNET_URL`
- `VRPC_VDEX_MAINNET_URL`
- `VRPC_CHIPS_MAINNET_URL`
- `BTC_MAINNET_API_URL`
- `BTC_TESTNET_API_URL`
- `DLIGHT_MAINNET_ENDPOINTS`
- `DLIGHT_TESTNET_ENDPOINTS`
- `ELECTRUM_MAINNET_ENDPOINTS`
- `ELECTRUM_TESTNET_ENDPOINTS`

`DLIGHT_*` and `ELECTRUM_*` accept comma-separated values.
Example:
`ELECTRUM_MAINNET_ENDPOINTS=https://endpoint-a,https://endpoint-b`

### ETH/ERC20 key variables

- `INFURA_PROJECT_ID`
- `ETHERSCAN_API_KEY`

Optional ETH URL overrides:

- `ETH_MAINNET_RPC_URL`
- `ETH_TESTNET_RPC_URL`
- `ETHERSCAN_MAINNET_URL`
- `ETHERSCAN_TESTNET_URL`

## Startup validation behavior

- Backend startup loads `.env` and `.env.local` from both workspace root and `src-tauri/`.
- If required ETH key vars are missing or empty, ETH provider pool stays disabled.
- If configured URLs are invalid, ETH provider pool stays disabled.
- Disabled ETH provider pool does not panic app startup.
- ETH/ERC20 routes return deterministic `EthNotConfigured` errors.

## Security notes

- Endpoint and key config is runtime-only and should live in untracked `.env` files.
- Frontend does not own runtime endpoint/key secrets.
- ETH private keys are accessed only from backend session state at send-time.
- Frontend never sends signed payloads or private key material.
- Preflight payloads remain backend-owned and single-use via `PreflightStore`.

## Phase-1 parity exclusions

The following mobile behaviors are intentionally out of scope for desktop core phase-1:

- Bridge convert/cross-chain flows (delegator contract, map-to/via/export-to semantics).
- Advanced approval edge-case workflows for conversion routes.
- Add-token ERC20 UX and persisted custom contract definitions.

These are tracked as phase-2 work once ETH/ERC20 core stability is validated.

## Groundbase parity reference

Desktop ETH/ERC20 history behavior is intentionally aligned to valu-mobile:

- `src/utils/web3/etherscan.js`
- `src/utils/api/channels/eth/requests/getEthTransactions.js`
- `src/utils/api/channels/erc20/requests/getErc20Transactions.js`
