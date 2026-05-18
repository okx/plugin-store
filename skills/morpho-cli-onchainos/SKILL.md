---
name: morpho-cli-onchainos
version: "0.1.0"
author: "morpho-org"
chains: [ethereum, base, arbitrum, optimism, polygon, unichain, worldchain, katana, hyperevm, monad, stable]

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

  Drive the Morpho lending protocol from the terminal via `npx @morpho-org/cli@latest` — queries vaults/markets/positions and prepares unsigned Morpho transactions with built-in simulation across all supported chains (Ethereum, Base, Arbitrum, Optimism, Polygon, Unichain, World Chain, Katana, HyperEVM, Monad, Stable). Invoke whenever the user asks to explore Morpho vault APYs / TVL / allocations ("best USDC vault on Base"), compare Morpho Blue markets ("ETH/USDC markets on Arbitrum"), inspect positions or health factor ("what are my Morpho positions"), or prepare any Morpho operation — deposit, withdraw, supply, borrow, repay, supply/withdraw collateral — even when the user doesn't explicitly name the CLI.

requiredTools:
  - onchainos wallet contract-call
  - onchainos gateway broadcast

tools:
  - healthCheck — [read-only] CLI health probe
  - getSupportedChains — [read-only] list registered chains
  - queryVaults — [read-only] list MetaMorpho vaults by criteria
  - getVault — [read-only] fetch a single vault's detail
  - queryMarkets — [read-only] list Morpho Blue markets
  - getMarket — [read-only] fetch a single market's detail
  - getPositions — [read-only] user vault + market positions
  - getTokenBalance — [read-only] wallet balance + Morpho/Bundler/Permit2 allowances
  - simulateTransactions — [read-only] standalone simulation of arbitrary txs
  - prepareDeposit — prepare unsigned vault deposit tx
  - prepareWithdraw — prepare unsigned vault withdraw tx
  - prepareSupply — prepare unsigned market supply tx
  - prepareBorrow — prepare unsigned market borrow tx
  - prepareRepay — prepare unsigned market repay tx
  - prepareSupplyCollateral — prepare unsigned market collateral supply tx
  - prepareWithdrawCollateral — prepare unsigned market collateral withdraw tx
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
2. Run `onchainos wallet --help` and `onchainos gateway --help` to confirm subcommands **onchainos wallet contract-call, onchainos gateway broadcast** are available
3. If any command is unavailable, fall back to Initialization and re-run the install
4. Only proceed to business logic after all checks pass

## Signing Constraint
All pending_sign transactions must be signed exclusively through the Onchain OS tool named in `next_action.tool`.
Local private keys, `signTransaction`, `ethers.Wallet`, and `sendTransaction` are forbidden in DApp code.

---

## Proactive Onboarding

When a user signals they are **new or just installed** this plugin — e.g. "I just installed morpho-cli-onchainos", "how do I get started", "what can I do with Morpho" — **do not wait for them to ask specific questions.** Proactively walk them through the Quickstart in order, one step at a time, waiting for confirmation before proceeding to the next:

1. **Check wallet** — run `onchainos wallet addresses --chain base`. If no address, direct them to connect via `onchainos wallet login your@email.com` and verify the OTP. Do not proceed to write operations until a wallet is confirmed.
2. **Pick a chain** — Morpho is on 11 chains (ethereum, base, arbitrum, optimism, polygon, unichain, worldchain, katana, hyperevm, monad, stable). Ask which one the user wants to operate on. If they don't know, suggest `base` (deepest Morpho liquidity for most assets, lowest gas) or `ethereum` (largest TVL).
3. **Browse vaults or markets** — Vaults (MetaMorpho) are passive yield: pick one, deposit, earn. Markets (Morpho Blue) are active: supply one asset as collateral, borrow another. Run `queryVaults` or `queryMarkets` to show what's available. Ask which path matches the user's goal.
4. **Preview first write** — Build the first prepare-* call (e.g. `prepareDeposit`). The returned `pending_sign` IS the preview — show the user `summary`, `unsigned_tx.to`, `step_info` (if approve+action is needed), and any `warnings` BEFORE routing to onchainos.
5. **Execute** — once they confirm, route through `onchainos wallet contract-call` per "Broadcasting a pending_sign" below. For multi-step flows (deposit/supply/repay with approve), re-invoke the same tool with the same params after each tx confirms.

Do not dump all steps at once. Guide conversationally — confirm each step before moving on.

---

## Quickstart

New to morpho-cli-onchainos? Follow these steps to go from zero to your first Morpho deposit.

