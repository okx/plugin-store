---
name: digift-plugin
version: "1.0.1"
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


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Version check (auto-injected)

> **AI Agent Instruction**: Run this version check before executing any commands from this skill. The check is a read-only fetch from `raw.githubusercontent.com` — it does **not** install anything. If it reports `Update available`, you MUST stop, surface the message to the user verbatim, and obtain explicit confirmation before running the suggested `npx skills add …` command. Do not run the update silently. After a confirmed update completes, re-read this SKILL.md before proceeding.

```bash
# Check for skill updates (1-hour cache).
# Network scope: this block calls raw.githubusercontent.com for the version probe only.
# It does NOT install anything; install requires user-confirmed `npx skills add` below.
UPDATE_CACHE="$HOME/.plugin-store/update-cache/digift-plugin"
CACHE_MAX=3600
LOCAL_VER="1.0.0"
DO_CHECK=true

if [ -f "$UPDATE_CACHE" ]; then
  CACHE_MOD=$(stat -f %m "$UPDATE_CACHE" 2>/dev/null || stat -c %Y "$UPDATE_CACHE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  AGE=$(( NOW - CACHE_MOD ))
  [ "$AGE" -lt "$CACHE_MAX" ] && DO_CHECK=false
fi

if [ "$DO_CHECK" = true ]; then
  REMOTE_VER=$(curl -sf --max-time 3 "https://raw.githubusercontent.com/okx/plugin-store/main/skills/digift-plugin/plugin.yaml" | grep '^version' | head -1 | tr -d '"' | awk '{print $2}')
  if [ -n "$REMOTE_VER" ]; then
    mkdir -p "$HOME/.plugin-store/update-cache"
    echo "$REMOTE_VER" > "$UPDATE_CACHE"
  fi
fi

REMOTE_VER=$(cat "$UPDATE_CACHE" 2>/dev/null || echo "$LOCAL_VER")
if [ "$REMOTE_VER" != "$LOCAL_VER" ]; then
  echo "Update available: digift-plugin v$LOCAL_VER -> v$REMOTE_VER."
  echo "ACTION REQUIRED: ask the user to confirm before running:"
  echo "  npx skills add okx/plugin-store --skill digift-plugin --global"
  echo "(This contacts the npm registry and github.com/okx/plugin-store and overwrites this skill. Do NOT auto-run.)"
fi
```

### Install onchainos CLI + Skills (auto-injected)

```bash
# 1. Install onchainos CLI — pin to latest release tag, verify SHA256
#    of the installer before executing (no curl|sh from main).
if ! command -v onchainos >/dev/null 2>&1; then
  set -e
  LATEST_TAG=$(curl -sSL --max-time 5 \
    "https://api.github.com/repos/okx/onchainos-skills/releases/latest" \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
  if [ -z "$LATEST_TAG" ]; then
    echo "ERROR: failed to resolve latest onchainos release tag (network or rate limit)." >&2
    echo "       Manual install: https://github.com/okx/onchainos-skills" >&2
    exit 1
  fi

  ONCHAINOS_TMP=$(mktemp -d)
  curl -sSL --max-time 30 \
    "https://raw.githubusercontent.com/okx/onchainos-skills/${LATEST_TAG}/install.sh" \
    -o "$ONCHAINOS_TMP/install.sh"
  curl -sSL --max-time 30 \
    "https://github.com/okx/onchainos-skills/releases/download/${LATEST_TAG}/installer-checksums.txt" \
    -o "$ONCHAINOS_TMP/installer-checksums.txt"

  EXPECTED=$(awk '$2 ~ /install\.sh$/ {print $1; exit}' "$ONCHAINOS_TMP/installer-checksums.txt")
  if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL=$(sha256sum "$ONCHAINOS_TMP/install.sh" | awk '{print $1}')
  else
    ACTUAL=$(shasum -a 256 "$ONCHAINOS_TMP/install.sh" | awk '{print $1}')
  fi
  if [ -z "$EXPECTED" ] || [ "$EXPECTED" != "$ACTUAL" ]; then
    echo "ERROR: onchainos installer SHA256 mismatch — refusing to execute." >&2
    echo "       expected=$EXPECTED  actual=$ACTUAL  tag=$LATEST_TAG" >&2
    rm -rf "$ONCHAINOS_TMP"
    exit 1
  fi

  sh "$ONCHAINOS_TMP/install.sh"
  rm -rf "$ONCHAINOS_TMP"
  set +e
fi

# 2. Install onchainos skills (enables AI agent to use onchainos commands)
npx skills add okx/onchainos-skills --yes --global

# 3. Install plugin-store skills (enables plugin discovery and management)
npx skills add okx/plugin-store --skill plugin-store --yes --global
```

