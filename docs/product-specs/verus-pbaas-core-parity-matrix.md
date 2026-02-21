---
owner: lite-wallet-team
last_reviewed: 2026-02-21
---

# Verus/PBaaS core parity matrix

This matrix tracks the in-scope backend/API transaction parity scenarios against
`valu-mobile` (`newsend3`).

## Scenario IDs

| ID              | Scenario                                        | Backend/API status | Notes                                            |
| --------------- | ----------------------------------------------- | ------------------ | ------------------------------------------------ |
| `VRPC-SEND-001` | VRPC simple send (`R/i/@` destination)          | Implemented        | Uses backend preflight + send by `preflight_id`. |
| `VRPC-RT-101`   | `sendcurrency` convert only                     | Implemented        | Through `preflight_vrpc_transfer`.               |
| `VRPC-RT-102`   | `sendcurrency` export only                      | Implemented        | Through `preflight_vrpc_transfer`.               |
| `VRPC-RT-103`   | `sendcurrency` convert + export + via           | Implemented        | Optional flags encoded in transfer output map.   |
| `VRPC-RT-104`   | `feecurrency` + `feesatoshis` route options     | Implemented        | Optional route fields accepted and encoded.      |
| `VRPC-RT-105`   | `preconvert`, `mapto`, `vdxftag` options        | Implemented        | Optional route fields accepted and encoded.      |
| `ID-UPD-201`    | `updateidentity`                                | Implemented        | Identity preflight + send path in Rust backend.  |
| `ID-REV-202`    | `revoke`                                        | Implemented        | Target-state and authority checks in place.      |
| `ID-REC-203`    | `recover`                                       | Implemented        | Enforces revoked-state requirement.              |
| `GUARD-REV-301` | Signed-out revoke via guard session             | Implemented        | In-memory guard session, no wallet persistence.  |
| `GUARD-REC-302` | Signed-out recover via guard session            | Implemented        | In-memory guard session, no wallet persistence.  |
| `PBAAS-DYN-401` | Dynamic PBaaS coin immediate preflight/send use | Implemented        | Uses runtime coin registry system-id resolution. |
| `DLIGHT-SEND-501` | dlight private send (`zs/R/i` destination)       | Implemented        | Backend spend engine is wired for preflight + build/prove/sign/broadcast; `i` resolves to primary `R` server-side. |
| `DLIGHT-AB-502` | dlight address book endpoint parity (`zs`)        | Implemented        | Address book endpoint kind includes `zs` normalization. |

## Validation status

- Rust unit tests: passing.
- TypeScript checks: passing.
- Testnet E2E txid validation: in progress.
  - 2026-02-12 guard recover scenario broadcasted:
    `a9a6980295946087131cefa796cdc93301b64479fe2ad0ce8d889993681bcae6`
    (`tryrevoke2@` recover via `player6@` authority with primary address patch).