### Step 1 — Connect your wallet

```bash
onchainos wallet login your@email.com     # sends OTP
onchainos wallet verify <6-digit-code>    # completes login
onchainos wallet addresses --chain base   # confirm address
```

### Step 2 — Check your balance

```bash
onchainos wallet balance --chain base
```

Minimum recommended: ~$2 USDC for a meaningful first deposit + ~$0.50 native ETH for gas. If zero, bridge or transfer USDC to Base.

### Step 3 — Browse available vaults

```bash
# Highest-APY USDC vaults on Base, top 5:
npx tsx cli.ts queryVaults '{"chain":"base","assetSymbol":"USDC","sort":"apy_desc","limit":5}'
```

Note the `address` of one that looks interesting. Things to compare:
- `apyPct` — current yield as percent string (`"5.34"` = 5.34%)
- `tvlUsd` — vault size; bigger TVL usually means deeper risk diversification
- `curator` — the team picking which markets the vault allocates into
- `feePct` — the cut the curator takes

### Step 4 — Preview the deposit (no funds moved)

Every write tool returns an unsigned `pending_sign` — this IS your preview. Nothing broadcasts until onchainos signs it.

```bash
npx tsx cli.ts prepareDeposit '{
  "chain":"base",
  "vaultAddress":"<vault from Step 3>",
  "userAddress":"<your wallet from Step 1>",
  "amount":"1"
}'
```

Inspect the response:
- `description` — human-readable summary ("Deposit 1 USDC into Steakhouse USDC")
- `unsigned_tx.to` — the contract you'll call (vault address for deposit; USDC address for approve)
- `step_info.total` — if 2, there's an approve + action sequence

### Step 5 — Broadcast the approve (if step_info.total === 2)

```bash
onchainos wallet contract-call \
  --chain base \
  --to <unsigned_tx.to> \
  --input-data <unsigned_tx.data> \
  --amt <unsigned_tx.value> \
  --biz-type defi \
  --strategy morpho-cli-onchainos \
  --force
```

Wait for the tx to confirm. Then re-invoke `prepareDeposit` with the SAME params — the CLI sees the allowance is set and now returns just the deposit tx.

### Step 6 — Broadcast the deposit

Repeat the `onchainos wallet contract-call` from Step 5 with the new `unsigned_tx`. Once confirmed, run `getPositions` to verify:

```bash
npx tsx cli.ts getPositions '{"chain":"base","userAddress":"<your wallet>"}'
```

