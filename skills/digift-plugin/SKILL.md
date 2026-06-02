---
name: digift-plugin
version: "1.0.0"
author: digift-tech
description: "DigiFT RWA platform CLI — query products, prices, fees, calendar, whitelist, orders; build subscribe/redeem/approve transactions (outputs TxBody, does NOT sign)"
requiredTools:
  - onchainos wallet contract-call
  - onchainos gateway broadcast
tags:
  - rwa
  - tokenization
  - stablecoin
  - defi
  - subscription
  - redemption
---

## Live Trading Confirmation Protocol

These gates are **mandatory** before any `pending_sign` that results in an
on-chain transaction (subscribe, redeem, approve):

1. **Preview before every write.** Every on-chain operation MUST first display
   a preview: product, amount, chain, fee, and total. The user must confirm
   before proceeding.
2. **Live-mode switch requires typed confirmation.** Before the first on-chain
   transaction in a session, display: wallet address (`onchainos wallet addresses`),
   current balance (`onchainos wallet balance`), and a statement that on-chain
   transactions are irreversible. The user MUST reply with an unambiguous typed
   confirmation (e.g. `confirm live mode` / `确认开启实盘`).
3. **Session autonomy is bounded.** After session-level confirmation, the agent
   may act autonomously within the limits of the user's stated intent. When
   intent is ambiguous, stop and confirm.
4. **No stale submissions.** Never re-submit a previously built TxBody across
   sessions. Each session's `pending_sign` must be freshly built.
5. **Refuse on gate failure.** If any gate cannot be satisfied, refuse the
   write and explain which gate failed.

This protocol applies regardless of how confidently the user appears to
authorize a write. Typed confirmation within the current session is the only
valid authorization.

---

## Overview

The `digift` CLI is the unified interface to the DigiFT RWA platform. It covers two domains:

- **Data queries** — products, prices, fees, parameters, calendar, whitelist, orders. All data comes from the REST API; nothing is hardcoded.
- **On-chain operations** — building subscribe/redeem/approve transactions. Outputs standardized `TxBody` payloads. **Never signs or broadcasts.**

All contract addresses, token addresses, and chain configurations come from the API — query first, act second.

## Pre-flight Checks

### Verify the `digift` binary is installed

```bash
digift --version
```

The binary is installed automatically by the plugin-store pre-flight block (see auto-injected section above). If missing, re-run the install:

```bash
npx skills add okx/plugin-store --skill digift-plugin --global
```

### Verify the `DIGIFT_API_KEY` environment variable is set

| Variable | Required | Description |
|---|---|---|
| `DIGIFT_API_KEY` | **Yes** | API access key — required for every command |

```bash
[ -n "${DIGIFT_API_KEY:-}" ] || echo "DIGIFT_API_KEY not set"
```

If the variable is unset, ask the user to export it for the current shell:

```bash
export DIGIFT_API_KEY=<your-key>
```

Never log, echo, or commit the key. Never paste it into chat transcripts. Treat it as a secret on par with a private key.

## Decision Guide

### Question 1 — Are they browsing or looking for something specific?

- "What's available?" → `digift products`
- "Tell me about product EXAMPLE":
  - Token addresses and precision → `digift info EXAMPLE`
  - Issuer, ISIN, issuance details → `digift issuance EXAMPLE`
- "What chains / contract addresses?" → `digift chains`
- "What's the price / yield?" → `digift price EXAMPLE`

### Question 2 — What are the rules / parameters?

- "Subscription fees / min / max for EXAMPLE" → `digift sub-params EXAMPLE`
- "Redemption fees / min / max for EXAMPLE" → `digift red-params EXAMPLE`
- "Is trading open?" → Check `isSubscribable` / `isRedeemable` on `digift products`
- "When will my order settle?" → `digift calendar EXAMPLE [--type sub|red]`
- "Calculate fee for 1000 USDC" → Read `feeRate` / `fixedFee` / `minFee` from params and apply formula

### Question 3 — Am I set up for a transaction?

- "Am I whitelisted?" → `digift whitelist <address> --chain <chain-id>`
- "Check my balance of TOKEN" → `digift balance <token> <address> --chain <chain-id> [--rpc <rpc-url>]`
- "Check my order status" → `digift order <txhash>`
- "My order history" → `digift orders <address> [--project <project>] [--size <n>]`

