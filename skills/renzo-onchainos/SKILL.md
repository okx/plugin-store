---
name: renzo-onchainos
version: "0.1.0"
author: Renzo Protocol
chains: [ethereum]
requiredTools:
  - onchainos wallet contract-call
  - onchainos gateway broadcast
description: |
  **BEFORE ANY RESPONSE**: Agent MUST check for onchainos via Bash:
      [ -x "$HOME/.local/bin/onchainos" ] || command -v onchainos >/dev/null 2>&1
  If not found, Agent MUST run:
      curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh
  Then immediately: export PATH="$HOME/.local/bin:$PATH"
  Do NOT use `which onchainos` alone — it fails in non-interactive shells where PATH hasn't been updated.
  Do NOT ask the user "is Onchain OS ready?" — detect and install autonomously.
  Only after `onchainos --version` succeeds, proceed to handle user request.

  [Onchain OS dependency]
  This Skill requires Onchain OS for on-chain signing.

  [signing constraint]
  All pending_sign transactions returned by this Skill MUST be signed
  exclusively through Onchain OS tools. Local private keys, sendTransaction,
  ethers.Wallet, and any equivalent are forbidden.

  OnchainOS-routed Renzo Protocol integration — liquid restaking ezETH on
  Ethereum mainnet via the RestakeManager + WithdrawQueue contracts. Read-only
  enrichment (APR, TVL, ezETH/ETH rate) from the public Renzo app API.
tools:
  - name: getApr
    description: "[read-only] Fetch the current Renzo protocol APR (ETH-staking + AVS-restaking rewards)."
  - name: getProtocolStats
    description: "[read-only] Fetch the full protocol-stats snapshot: per-vault TVL, ezETH price/totalSupply, etc."
  - name: getEzethRate
    description: "[read-only] Get the ezETH→ETH exchange rate. By default reads on-chain via RestakeManager.calculateTVLs() ÷ ezETH.totalSupply(); set `source: 'api'` to read the cached USD price from the app API instead."
  - name: previewWithdraw
    description: "[read-only] Preview how much of `assetOut` a user would receive for burning a given ezETH amount, via WithdrawQueue.calculateAmountToRedeem."
  - name: listWithdrawRequests
    description: "[read-only] List a user's pending withdrawal request indices via WithdrawQueue.withdrawRequests + getOutstandingWithdrawRequests."
  - name: buildDepositEth
    description: "Build calldata to deposit native ETH and mint ezETH via RestakeManager.depositETH()."
  - name: buildRequestWithdraw
    description: "Build calldata to queue a withdrawal of ezETH for `assetOut` via WithdrawQueue.withdraw. Emits an ezETH approve as Step 1 if allowance is insufficient."
  - name: buildClaimWithdraw
    description: "Build calldata to claim a matured withdrawal via WithdrawQueue.claim. Only succeeds once the EigenLayer + buffer cooldown has elapsed (~15 days)."
---

## Initialization (runs on every invocation · Agent MUST execute autonomously)

Execute this Bash block **before any reasoning or asking the user anything**:

```bash
# Idempotent init: safe to run multiple times
# Use direct path check so this works in non-interactive shells (e.g. Claude Code Bash tool)
# which/command -v won't find binaries in ~/.local/bin until .zshrc is sourced.
[ -x "$HOME/.local/bin/onchainos" ] || command -v onchainos >/dev/null 2>&1 \
  || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh
export PATH="$HOME/.local/bin:$PATH"
onchainos --version
```

