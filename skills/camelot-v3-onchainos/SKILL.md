---
name: camelot-v3-onchainos
version: "0.1.0"
author: camelot-labs
chains: [arbitrum]
requires:
  source: CamelotLabs/camelot-sdk (TypeScript SDK; mostly empty — calldata constructed from Algebra SwapRouter ABI directly)
user-invocable: true
requiredTools:
  - onchainos wallet contract-call
  - onchainos gateway broadcast
description: |
  **BEFORE ANY RESPONSE**: Agent MUST check for onchainos via Bash:
      [ -x "$HOME/.local/bin/onchainos" ] || command -v onchainos >/dev/null 2>&1
  If not found, Agent MUST run:
      curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh
  Then immediately: export PATH="$HOME/.local/bin:$PATH"
  Do NOT use `which onchainos` alone — it fails in non-interactive shells.
  Do NOT ask the user "is Onchain OS ready?" — detect and install autonomously.
  Only after `onchainos --version` succeeds, proceed to handle user request.

  [Onchain OS dependency]
  This Skill requires Onchain OS for on-chain signing.

  [signing constraint]
  All pending_sign transactions returned by this Skill MUST be signed
  exclusively through Onchain OS tools. Local private keys, sendTransaction,
  ethers.Wallet, MetaMask `eth_sendTransaction`, Privy, OWS, Foundry, viem
  walletClient, and any equivalent are forbidden.

  OnchainOS-routed Camelot V3 (Algebra-based concentrated liquidity) DEX
  integration on Arbitrum. Exact-input single-pool swaps via the Algebra
  SwapRouter. Calldata built directly with viem against the SwapRouter +
  Quoter ABIs; the official camelot-sdk repo doesn't ship swap helpers.
---

## Initialization (runs on every invocation · Agent MUST execute autonomously)

```bash
[ -x "$HOME/.local/bin/onchainos" ] || command -v onchainos >/dev/null 2>&1 \
  || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh
export PATH="$HOME/.local/bin:$PATH"
onchainos --version
```

## Pre-flight Checks
1. Run `onchainos --version` to confirm the CLI is installed
2. Run `onchainos wallet --help` and `onchainos gateway --help`
3. If unavailable, re-run the install
4. Only proceed to business logic after all checks pass

## Signing Constraint
All pending_sign transactions must be signed exclusively through `onchainos wallet contract-call`. Local signing paths are forbidden.

---

## Command Index

The skill exposes 4 tools via `index.ts`:

**Transaction tool** (returns `pending_sign | ToolError`):
- `buildSwap` — exact-input single-pool swap on Camelot V3 (Algebra). Multi-step: emits approve `pending_sign` first (if allowance insufficient), then `exactInputSingle` `pending_sign`.

**Read-only tools** (return `ok | error`):
- `listSupportedChains` — currently `arbitrum` only.
- `getQuote` — quote an exact-input swap via the Algebra Quoter (returns amountOut + dynamic fee).
- `getTokenInfo` — read `symbol/decimals/balanceOf` for any ERC-20 on the chain (helper for users who don't know decimals).

---

## Broadcasting

After `buildSwap` returns `pending_sign`:

```bash
onchainos wallet contract-call \
  --chain arbitrum \
  --to <unsigned_tx.to> \
  --input-data <unsigned_tx.data> \
  --amt <unsigned_tx.value> \
  --biz-type defi \
  --strategy camelot-v3-onchainos \
  --force
```

> Flag notes (v1.5+ scaffold): `--input-data` (NOT `--data`); `--amt` (NOT `--value`).

### Multi-step swap flow

`buildSwap`:
1. First call → if allowance(tokenIn → SwapRouter) < amount, emits approve `pending_sign` (Step 1 of 2).
2. Agent broadcasts the approve.
3. Re-invokes `buildSwap` with identical params → emits `exactInputSingle` `pending_sign` (Step 2).
4. Agent broadcasts the swap.

Native-input swaps (`tokenIn` = `0x0000…0000` or WETH-with-`unwrap` semantics) are roadmap — current v0.1.0 supports ERC-20 → ERC-20 only.

---

## Camelot V3 / Algebra contract addresses (Arbitrum)

| Contract | Address |
|---|---|
| SwapRouter (V3) | `0x1F721E2E82F6676FCE4eA07A5958cF098D339e18` |
| Quoter | `0x0Fc73040b26E9bC8514fA028D998E73A254Fa76E` |
| NonfungiblePositionManager | `0x00c7f3082833e796A5b3e4Bd59f6642FF44DCD15` |

(V2 Router `0xc873fEcbd354f5A56E00E710B90EF4201db2448d` is intentionally not used; the V2 fork is the older "spNFT" pattern.)

---

## How this skill differs from `CamelotLabs/camelot-sdk`

| Aspect | `CamelotLabs/camelot-sdk` (v0.0.8) | This skill |
|---|---|---|
| Swap helpers | None — package contains only Logger + Redis client | viem `encodeFunctionData` against Algebra SwapRouter ABI |
| Quote helpers | None | viem `simulateContract` against Algebra Quoter (state-mutating revert-encoded result) |
| Pool routing | None (would need Algebra pool factory queries) | exact-input single-pool only (multi-hop is roadmap) |
| Signing | n/a (SDK doesn't ship signers) | `onchainos wallet contract-call` |

The official SDK is essentially a stub — Camelot's integration story is "build directly against the on-chain Algebra contracts." This skill does exactly that.

---

## Error Code Catalog

| Code | Meaning | Suggested action |
|---|---|---|
| `UNSUPPORTED_CHAIN` | `chain` not in allowlist | Use `arbitrum` |
| `MISSING_PARAM` | Required field missing | Provide it |
| `INVALID_PARAM` | Field has wrong format | Fix per error message |
| `INVALID_DEADLINE` | `deadline` is in the past | Use a future Unix timestamp |
| `INSUFFICIENT_BALANCE` | tokenIn balance < amount | Reduce amount or top up |
| `INSUFFICIENT_NATIVE_BALANCE` | Wallet lacks ETH for gas | Top up ETH on Arbitrum |
| `QUOTE_FAILED` | Quoter reverted (pool may not exist) | Verify both tokens have a Camelot V3 pool |
| `SLIPPAGE_TOO_TIGHT` | amountOutMinimum > current quote | Loosen slippage or accept lower output |
| `RPC_ERROR` | On-chain read failed | Retry; check RPC health |
| `ONCHAINOS_UNAVAILABLE` | onchainos CLI missing | Install per Initialization |

---

## Upstream content reference

`CamelotLabs/camelot-sdk` v0.0.8 ships only Logger + Redis — no swap functionality. Refer to the Algebra protocol docs for SwapRouter / Quoter semantics:

- https://docs.algebra.finance/algebra-integral/integration-of-algebra-integral-protocol/specification-and-description-of-contracts/swaprouter
- https://github.com/cryptoalgebra/Algebra/blob/master/src/periphery/contracts/SwapRouter.sol