### Question 4 — Build a transaction

1. **Check tradability** → `digift products` (verify `isSubscribable` / `isRedeemable`)
2. **Check whitelist** → `digift whitelist <address> --chain <chain-id>`
3. **Check balance** → `digift balance <token> <address> --chain <chain-id>`
4. **Build transaction** → `digift subscribe` / `digift redeem` / `digift approve`
5. **Check `needsApprove`** in output → broadcast approval first if present

---

## Quick Reference

### Query commands (at a glance)

| User says | CLI command |
|-----------|-------------|
| "what products are available" / "list RWA tokens" | `digift products` |
| "what chains does DigiFT support" / "contract addresses" | `digift chains` |
| "EXAMPLE token address / precision / chain info" | `digift info EXAMPLE` |
| "who issues EXAMPLE" / "ISIN / issuance details" | `digift issuance EXAMPLE` |
| "subscription rules for EXAMPLE" / "subscribe fee / min / max" | `digift sub-params EXAMPLE` |
| "redemption rules for EXAMPLE" / "redeem fee / min / max" | `digift red-params EXAMPLE` |
| "when will my order settle" / "settlement cycle / holidays" | `digift calendar EXAMPLE [--type sub\|red]` |
| "what's the price / yield of EXAMPLE" | `digift price EXAMPLE` |
| "price history for EXAMPLE" | `digift price-history EXAMPLE` |
| "is 0x... whitelisted" / "check whitelist" | `digift whitelist <address> --chain <chain-id>` |
| "order status for tx 0x..." / "has my order settled" | `digift order <txhash>` |
| "my order history" / "list orders for 0x..." | `digift orders <address> [--project <project>] [--size <n>]` |

### On-chain commands (at a glance)

| User says | CLI command |
|-----------|-------------|
| "check balance of USDC for 0x..." / "how much EXAMPLE does 0x... have" | `digift balance <token> <address> --chain <chain-id> [--rpc <rpc-url>]` |
| "subscribe 1000 USDC to EXAMPLE" / "buy EXAMPLE" | `digift subscribe --product <tokencode> --amount <n> --from <address> --chain <chain-id> [--rpc <rpc-url>] [--currency <currency>]` |
| "redeem 100 EXAMPLE" / "sell EXAMPLE" | `digift redeem --product <tokencode> --quantity <n> --from <address> --chain <chain-id> [--rpc <rpc-url>] [--currency <currency>]` |
| "approve USDC for contract 0x..." / "authorize spending" | `digift approve --token <address> --spender <address> --amount <n> --from <address> --chain <chain-id> [--rpc <rpc-url>]` |

---

## Core Concepts

### API is the single source of truth
All chain configurations, contract addresses, token addresses, fee structures, and parameters come from the REST API. Never hardcode values. No fallback hardcoded addresses.

### Token codes vs addresses
Products are identified by token codes (e.g., `EXAMPLE`). On-chain addresses are resolved by querying `/products/{tokenCode}/chains` for ST tokens or `/platform/chains` for currencies. The CLI handles resolution automatically via `resolveToken()`.

**Resolution order:**
1. If input is a valid `0x...` address → use as-is
2. Query `/products/{code}/chains` → ST token address
3. Query `/platform/chains` → currency address (USDC, etc.)
4. Error if nothing matches

### Precision — on-chain vs API display
> **⚠️ Critical:** The API's `precision` fields (`pricePrecision`, `stPrecision`, `decimal`) are **price/display precisions only**. They describe how values are formatted in the UI. Do NOT use these for `ethers.parseUnits()` or on-chain amount conversion. Always read the actual `decimals()` from the ERC20 contract on-chain. The CLI handles this automatically — you never need to read API precision fields manually.

### Trading calendar
- Tradability is determined by `isSubscribable` / `isRedeemable` on `digift products`, **not** by the calendar.
- The calendar tells you **when settlement occurs** after a trade.
- Settlement follows a business-day calendar with configurable cycle (e.g., `T+1`).
- When `unit` is `BUSINESS_DAY`, settlement skips weekends and holidays.
- When `unit` is `CALENDAR_DAY`, settlement occurs after N calendar days regardless.

### Fee structure
Every subscription/redemption config has: `feeRate` + `fixedFee` + `minFee` + `gst`.

**Formula:** `Fee = max(amount × feeRate + fixedFee, minFee)`