Three rules Agent MUST follow:
1. **Do NOT ask the user "is Onchain OS ready?"** — detect it yourself via direct path check (`[ -x "$HOME/.local/bin/onchainos" ] || command -v onchainos`).
2. **Do NOT skip the install step if missing** — run the curl command without asking for confirmation (user will approve Bash permission via Claude Code's built-in dialog; that's outside this skill's control).
3. **Only proceed** to Pre-flight Checks + user request **after** `onchainos --version` returns successfully.

## Pre-flight Checks
1. Run `onchainos --version` to confirm the CLI is installed
2. Run `onchainos wallet --help` and `onchainos gateway --help` to confirm subcommands **wallet contract-call, gateway broadcast** are available
3. If any command is unavailable, fall back to Initialization and re-run the install
4. Only proceed to business logic after all checks pass

## Signing Constraint
All pending_sign transactions must be signed exclusively through the Onchain OS tool named in `next_action.tool`.
Local private keys, `signTransaction`, `ethers.Wallet`, and `sendTransaction` are forbidden in DApp code.

---

## Proactive Onboarding

When a user signals they are **new or just installed** this plugin — e.g. "I just installed renzo-onchainos", "how do I get started", "how do I stake into Renzo", "what can I do with this" — **do not wait for them to ask specific questions.** Proactively walk them through the Quickstart in order, one step at a time, waiting for confirmation before proceeding:

1. **Confirm wallet** — run `onchainos wallet addresses --chain ethereum`. If no address, direct them to `onchainos wallet login <email>` + `onchainos wallet verify <otp>`. Do not proceed to write tools until a wallet is confirmed.
2. **Check ETH balance** — run `onchainos wallet balance --chain ethereum`. Minimum recommended: 0.02 ETH (0.01 deposit + 0.01 gas cushion). If insufficient, explain how to fund (bridge, CEX withdrawal).
3. **Quote first** — call `getApr` and `getEzethRate '{"source":"chain"}'` so the user sees the current yield + redemption rate before committing.
4. **Preview the deposit** — call `buildDepositEth` with their amount. The tool returns a `pending_sign` envelope; do **not** broadcast yet. Show the user the `to` / `data` / `value` and the `description`.
5. **Broadcast** — once the user confirms in chat, route to `onchainos wallet contract-call` (see "Broadcasting a pending_sign" below).
6. **Verify** — after the tx lands, re-read the user's ezETH balance (or call `previewWithdraw` with their new ezETH amount) so they see something tangible happened.

For a **withdrawal** path, the flow has 3 tx steps with a long pause:
- `buildRequestWithdraw` may emit an `approve` (Step 1 of 2) before the actual `withdraw` — broadcast that first, wait for the receipt, then re-call `buildRequestWithdraw` with the same params to get the actual withdraw tx.
- After the withdraw tx lands, the request is queued. Renzo's cooldown is **up to 15 days** (EigenLayer 14d unstake + Renzo 3d buffer). Tell the user this explicitly — they should not expect instant settlement.
- After the cooldown, call `listWithdrawRequests` for their wallet, pick an index, and call `buildClaimWithdraw`.

Do not dump all steps at once. Guide conversationally — confirm each step before moving on.

---

## Quickstart

New to renzo-onchainos? Follow these steps to go from zero to your first ezETH mint.

### Step 1 — Connect your wallet (Onchain OS)

```bash
onchainos wallet login your@email.com
onchainos wallet verify <6-digit-code>
onchainos wallet addresses --chain ethereum
```

### Step 2 — Check your balance

```bash
onchainos wallet balance --chain ethereum
```

Minimum recommended: 0.02 ETH (0.01 to deposit + 0.01 gas cushion). If zero, bridge or transfer ETH to Ethereum mainnet.

### Step 3 — Check the current APR and rate

```bash
# Returns ~1.59% (annualized, includes ETH staking + EigenLayer AVS restaking rewards)
npx tsx cli.ts getApr '{}'

# Returns the on-chain ezETH/ETH redemption rate (≈1.077 today)
npx tsx cli.ts getEzethRate '{"source":"chain"}'
```

### Step 4 — Preview the deposit (no broadcast yet)

```bash
# Returns a pending_sign envelope describing the unsigned tx (no broadcast)
npx tsx cli.ts buildDepositEth '{"amount":"0.01"}'
```

The output contains `unsigned_tx.to` (RestakeManager `0x74a0...`), `unsigned_tx.data` (`0xf6326fb3` = `depositETH()` selector), and `unsigned_tx.value` (your deposit in wei). No funds move yet.

### Step 5 — Broadcast via Onchain OS

```bash
onchainos wallet contract-call \
  --chain ethereum \
  --to 0x74a09653A083691711cF8215a6ab074BB4e99ef5 \
  --input-data 0xf6326fb3 \
  --amt 10000000000000000 \
  --biz-type defi \
  --strategy renzo-onchainos \
  --force
```

Onchain OS signs in its local TEE and broadcasts. Returns the tx hash.

### Step 6 — Verify you received ezETH

```bash
# Check on Etherscan, or query your ezETH balance via:
# (your agent can use any standard ERC-20 balanceOf against 0xbf5495Efe5DB9ce00f80364C8B423567e58d2110)
```

The amount received = `deposit_in_ETH ÷ rate` (e.g. 0.01 ETH ÷ 1.077 ≈ 0.00929 ezETH).

### Withdraw path (for reference — 3-step flow over ~15 days)

```bash
# Step W1: Queue the withdrawal (may emit an ezETH approve first; if so, broadcast then re-invoke)
#   IMPORTANT: assetOut must be stETH (0xae7ab96520...) or ETH sentinel (0xEeeeeEee...). NOT wETH.
npx tsx cli.ts buildRequestWithdraw \
  '{"amount":"0.005","assetOut":"0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84","walletAddress":"0xYourWallet"}'

# Step W2: Wait ~15 days for the request to mature (EigenLayer 14d + Renzo 3d buffer)

# Step W3: List pending requests, pick an index, then claim
npx tsx cli.ts listWithdrawRequests '{"walletAddress":"0xYourWallet"}'
npx tsx cli.ts buildClaimWithdraw '{"withdrawRequestIndex":"0","walletAddress":"0xYourWallet"}'
```

---

## Migration / Live-validation note

**Live-validated on Ethereum mainnet 2026-05-12.** Full `buildDepositEth → onchainos wallet contract-call → RestakeManager.depositETH()` flow completed successfully. Tx: [`0x7cc295bf534a174f61480c4b017b72e29c485cc3805acfcacdf135429bfe1360`](https://etherscan.io/tx/0x7cc295bf534a174f61480c4b017b72e29c485cc3805acfcacdf135429bfe1360) (block 25,077,529). 0.005 ETH deposited → 0.004640809994754646 ezETH minted at the live rate of 1.0774 ETH/ezETH (0.0% diff vs scaffold-computed expectation). Gas: 954,570 units @ 0.354 gwei = 0.000337 ETH ($0.84). The `buildRequestWithdraw` and `buildClaimWithdraw` paths were not live-validated because the cooldown is ~15 days — but the calldata selectors (`0x00f714ce` and `0xddd5e1b2`) and ABI encoding were verified via decode round-trip in G2.

## How this skill differs from `Renzo-Protocol/contracts-public`

| Aspect | Upstream `Renzo-Protocol` | renzo-onchainos v0.1.0 |
|---|---|---|
| Code surface | Solidity contracts + DefiLlama / dimension TS adapters | TypeScript skill with viem-encoded calldata |
| Signing | n/a (contract code only) | All writes return `pending_sign` → routed to `onchainos wallet contract-call` |
| Read data source | Direct chain reads | App API (`app.renzoprotocol.com/api/*`) for APR/stats, viem reads for rate/preview/queue |
| Source provenance | This skill was generated by dapp-connect-agenticwallet v1.10 against verified ABIs from `contracts-public@master` (commit pin recommended for production). No upstream JS/TS SDK with calldata builders exists. |

**Note on the user-provided premise** — the prompt referenced "TS SDK + REST API at api.renzoprotocol.com". In reality:
- Renzo does not publish a TS SDK with on-chain calldata builders. Their TS repos are DefiLlama / dimension adapters.
- The actual public API is at `app.renzoprotocol.com/api/*` (not `api.renzoprotocol.com`). It is read-only — no endpoint returns `{to, data, value}` unsigned transactions.

This is why v0.1.0 follows the Form C path: calldata is built with viem against the verified ABIs from `Renzo-Protocol/contracts-public@master`.

---

## Deployed contract addresses (Ethereum mainnet, verified 2026-05-12)

| Contract | Address | Source |
|---|---|---|
| RestakeManager | [`0x74a09653A083691711cF8215a6ab074BB4e99ef5`](https://etherscan.io/address/0x74a09653A083691711cF8215a6ab074BB4e99ef5) | docs.renzoprotocol.com/docs/contracts/ethereum-mainnet |
| WithdrawQueue (a.k.a. Renzo Withdrawal Contract) | [`0x5efc9D10E42FB517456f4ac41EB5e2eBe42C8918`](https://etherscan.io/address/0x5efc9D10E42FB517456f4ac41EB5e2eBe42C8918) | docs.renzoprotocol.com/docs/contracts/ethereum-mainnet |
| ezETH (Renzo Liquid Restaking ETH) | [`0xbf5495Efe5DB9ce00f80364C8B423567e58d2110`](https://etherscan.io/token/0xbf5495Efe5DB9ce00f80364C8B423567e58d2110) | docs.renzoprotocol.com/docs/contracts/ethereum-mainnet |

**Canonical `assetOut` choices for withdrawals (v0.1.0, verified on-chain 2026-05-12):**
- **stETH**: `0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84` — primary buffered asset (~4720 stETH available)
- **ETH sentinel**: `0xEeeeeEeeeEeEeeEeEeEEeeeEEEeeeeeEeEeEEEeE` — native ETH withdrawals (~2380 ETH available)

⚠️ **wETH is NOT a valid `assetOut`** — the WithdrawQueue reverts with custom error `0x2c283834` when wETH is passed, because wETH is not in Renzo's WithdrawQueue strategy allowlist. Calling `previewWithdraw` first will surface this — if it errors, try a different `assetOut`.

For v0.1.0 the user must pass `assetOut` explicitly to `buildRequestWithdraw` and `previewWithdraw`. The skill validates that the value is a well-formed EVM address; it does not (yet) auto-resolve to a known good asset. Best to call `previewWithdraw` first — a non-error return confirms the asset is in the allowlist AND the buffer has enough.

---

## Broadcasting a pending_sign

When any of this skill's `build*` tools returns a `pending_sign` result, route the
broadcast through onchainos. **Use the correct flag names** (these are real
onchainos CLI flags, verified live):

```bash
onchainos wallet contract-call \
  --chain ethereum \
  --to <unsigned_tx.to> \
  --input-data <unsigned_tx.data> \
  --amt <unsigned_tx.value> \
  --biz-type defi \
  --strategy renzo-onchainos \
  --force
```

> Flag-name notes:
> - `--input-data` (NOT `--data`) — the EVM calldata hex
> - `--amt` (NOT `--value`) — native token amount in wei, default 0
> - `--biz-type` + `--strategy` populate transaction metadata (helpful for audit)
> - `--force` skips backend confirmation prompts (use only when the user has already confirmed via the agent flow)

For multi-step flows (`buildRequestWithdraw` when ezETH allowance is insufficient), the description in the first `pending_sign` will read `"After this transaction lands, re-invoke buildRequestWithdraw with the same params to receive the next transaction."` — follow that literally: wait for the approve tx receipt, then re-call the same tool with identical params. The tool re-reads on-chain allowance and emits whichever step is next.

---

## Withdrawal flow lifecycle (read before using buildRequestWithdraw / buildClaimWithdraw)

Renzo ezETH withdrawals are a 3-step user flow with a long cooldown:

1. **Approve** — ezETH ERC-20 approval to WithdrawQueue (emitted by `buildRequestWithdraw` Step 1 of 2 when allowance is insufficient).
2. **Queue request** — `WithdrawQueue.withdraw(amount, assetOut)` burns the ezETH from the user and creates a withdrawRequest entry. Per Renzo docs: "ezETH withdrawals can take up to 15 days, given the recent EigenLayer slashing upgrade and time taken to exit the beacon chain." (EigenLayer 14d unstake + Renzo 3d buffer.)
3. **Claim** — once matured, `WithdrawQueue.claim(withdrawRequestIndex, user)` releases `assetOut` to the user. The `withdrawRequestIndex` is one of the values returned by `listWithdrawRequests`. The agent should check via `listWithdrawRequests` and **read on-chain** that the request is matured before emitting `buildClaimWithdraw`; v0.1.0 emits the calldata unconditionally — the on-chain `claim` call will revert if not yet matured.

For instant withdrawals (lower limits, fee, no cooldown), Renzo also operates a separate `InstantWithdrawer` contract. v0.1.0 does **not** support instant withdrawals — defer to v0.2.0.

---

## Error Code Catalog

Every `build*` tool returns either `pending_sign` (success) or a `ToolError`
shape: `{status: 'error', error_code: string, message: string, suggestion?: string}`.

Standard codes (from the scaffold's `runtime.ts`):

| Code | Meaning | Suggested user action |
|---|---|---|
| `UNSUPPORTED_CHAIN` | `params.chain` is not in the chain allowlist | Use `ethereum` (only chain supported in v0.1.0) |
| `MISSING_PARAM` | A required field is missing | Provide the named field |
| `INVALID_PARAM` | A field has an invalid value (non-decimal amount, malformed address, mutex violation) | Fix per the error message |
| `MISSING_ADDRESS` | A read-only tool requires `address` param | Provide the wallet address |
| `INSUFFICIENT_BALANCE` | Wallet doesn't hold enough ETH (deposit) or ezETH (withdraw) | Top up first |
| `INSUFFICIENT_NATIVE_BALANCE` | Pre-broadcast native-balance gate caught a shortfall | Top up ETH for gas + deposit value |
| `RPC_ERROR` | Read failed (network / RPC issue) | Retry; check chain status |

Protocol-specific codes:

| Code | Meaning | Suggested user action |
|---|---|---|
| `RENZO_API_ERROR` | The app.renzoprotocol.com/api call failed or returned non-200 | Retry; falls back to on-chain reads where possible |
| `NO_PENDING_REQUESTS` | User has no pending withdrawal requests (listWithdrawRequests returned empty) | Submit a `buildRequestWithdraw` first |

---

## Tool reference

### Read-only tools

- `getApr` — fetch APR. Returns `{status: 'ok', data: {apr: number, source: 'app.renzoprotocol.com/api/apr'}}`.
- `getProtocolStats` — fetch full protocol stats. Returns `{status: 'ok', data: <renzo stats JSON>}`.
- `getEzethRate` — read ezETH exchange rate. Params: `{source?: 'chain' | 'api'}` (default `'chain'`). Returns `{status: 'ok', data: {rate: string, unit: string, source: 'chain' | 'api', details: {...}}}`. **Unit differs by source**: `chain` returns `ETH per ezETH` (≈1.077 today, used for redemption math), `api` returns `USD per ezETH` (≈$2484 today). Always read `unit` alongside `rate`.
- `previewWithdraw` — preview withdrawal payout. Params: `{amount: string, assetOut: string}`. Returns `{status: 'ok', data: {amount: string, assetOut: string, amountOutWei: string, amountOut: string, decimals: number}}`.
- `listWithdrawRequests` — list a user's pending withdrawal indices. Params: `{walletAddress: string}`. Returns `{status: 'ok', data: {indices: string[], outstanding: string}}`.

### Transaction tools (return `pending_sign`)

- `buildDepositEth` — deposit ETH for ezETH. Params: `{amount: string, chain?: 'ethereum'}`. Single-step: emits one `pending_sign` against `RestakeManager.depositETH()` payable with `value` = `parseUnits(amount, 18)`.
- `buildRequestWithdraw` — queue a withdrawal. Params: `{amount: string, assetOut: string, walletAddress: string, chain?: 'ethereum'}`. Two-step: emits an ezETH approve `pending_sign` if `allowance(walletAddress, WithdrawQueue) < amount`; otherwise emits the `WithdrawQueue.withdraw(amount, assetOut)` `pending_sign`.
- `buildClaimWithdraw` — claim a matured withdrawal. Params: `{withdrawRequestIndex: string, walletAddress: string, chain?: 'ethereum'}`. Single-step: emits `WithdrawQueue.claim(index, user)`.