Your deposit should appear in `vaultPositions[]`. (Note: the Morpho indexer can lag 30-60s behind on-chain state — if the position doesn't appear immediately, wait and retry.)

### Common pitfalls

- **`max` keyword** is only valid on `prepareWithdraw`, `prepareRepay`, `prepareWithdrawCollateral`. On deposit/supply/borrow, provide a concrete amount.
- **Amounts are human-readable**: pass `"1000"` for $1000 USDC, NOT `"1000000000"` (that would be 1000 USDC with 6 decimals — the CLI handles decimals for you).
- **Health factor** on borrows: this skill blocks any borrow that would push HF below 1.0 (`HEALTH_FACTOR_TOO_LOW`). Stay above 1.1 for safety margin.

---

## Broadcasting a pending_sign

When any of this skill's `prepare*` tools returns a `pending_sign` result, route the
broadcast through onchainos. **Use the correct flag names** (these are real
onchainos CLI flags, verified live):

```bash
onchainos wallet contract-call \
  --chain <chain> \
  --to <unsigned_tx.to> \
  --input-data <unsigned_tx.data> \
  --amt <unsigned_tx.value> \
  --biz-type defi \
  --strategy morpho-cli-onchainos \
  --force
```

> Flag-name notes:
> - `--input-data` (NOT `--data`) — the EVM calldata hex
> - `--amt` (NOT `--value`) — native token amount in wei, default 0
> - `--biz-type` + `--strategy` populate transaction metadata (helpful for audit)
> - `--force` skips backend confirmation prompts (use only when the user has already confirmed via the agent flow)

### Multi-step flows

Morpho `prepare-*` commands often return BOTH an ERC-20 approve and the action
transaction (e.g. an `approve` to the vault, then the `deposit`). The
pending_sign envelope returns ONE tx at a time. This skill is stateless and
self-healing:

1. Agent invokes `prepareDeposit` (or any other write tool) with the user's params
2. Skill calls the Morpho CLI under the hood and inspects the returned
   `transactions[]` array — emits the FIRST tx as `pending_sign`, with a
   description that includes "After this transaction lands, re-invoke
   `prepareDeposit` with the same params to receive the next transaction."
3. Onchain OS signs+broadcasts the approve
4. Agent re-invokes `prepareDeposit` with the same params; the CLI now sees
   the allowance is sufficient and returns ONLY the deposit tx — skill emits
   THAT as pending_sign
5. Onchain OS signs+broadcasts the deposit; flow complete

`step_info` (when present) tells you the position in the sequence (`current` /
`total` / `name`) so the agent can keep the user informed.

---

## Error Code Catalog

Every transaction tool returns either `pending_sign` (success) or a `ToolError`
shape: `{status: 'error', error_code: string, message: string, suggestion?: string}`.

Standard codes (from the scaffold's `runtime.ts`):

| Code | Meaning | Suggested user action |
|---|---|---|
| `UNSUPPORTED_CHAIN` | `params.chain` is not in the chain allowlist | Use a supported chain name |
| `MISSING_PARAM` | A required field is missing | Provide the named field |
| `INVALID_PARAM` | A field has an invalid value (non-decimal amount, malformed address, mutex violation) | Fix per the error message |
| `MISSING_AMOUNT` | Neither `amount` nor `repayAll`/`withdrawAll` was provided | Provide one |
| `MISSING_ADDRESS` | A read-only tool requires an address param | Provide the wallet address |
| `INSUFFICIENT_BALANCE` | Wallet doesn't hold enough for the operation | Top up first |
| `EXCEEDS_CAP` | Action exceeds protocol-allowed limit | Reduce amount, supply more collateral, etc. |
| `NO_POSITION` | User has nothing to act on (e.g. withdraw with zero shares) | Deposit first |
| `RPC_ERROR` | Read failed (network / RPC issue) | Retry; check chain status |
| `SDK_ERROR` | Underlying protocol SDK (Morpho CLI) threw | Check error message details |

Protocol-specific codes (Morpho):

| Code | Meaning | Suggested user action |
|---|---|---|
| `CLI_NOT_FOUND` | `npx @morpho-org/cli@latest` is unavailable | Install Node.js >=18 and ensure `npx` is on PATH |
| `CLI_TIMEOUT` | The Morpho CLI did not respond within the timeout window | Retry; the public RPC may be slow |
| `CLI_PARSE_ERROR` | The CLI output could not be parsed as JSON | Re-run; if persistent, file an issue with morpho-org/cli |
| `SIMULATION_REVERTED` | `simulationOk` came back false on a `prepare-*` call | Inspect `revertReason`; commonly `ERC4626ExceededMaxWithdraw` or `ERC20: insufficient allowance` |
| `HEALTH_FACTOR_TOO_LOW` | Borrow / withdrawCollateral would drop HF below 1.0 | Reduce borrow amount / supply more collateral |
| `LIQUIDITY_SHORTFALL` | `prepare-withdraw --amount max` returned a partial; `warnings[]` calls out the shortfall | Accept the partial amount or wait for more vault liquidity |
| `NOT_FOUND` | `get-vault` / `get-market` against an unknown address/id | Verify the address/id on the right chain |

---

# morpho-cli

> **Experimental (pre-v1.0)** — Command syntax, response schemas, and available operations may change. Always verify critical outputs independently.

Query Morpho protocol data and build unsigned transactions. All commands output JSON to stdout. No private keys needed.

```bash
npx @morpho-org/cli@latest <command> [options]
```

Supported chains: `ethereum`, `base`, `arbitrum`, `optimism`, `polygon`, `unichain`, `worldchain`, `katana`, `hyperevm`, `monad`, `stable`. Every command requires `--chain`.

## Response Schemas

- **[Read commands](references/read.md)** — exact JSON shapes for query-vaults, get-vault, query-markets, get-market, get-positions, get-token-balance, health-check, get-supported-chains
- **[Write commands](references/write.md)** — exact JSON shapes for prepare-\* and simulate-transactions

## Quick Reference

```bash
# Read — query protocol state
npx @morpho-org/cli@latest query-vaults    --chain base [--asset-symbol USDC] [--asset-address 0x...] [--sort apy_desc|apy_asc|tvl_desc|tvl_asc] [--limit 5] [--skip 0] [--fields address,name,symbol,apyPct,tvl,tvlUsd,feePct]
npx @morpho-org/cli@latest get-vault       --chain base --address 0x...
npx @morpho-org/cli@latest query-markets   --chain base [--loan-asset 0x...] [--collateral-asset 0x...] [--sort-by supplyApy|borrowApy|netSupplyApy|netBorrowApy|supplyAssetsUsd|borrowAssetsUsd|totalLiquidityUsd] [--sort-direction asc|desc] [--limit 10] [--skip 0] [--fields supplyApy,borrowApy,totalSupply,totalBorrow,totalCollateral,totalLiquidity,supplyAssetsUsd,borrowAssetsUsd,collateralAssetsUsd,liquidityAssetsUsd]
npx @morpho-org/cli@latest get-market      --chain base --id 0x...
npx @morpho-org/cli@latest get-positions   --chain base --user-address 0x...
npx @morpho-org/cli@latest get-token-balance --chain base --user-address 0x... --token-address 0x...

# Write — prepare unsigned transactions (simulation runs by default; add --no-simulate to skip)
npx @morpho-org/cli@latest prepare-deposit              --chain base --vault-address 0x... --user-address 0x... --amount 1000
npx @morpho-org/cli@latest prepare-withdraw             --chain base --vault-address 0x... --user-address 0x... --amount max
npx @morpho-org/cli@latest prepare-supply               --chain base --market-id 0x... --user-address 0x... --amount 5000
npx @morpho-org/cli@latest prepare-borrow               --chain base --market-id 0x... --user-address 0x... --borrow-amount 1
npx @morpho-org/cli@latest prepare-repay                --chain base --market-id 0x... --user-address 0x... --amount max
npx @morpho-org/cli@latest prepare-supply-collateral    --chain base --market-id 0x... --user-address 0x... --amount 5000
npx @morpho-org/cli@latest prepare-withdraw-collateral  --chain base --market-id 0x... --user-address 0x... --amount max

# Simulate — standalone re-simulation or arbitrary transaction simulation
npx @morpho-org/cli@latest simulate-transactions --chain base --from 0x... --transactions '<JSON>' --analysis-context '<JSON>'

# Utility
npx @morpho-org/cli@latest health-check
npx @morpho-org/cli@latest get-supported-chains
```

## Write Workflow: Prepare → Present

Every write operation follows two steps. Simulation runs automatically inside `prepare-*`.

1. **Prepare** — run a `prepare-*` command. The CLI handles token decimals, allowances, approvals, and simulation automatically. Returns a flat `PreparedOperation` with the fields you need at the root: `operation`, `summary`, `requirements` (informational — approval txs are already in `transactions`), `transactions` (the unsigned payloads to sign), `simulated`, `simulationOk`, `totalGasUsed`, `outcome`, `warnings`. Pass `--no-simulate` to skip simulation (in which case `simulationOk`, `totalGasUsed`, and most of `outcome` will be absent).
2. **Present** — show `summary`, the `transactions` list, the key `outcome` fields (see table below), and any `warnings` in tabular format. If `simulationOk` is `false`, inspect `revertReason` before presenting.

The `outcome` block is discriminated by operation type:

| Operation | `outcome` shape | Key fields to surface |
|-----------|-----------------|------------------------|
| `deposit`, `withdraw` (vaults) | `outcome.vault` | `sharesReceived`, `assetsReceived`, `positionAssets`, `positionShares` |
| `supply`, `borrow`, `repay`, `supply_collateral`, `withdraw_collateral` (markets) | `outcome.market` | `healthFactor`, `isHealthy`, `maxBorrowable`, `utilizationBeforePct` → `utilizationAfterPct`, `borrowApyBeforePct` → `borrowApyAfterPct`, plus post-operation `supplied` / `borrowed` / `collateral` (raw integer strings — divide by 10^decimals) |

Use `simulate-transactions` separately only for re-simulating with different parameters or simulating arbitrary transactions. Its top-level success field is `allSucceeded` (not `simulationOk`) — see [references/write.md](references/write.md).


## Simulation Failures

| Revert | Cause | What to do |
|--------|-------|------------|
| `ERC20: insufficient allowance` | Missing approval | Re-prepare — CLI should include approvals automatically |
| `ERC4626ExceededMaxWithdraw` | Vault liquidity insufficient | Reduce amount (see below) |
| `insufficient balance` | User lacks tokens | Tell the user |
| Custom error hex | Protocol-specific | Query state with `get-market` or `get-vault` to diagnose |

## Partial Withdrawal

When `prepare-withdraw --amount max` cannot withdraw the full balance, the CLI returns a `PreparedOperation` whose `warnings[]` calls out the liquidity shortfall. The response is still valid — it represents the largest withdrawable amount right now.

1. **Surface the warning** to the user verbatim — do not silently accept a smaller withdrawal.
2. **Offer two paths**:
   - Accept the partial amount the CLI prepared (inspect `outcome.vault.assetsReceived` for the concrete figure, optionally re-run `prepare-withdraw` with `--amount <value>` using ~99% of that figure as a safety buffer against interest accrual between prepare and execute).
   - Wait for more liquidity — locked assets unlock as underlying-market borrowers repay or the curator reallocates.
3. **Never invent an amount** by parsing the `summary` string — it is a human sentence, not a machine-readable field.

## Safety Rules

1. **Check simulation before presenting** — simulation runs by default; check `simulationOk` (for `prepare-*`) or `allSucceeded` (for `simulate-transactions`) before presenting
2. **Never sign or broadcast** — unsigned payloads only
3. **Watch health factor** for borrows — warn if below 1.1
4. **Communicate liquidity constraints** clearly for partial withdrawals

## CLI Errors

When a `npx @morpho-org/cli@latest` command fails, **stop and report the error to the user**. Do not:
- Retry with different parameters you invented
- Fall back to alternative tools or APIs
- Attempt to work around missing required options
- Pipe output through `jq` or other filters — use the CLI's built-in flags (`--fields`, `--sort-by`, `--limit`, etc.) to shape the response


## Common Mistakes

- Forgetting `--chain` — every command requires it, there is no default
- Using chain IDs (`1`, `8453`) instead of names (`ethereum`, `base`)
- Dividing `TokenAmount.value` by `10^decimals` — `TokenAmount` values are already decimal-applied (a USDC value of `"1000"` means $1,000, not 1,000 micro-USDC). `*Pct` fields are already percent-scaled; `*Usd` fields are already in dollars. The only raw integer strings are inside `outcome.market.{supplied,borrowed,collateral}` and `outcome.vault.{sharesReceived,assetsReceived,positionShares}` — those do need `/10^decimals` for display.
- Assuming 18 decimals — USDC/USDT have 6, WBTC/cbBTC have 8. Read decimals from response metadata; never assume.
- Passing raw units as `--amount` — CLI expects human-readable (`1000` not `1000000000`)
- Using `--no-simulate` without reason — simulation is on by default; only skip when debugging or for speed
- Ignoring `simulationOk === false` on `prepare-*` (or `allSucceeded === false` on `simulate-transactions`) — diagnose before presenting

---

## Onchain OS routing instructions (read before executing any tool)

The original `morpho-cli` SKILL.md above documents the Morpho CLI as a CLI
invocation. The CLI returns **unsigned** `transactions[]` arrays — Morpho
itself never signs anything. That makes this an ideal candidate for the
Onchain OS pending_sign routing pattern.

**Local-signing examples in the body above** — there are NONE. The "Safety
Rules" section explicitly says "Never sign or broadcast — unsigned payloads
only." The Morpho CLI is signing-free by design; this skill keeps it that
way.

**Routing replacements:**

- `prepare-deposit` / `prepare-withdraw` / `prepare-supply` / `prepare-borrow`
  / `prepare-repay` / `prepare-supply-collateral` /
  `prepare-withdraw-collateral` → wrapped by the corresponding
  `prepare<Op>` tool in `index.ts`. Each tool returns a `pending_sign` whose
  `next_action.tool` is `onchainos wallet contract-call`. The agent MUST
  route the broadcast through that tool — see "Broadcasting a pending_sign"
  above for the exact CLI invocation.

- `simulate-transactions` → wrapped as a **read-only** `simulateTransactions`
  tool. Returns `{status: 'ok', data: ...}`. The agent does NOT need to
  route this through Onchain OS; the simulation result is purely informational.

**pending_sign envelope shape (returned by every write tool):**

```json
{
  "status": "pending_sign",
  "unsigned_tx": { "to": "0x...", "data": "0x...", "value": "0", "chain": "base" },
  "description": "Deposit 1000 USDC into Steakhouse USDC. After this transaction lands, re-invoke prepareDeposit with the same params to receive the next transaction.",
  "next_action": { "tool": "onchainos wallet contract-call" },
  "step_info": { "current": 1, "total": 2, "name": "approve" }
}
```

**Runtime LLM MUST read**: do not execute any local-signing pattern from this
skill's body. Every transaction this skill emits is unsigned — sign+broadcast
exclusively through `onchainos wallet contract-call` as named in
`next_action.tool`. The `step_info` field tells you when a multi-step
flow is partway through; re-invoke the same tool with the same params after
each tx lands to receive the next one.