- All math uses `bigint` — no floating-point on monetary values.
- Fee rate is a **decimal** (e.g., `"0.001"` = 0.1%), not a percentage.
- `gst` is display-only — not used in fee computation.
- **Subscribe:** user pays `amount + fee`; the `subscribe()` contract parameter includes the fee.
- **Redeem:** fee is informational (deducted from settlement proceeds on-chain).

### TxBody — the standardized output
All on-chain commands output a `TxBody` payload. This skill **never signs or broadcasts** — any EVM wallet can sign and broadcast the `TxBody`.

```json
{
  "to": "0x...",
  "from": "0x...",
  "data": "0x...",
  "value": "0",
  "deadline": 1716200000,
  "description": "DigiFT subscribe: ..."
}
```

| Field | Description |
|-------|-------------|
| `to` | Target contract address |
| `from` | Sender address |
| `data` | ABI-encoded function calldata |
| `value` | Native token value (always `"0"` for DigiFT operations) |
| `deadline` | Expiry timestamp (now + 30 minutes) — reverts if submitted after |
| `description` | Human-readable summary |

### Approve before action
ERC20 tokens require the spender (SubRed contract) to be approved before subscribe/redeem. The CLI automatically checks allowance and outputs `needsApprove` if needed:

- **Subscribe:** checks allowance on the **currency token** (e.g., USDC) for the SubRed contract
- **Redeem:** checks allowance on the **ST token** for the SubRed contract

If `needsApprove` is present, broadcast the approval TxBody first, wait for confirmation, then broadcast the main TxBody.

### Standard methods only
Currently only `STANDARD_SUBSCRIPTION` and `STANDARD_REDEMPTION` are supported. Flash subscription/redemption methods exist in the API but are not used by the CLI.

### Deadline
Every transaction includes a `deadline` timestamp (30 minutes from build time). Transactions submitted after the deadline will revert. The user must broadcast within 30 minutes.

---

## Commands

### Query Commands

All query commands are **read-only** — no wallet, no private key, no RPC endpoint required.

#### `digift products`
List all available RWA products.

```bash
digift products
```

Output: array of `{ tokenCode, projectName, assetType, isSubscribable, isRedeemable }`. If no products, outputs `No products found.`

#### `digift chains`
Platform chain configuration, contract addresses, and supported currencies.

```bash
digift chains
```

Key fields: `chainId`, `chainName`, `standardSubRedContractAddress`, `currencies[]`.

#### `digift info <tokencode>`
Product-specific chain info — ST token address and precision on each deployed chain.

```bash
digift info EXAMPLE
```

#### `digift issuance <tokencode>`
Product issuance details — issuer, ISIN, issue date, highlights.

```bash
digift issuance EXAMPLE
```

#### `digift sub-params <tokencode>`
Subscription parameters — fees, min/max, increment per chain and method.

```bash
digift sub-params EXAMPLE
```

**Validation rules:**
- Amount must be `>= min` and `<= max`
- Amount must be a multiple of `increment`
- Fee = `max(amount × feeRate + fixedFee, minFee)`

#### `digift red-params <tokencode>`
Redemption parameters — same structure as subscription.

```bash
digift red-params EXAMPLE
```

#### `digift calendar <tokencode> [--type sub|red]`
Trading calendar and settlement schedule.

```bash
digift calendar EXAMPLE --type sub
```

#### `digift price <tokencode>`
Current indicative price and yield.

```bash
digift price EXAMPLE
```

> **⚠️ Prices are indicative** — the actual settlement price may differ.

**Yield types:** `0`=Non-yield, `1`=Yield to Maturity, `2`=7-Day Annualized, `3`=1-Year Floating, `4`=Redemption Yield.

#### `digift price-history <tokencode>`
Historical price data.

```bash
digift price-history EXAMPLE
```

#### `digift order <txhash>`
Look up a single order by transaction hash. Tries subscription first, then redemption.

```bash
digift order 0x...
```

**Order statuses:** `Pending` → `Submitted` → `Allocating` → `ConfirmShare` → `Completed` (or `Cancelled` / `Refunded`).

#### `digift orders <address> [--project <project>] [--size <n>]`
Paginated order history for a wallet address.

```bash
digift orders 0x... --project EXAMPLE --size 10
```

### On-Chain Commands

