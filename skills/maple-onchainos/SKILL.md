---
name: maple-onchainos
version: "0.1.0"
author: maple-labs
chains: [ethereum, base]
requires:
  source: maple-labs/maple-js (TypeScript SDK; only `generateUnsignedTransactionData` reused conceptually)
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

  OnchainOS-routed Maple Finance integration — lend USDC/USDT into Maple/Syrup
  ERC-4626 pools (syrupUSDC, syrupUSDT) and queue withdrawals. EVM only
  (Ethereum mainnet + Base). Reuses the calldata semantics from
  `maple-labs/maple-js` (`poolDeposit` → `deposit(uint256,address)`,
  `poolQueueWithdrawal` → `requestRedeem(uint256,address)`) but skips the
  SDK's provider-dependent gas estimation and local-signing helpers since
  Onchain OS handles those.
---

## Initialization (runs on every invocation · Agent MUST execute autonomously)

```bash
[ -x "$HOME/.local/bin/onchainos" ] || command -v onchainos >/dev/null 2>&1 \
  || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh
export PATH="$HOME/.local/bin:$PATH"
onchainos --version
```

Three rules:
1. **Do NOT ask the user "is Onchain OS ready?"** — detect it yourself via the direct path check.
2. **Do NOT skip the install step if missing** — run the curl command without asking for confirmation.
3. **Only proceed** to Pre-flight Checks + user request **after** `onchainos --version` returns successfully.

## Pre-flight Checks
1. Run `onchainos --version` to confirm the CLI is installed
2. Run `onchainos wallet --help` and `onchainos gateway --help` to confirm subcommands **wallet contract-call, gateway broadcast** are available
3. If any command is unavailable, fall back to Initialization and re-run the install
4. Only proceed to business logic after all checks pass

## Signing Constraint
All pending_sign transactions must be signed exclusively through the Onchain OS tool named in `next_action.tool`.
Local private keys, MetaMask `eth_sendTransaction`, OWS, Foundry cast, viem `walletClient.sendTransaction`, ethers `wallet.sendTransaction`, and Privy embedded signing are forbidden.

---

## Command Index

The skill exposes 5 tools via `index.ts`:

**Transaction tools** (return `pending_sign | ToolError`):
- `buildDeposit` — lend USDC/USDT into a Maple pool. Multi-step: emits approve `pending_sign` first (if allowance insufficient), then ERC-4626 `deposit` `pending_sign`.
- `buildQueueWithdrawal` — queue redemption of pool shares back to underlying. Calls pool's `requestRedeem(shares, receiver)`. No approval needed (user owns shares).

**Read-only tools** (return `ok | error`):
- `listSupportedPools` — known Maple/Syrup pools per chain with underlying asset + share-token symbol.
- `getPool` — pool stats (symbol, asset, decimals, totalAssets, exchange rate snapshot).
- `getPosition` — user's share balance + assets-equivalent + pending redemption (if any).

---

## Broadcasting

After any `build*` tool returns `pending_sign`:

```bash
onchainos wallet contract-call \
  --chain <chain> \
  --to <unsigned_tx.to> \
  --input-data <unsigned_tx.data> \
  --amt <unsigned_tx.value> \
  --biz-type defi \
  --strategy maple-onchainos \
  --force
```

> Flag notes:
> - `--input-data` (NOT `--data`)
> - `--amt` (NOT `--value`)
> - `--biz-type defi` + `--strategy maple-onchainos` for audit metadata
> - `--force` skips backend confirmation (agent flow already confirmed)

### Multi-step deposit flow

`buildDeposit` uses the stateful single-step emission pattern:

1. First call → if allowance to the pool is insufficient, emits approve `pending_sign` with description "Step 1 of 2: Approve … After this transaction lands, re-invoke buildDeposit with the same params"
2. Agent broadcasts the approve, waits for confirmation
3. Re-invokes `buildDeposit` with identical params
4. Now allowance is sufficient → emits the ERC-4626 deposit `pending_sign`

### Withdrawal lifecycle

Maple withdrawals are **queued**, not instant:
1. User calls `buildQueueWithdrawal` → broadcasts `requestRedeem(shares, recipient)`
2. The withdrawal manager processes the queue periodically (typically end-of-cycle)
3. Once processed, the assets become claimable. This skill currently exposes the queue step only; the claim step is roadmap.

---

## How this skill differs from `maple-labs/maple-js`

| Aspect | `maple-labs/maple-js` | This skill |
|---|---|---|
| Tx construction | `generateUnsignedTransactionData({type, params, provider, ...})` returning `{txBytes, txInstance}` | Direct viem `encodeFunctionData` returning `pending_sign` envelope (no provider needed) |
| Gas + nonce | SDK calls `provider.estimateGas` / `provider.getTransactionCount` | Onchain OS handles gas + nonce at broadcast time |
| Signing | `generateSignedTransactionData` + `broadcastSignedTransaction` (local signer) | `onchainos wallet contract-call` (TEE-based) |
| Pool discovery | Hardcoded address constants in `src/addresses/<chain>.ts` (mostly infrastructure addrs, not pool instances) | Curated list of canonical user-facing pools (`syrupUSDC`, `syrupUSDT`) read via ERC-4626 |
| Output contract | `txInstance` shape varies (UnsignedTransaction or hex bytes) | Always returns `pending_sign` envelope; agent routes via `next_action.tool` |

The Maple SDK's calldata semantics are unchanged — the wrapper just bypasses the heavy ethers v5 provider plumbing in favor of viem + onchainos.

---

## Error Code Catalog

| Code | Meaning | Suggested action |
|---|---|---|
| `UNSUPPORTED_CHAIN` | `chain` not in allowlist | Use a supported chain (ethereum, base) |
| `MISSING_PARAM` | Required field missing | Provide it |
| `INVALID_PARAM` | Field has wrong format | Fix per error message |
| `UNKNOWN_POOL` | Pool address not in the curated list | Use a pool from `listSupportedPools` |
| `RPC_ERROR` | On-chain read failed | Retry; check RPC health |
| `INSUFFICIENT_BALANCE` | User holds less than the requested amount of the underlying / shares | Reduce amount or top up |
| `INSUFFICIENT_NATIVE_BALANCE` | Wallet lacks gas for the tx | Top up native asset |
| `NO_POSITION` | User has no shares in the requested pool | Deposit first |
| `ONCHAINOS_UNAVAILABLE` | onchainos CLI missing | Install per Initialization block |

---

## Upstream content reference

The upstream `maple-labs/maple-js` is an SDK package (not a skill). It is not bundled into this skill directory because the SDK's runtime requirements (ethers v5 provider, gas estimation, local signing) conflict with the OnchainOS routing model. Consult the upstream repo for protocol mechanics:

- `src/helpers/serialiseTransaction.ts` — calldata semantics (this skill reuses `deposit(assets,receiver)` and `requestRedeem(assets,receiver)`)
- `src/addresses/<network>.ts` — infrastructure addresses (Globals, Factories, PoolManagers)
- `src/typechain/poolV3/` — full ABI for current pool contracts
