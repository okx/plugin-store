# FILL-IN — lista-lending-onchainos v0.3.0

Status: **all 12 tools implemented; ready for v1.8 G1+G2 audit gates.**

v0.3.0 restores feature parity with the upstream `lista-lending` v0.1.1 CLI (12 commands), repackaged per v1.8 scaffold spec. Business logic preserved verbatim from v0.1.6 (live-validated on BSC mainnet 2026-05-11).

## Transaction tools (6)

| Tool | Description | Multi-step? | A7 (decimals validated) | A8 (native balance gate) |
|---|---|---|---|---|
| `buildDeposit` | Vault deposit (ERC-4626 `deposit`) | Yes — approve then deposit | Structural PASS — decimals SDK-derived from `vaultInfo.assetInfo.decimals` | N/A (value=0 always) |
| `buildWithdraw` | Vault withdraw (ERC-4626 `redeem`) | No | Structural PASS | N/A |
| `buildSupply` | Market collateral supply | Yes — approve then supply | Structural PASS — decimals from `marketConfig.collateral.decimals` | N/A |
| `buildBorrow` | Market loan-token borrow | No | Structural PASS — decimals from `marketConfig.loan.decimals` | N/A |
| `buildRepay` | Market debt repay | Yes — approve then repay | Structural PASS | N/A |
| `buildMarketWithdraw` | Market collateral withdraw | No | Structural PASS | N/A |

**Why A7 is structural PASS**: index.ts never invokes `parseUnits(amount, params.X)`. Decimals always come from SDK-fetched `vaultInfo.assetInfo.decimals` / `marketConfig.collateral.decimals` / `marketConfig.loan.decimals` — local variables, not user-passed.

**Why A8 is structural N/A**: Lista lending operations are all ERC-20 token flows on the vault/market contract; the user-signed tx never carries native value. `unsigned_tx.value` is always emitted as `'0'` (or `step.params.value.toString()` which is `'0'` from the SDK).

## Read-only tools (6)

| Tool | Description | Source |
|---|---|---|
| `listVaults` | Curated vault catalog per chain | SDK `listVaults(chainId)` |
| `listMarkets` | Active markets per chain (filters SmartLending zone=3 + fixed-term termType=1) | SDK `listMarkets(chainId)` |
| `getHoldings` | User's vault + market positions across chains | Lista API (note: 30-60s indexer lag) |
| `getVaultPosition` | User's direct ERC-4626 share balance + asset equivalent | viem `balanceOf` + `convertToAssets` (no indexer lag) |
| `simulateBorrow` | Pre-execution: max borrowable / health impact for a `(market, supply, borrow)` triple | SDK `simulateBorrow(...)` |
| `simulateRepay` | Pre-execution: health impact of partial/full repay | SDK `simulateRepay(...)` |

## v0.3.0 cross-cutting checklist (v1.8 conformance)

- [x] No user-passed decimals — all SDK-derived (A7 structural PASS)
- [x] No non-zero `unsigned_tx.value` for any lending tool (A8 N/A)
- [x] No local-signing residue (A4 PASS — verified with expanded keyword set)
- [x] `pending_sign` contract present for every write tool (A5 PASS)
- [x] 3 fixed SKILL.md sections present: `[Onchain OS dependency]`, `[signing constraint]`, `## Pre-flight Checks`, `## Signing Constraint` (A3 PASS)
- [x] Frontmatter merged from source — `name`, `version`, `repository`, `chains`, `requires`, `node`, `requiredTools`, `author`, `description` (Step 3a v1.4 merge rules)
- [x] Mandatory G1 (`explore-plugin`) + G2 (`test-skill`) gates per P0-9 — see QA-PIPELINE.md
- [x] Validators: `resolveChain`, `validatePositiveAmount`, `validateEvmAddress`, `validateMarketId`, `safeSdkCall` (lista-specific helpers in index.ts, predating the v1.8 generic helpers but functionally equivalent)
- [x] PUBLIC_RPCS pinned via `runtime.ts` — defaults to publicnode endpoints, overridable via `LISTA_RPC_*` env vars

## Roadmap (post-v0.3.0)

- [ ] Add explicit `validateDecimals` / `nativeBalanceCheck` / `lc()` from v1.8 generic helper set for forward-compat with future scaffold auto-checks (no functional change; just upgrades A7/A8 from structural-PASS to mechanical-PASS)
- [ ] Re-add `select` as a state-mutation tool if user feedback shows the explicit-param model is too verbose
- [ ] Optional: SmartLending zone=3 + fixed-term termType=1 market support (currently out of scope per source SKILL.md "Temporary Limitations")