All on-chain commands require `--chain <chain-id>`  an RPC endpoint (via `--rpc` flag, `DIGIFT_RPC_URL` env, or built-in fallback).

#### `digift balance <token> <address> --chain <chain-id> [--rpc <rpc-url>]`
Check on-chain ERC20 token balance.

```bash
digift balance USDC 0x... --chain 1
```

#### `digift subscribe --product <tokencode> --amount <n> --from <address> --chain <chain-id> [--rpc <rpc-url>] [--currency <currency>]`
Build an unsigned subscribe transaction. The CLI performs **all** pre-flight checks automatically:

1. Fetch chain info and connect to RPC
2. Get ST token address from API
3. Get subscription parameters, match config for requested currency
4. Validate amount against min/max/increment
5. Check whitelist status
6. Check on-chain currency balance (must cover amount + fee)
7. Calculate fee (bigint arithmetic)
8. Build subscribe TxBody
9. Check allowance on currency token → outputs `needsApprove` if insufficient

```bash
digift subscribe --product EXAMPLE --amount 1000 --from 0x... --chain 1 --currency USDC
```

**Key details:**
- Contract parameter `amount` = subscription amount + fee
- Amount uses currency on-chain decimals (read from ERC20, not API)
- Allowance checked on **currency token** for SubRed contract

#### `digift redeem --product <tokencode> --quantity <n> --from <address> --chain <chain-id> [--rpc <rpc-url>] [--currency <currency>]`
Build an unsigned redeem transaction.

```bash
digift redeem --product EXAMPLE --quantity 100 --from 0x... --chain 1 --currency USDC
```

**Key differences from subscribe:**

| Aspect | Subscribe | Redeem |
|--------|-----------|--------|
| Contract parameter | `amount` = total payment (USDC) | `quantity` = ST token count |
| Amount precision | Currency on-chain decimals | ST token on-chain decimals |
| Fee handling | User pays amount + fee | Fee is informational; deducted from proceeds |
| Balance check | Currency balance ≥ amount + fee | ST token balance ≥ quantity |
| Allowance check | Currency allowance ≥ amount + fee | ST token allowance ≥ quantity |

#### `digift approve --token <address> --spender <address> --amount <n> --from <address> --chain <chain-id> [--rpc <rpc-url>]`
Build an ERC20 approve transaction.

```bash
digift approve --token USDC --spender 0x... --amount 1000 --from 0x... --chain 1
```

> Prefer **exact-amount approvals** over unlimited (`type(uint256).max`).

---

## Fee Calculation

**Formula:** `Fee = max(amount × feeRate + fixedFee, minFee)`

| Field | Type | Example | Description |
|-------|------|---------|-------------|
| `rate` | `string` | `"0.001"` | Decimal rate (0.1%) |
| `fixedFee` | `string` | `"0"` | Fixed fee in currency units |
| `minFee` | `string` | `"1"` | Minimum fee floor |
| `gst` | `string` | `"0.002"` | GST — display-only, not used in computation |

- For subscriptions, user pays `amount + fee` — verify wallet balance covers the total.
- For redemptions, fee is informational — the contract deducts it from settlement proceeds.

---

## Error Handling

| Symptom / message | Cause | Resolution |
|---|---|---|
| `DIGIFT_API_KEY not set` / `401 Unauthorized` from `digift` | Env var missing or invalid | Ask the user to `export DIGIFT_API_KEY=<key>` in the current shell, then retry. Never log the key. |
| `digift: command not found` | Binary not installed | Re-run `npx skills add okx/plugin-store --skill digift-plugin --global`. |
| `Product not found` / `Unknown tokenCode` | Token code typo, or product delisted | Run `digift products` to list current tokens; verify exact `tokenCode` (case-sensitive). |
| `isSubscribable=false` / `isRedeemable=false` | Trading window closed for that product | Show the user the next trading window from `digift calendar <tokencode>`. Do not build the TxBody. |
| `Whitelist check failed` / wallet not whitelisted | KYC not completed for this wallet+chain | Direct the user to DigiFT's KYC flow; do not build the TxBody. |
| `Insufficient balance` (subscribe) | Wallet `amount + fee` exceeds token balance | Show the user the shortfall amount; let them adjust `--amount` or fund the wallet. |
| `Insufficient allowance` / `needsApprove` present in output | Spender allowance < amount | Run `digift approve --token ... --spender ... --amount ...` first, route through `onchainos wallet contract-call`, then re-run the original command. |
| `Deadline exceeded` after `pending_sign` build | TxBody older than 30 minutes | Rebuild the TxBody (re-run `digift subscribe` / `digift redeem`). Never replay a stale TxBody. |
| `RPC error` / `chain RPC unreachable` | Public RPC unreachable or rate-limited | Retry with `--rpc <alternative-rpc>` or set `DIGIFT_RPC_URL`. Do not silently fall back to a different chain. |
| `Order not found` for `digift order <txhash>` | Tx not yet indexed or wrong hash | Wait a few seconds and retry; verify the user pasted the correct hash. |