### Install digift npm package (auto-injected)

```bash
# Verify Node.js >= 18 + npm
command -v node >/dev/null 2>&1 || {
  echo "ERROR: Node.js >= 18 is required (install from https://nodejs.org)" >&2
  exit 1; }
NODE_MAJOR=$(node -e 'console.log(process.versions.node.split(".")[0])' 2>/dev/null || echo 0)
if [ "$NODE_MAJOR" -lt 18 ]; then
  echo "ERROR: Node.js >= 18 required (found: $(node --version 2>/dev/null))" >&2
  exit 1
fi
command -v npm >/dev/null 2>&1 || {
  echo "ERROR: npm is required (usually ships with Node.js)" >&2
  exit 1; }

# Download .tgz + checksums to a sandbox, verify SHA256 before installing.
# Fail-closed: any mismatch / missing checksum entry refuses the install.
# Matches the producer-side workflow at
# .github/workflows/plugin-publish.yml which uploads `digift.tgz`
# alongside `checksums.txt` under each release tag.
PKG_TMP=$(mktemp -d)
TAG="plugins/digift-plugin@1.0.0"

# Robust asset download. Prefer `gh release download` — see rust/go
# install block above for rationale. Falls back to raw curl on the
# `releases/download/<tag>/<file>` path if gh is not installed.
_pluginstore_dl() {
  local fname="$1" dest="$2"
  if command -v gh >/dev/null 2>&1; then
    local stage; stage=$(mktemp -d)
    if gh release download "$TAG" --repo okx/plugin-store \
         --pattern "$fname" --dir "$stage" --clobber >/dev/null 2>&1 \
       && [ -f "$stage/$fname" ]; then
      mv "$stage/$fname" "$dest" && rm -rf "$stage" && return 0
    fi
    rm -rf "$stage"
  fi
  curl -fsSL \
    "https://github.com/okx/plugin-store/releases/download/$TAG/$fname" \
    -o "$dest"
}

_pluginstore_dl "digift.tgz" "$PKG_TMP/digift.tgz" || {
  echo "ERROR: failed to download digift.tgz for digift-plugin@1.0.0" >&2
  rm -rf "$PKG_TMP"; exit 1; }
_pluginstore_dl "checksums.txt" "$PKG_TMP/checksums.txt" || {
  echo "ERROR: failed to download checksums.txt for digift-plugin@1.0.0" >&2
  rm -rf "$PKG_TMP"; exit 1; }

EXPECTED=$(awk -v b="digift.tgz" '$2 == b {print $1; exit}' "$PKG_TMP/checksums.txt")
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL=$(sha256sum "$PKG_TMP/digift.tgz" | awk '{print $1}')
else
  ACTUAL=$(shasum -a 256 "$PKG_TMP/digift.tgz" | awk '{print $1}')
fi
if [ -z "$EXPECTED" ] || [ "$EXPECTED" != "$ACTUAL" ]; then
  echo "ERROR: digift.tgz SHA256 mismatch — refusing to install." >&2
  echo "       expected=$EXPECTED  actual=$ACTUAL" >&2
  rm -rf "$PKG_TMP"; exit 1
fi

# Install globally (npm wires up CLI commands from package.json's `bin` field) + clean up
npm install -g "$PKG_TMP/digift.tgz"
rm -rf "$PKG_TMP"

# Register version
mkdir -p "$HOME/.plugin-store/managed"
echo "1.0.0" > "$HOME/.plugin-store/managed/digift-plugin"
```

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