---

## Security Notices

- **NEVER sign or broadcast transactions** — this skill only builds `TxBody` payloads.
- **NEVER handle or access private keys.**
- **NEVER hardcode contract addresses, chain IDs, or decimals** — all data comes from the API.
- **NEVER display or log API keys.** If a user shares one, warn them and advise rotation.
- **ALWAYS load API keys from environment variables only.**
- **ALWAYS check whitelist status** before building subscribe/redeem.
- **ALWAYS verify token balance** before building a transaction.
- **ALWAYS broadcast approval TxBody before the main TxBody** if `needsApprove` is present.
- **Prefer exact-amount approvals** over unlimited (`type(uint256).max`).
- **Transaction deadline is 30 minutes** — user must broadcast promptly or it reverts.
- Do NOT provide investment advice — only present factual data.

## Best Practices

- **ALWAYS query the API for data** — never hardcode addresses, chain IDs, or decimals.
- **ALWAYS check `isSubscribable` / `isRedeemable`** before suggesting subscription or redemption.
- **Follow the pre-flight checklist** for subscribe/redeem: tradability → parameters → whitelist → balance → allowance → build.
- **Wait for approval confirmation** before broadcasting the main transaction.

---

## Onchain OS routing instructions (read before executing any tool)

### pending_sign mapping

Every `digift` on-chain command outputs JSON containing a `txBody`. Convert per:

```
digift CLI output           →  pending_sign
────────────────────────────────────────────────
txBody.to             →  unsigned_tx.to
txBody.data           →  unsigned_tx.data
txBody.value          →  unsigned_tx.value ("0")
txBody.from           →  (not used in pending_sign)
chainId from output   →  unsigned_tx.chain (just the number, e.g. "1")
txBody.description    →  description
```

### next_action.tool mapping

| Operation | next_action.tool |
|-----------|------------------|
| subscribe, redeem, approve | `onchainos wallet contract-call` |

### Approve-before-action sequence (Onchain OS)

When `needsApprove` is present, execute two `pending_sign` cycles:

> **⚠️ `needsApprove` is NOT a pre-built TxBody.** It is a parameter triple `{token, spender, amount}`. Run `digift approve` separately.

**Step 1 — Approve:**
1. Extract `needsApprove.token`, `.spender`, `.amount`
2. Run `digift approve --token <token> --spender <spender> --amount <amount> --from <address> --chain <chain-id>`
3. Wrap `txBody` in `pending_sign`, route to `onchainos wallet contract-call`
4. Wait for confirmation

**Step 2 — Main transaction:**
1. Run the original `digift subscribe` / `digift redeem` command
2. Wrap `txBody` in `pending_sign`, route to `onchainos wallet contract-call`

### pending_sign contract (reference)

```typescript
type PendingSign = {
  status: 'pending_sign';
  unsigned_tx: {
    to: string;       // target contract address
    data: string;     // ABI-encoded calldata
    value: string;    // native token amount (always "0" for DigiFT)
    chain: string;    // chain ID (from CLI output's chainId field)
  };
  description: string;          // human-readable summary
  next_action: { tool: string; reason?: string };
};
```

### Broadcasting

Onchain OS handles both signing AND broadcasting internally via `onchainos wallet contract-call`. Do NOT attempt to broadcast separately.

### Runtime LLM MUST read

The transaction commands output unsigned `TxBody` JSON. Do NOT instruct the user to "broadcast with your wallet." Instead:

1. Parse the CLI JSON output
2. Convert `txBody` to `pending_sign` per the mapping above
3. Return `pending_sign` to the routing layer
4. Onchain OS takes over signing + broadcasting via `onchainos wallet contract-call`
