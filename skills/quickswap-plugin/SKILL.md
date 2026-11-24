---
name: QuickSwap V3 DEX
version: "0.1.2"
description: Swap tokens on QuickSwap V3 (Algebra Protocol CLMM) on Polygon
author: skylavis-sky
tags:
  - defi
  - polygon
  - quickswap
  - clmm
  - swap
chains:
  - polygon
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Version check (auto-injected)

> **AI Agent Instruction**: Run this version check before executing any commands from this skill. The check is a read-only fetch from `raw.githubusercontent.com` — it does **not** install anything. If it reports `Update available`, you MUST stop, surface the message to the user verbatim, and obtain explicit confirmation before running the suggested `npx skills add …` command. Do not run the update silently. After a confirmed update completes, re-read this SKILL.md before proceeding.

```bash
# Check for skill updates (1-hour cache).
# Network scope: this block calls raw.githubusercontent.com for the version probe only.
# It does NOT install anything; install requires user-confirmed `npx skills add` below.
UPDATE_CACHE="$HOME/.plugin-store/update-cache/quickswap-plugin"
CACHE_MAX=3600
LOCAL_VER="0.1.2"
DO_CHECK=true

if [ -f "$UPDATE_CACHE" ]; then
  CACHE_MOD=$(stat -f %m "$UPDATE_CACHE" 2>/dev/null || stat -c %Y "$UPDATE_CACHE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  AGE=$(( NOW - CACHE_MOD ))
  [ "$AGE" -lt "$CACHE_MAX" ] && DO_CHECK=false
fi

if [ "$DO_CHECK" = true ]; then
  REMOTE_VER=$(curl -sf --max-time 3 "https://raw.githubusercontent.com/okx/plugin-store/main/skills/quickswap-plugin/plugin.yaml" | grep '^version' | head -1 | tr -d '"' | awk '{print $2}')
  if [ -n "$REMOTE_VER" ]; then
    mkdir -p "$HOME/.plugin-store/update-cache"
    echo "$REMOTE_VER" > "$UPDATE_CACHE"
  fi
fi

REMOTE_VER=$(cat "$UPDATE_CACHE" 2>/dev/null || echo "$LOCAL_VER")
if [ "$REMOTE_VER" != "$LOCAL_VER" ]; then
  echo "Update available: quickswap-plugin v$LOCAL_VER -> v$REMOTE_VER."
  echo "ACTION REQUIRED: ask the user to confirm before running:"
  echo "  npx skills add okx/plugin-store --skill quickswap-plugin --global"
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

---


# QuickSwap V3 DEX

QuickSwap V3 is the leading concentrated liquidity DEX on Polygon, built on the **Algebra Protocol** (not Uniswap V3). Key difference: no fee tier parameter — fees are dynamic per pool.

**Chain:** Polygon (137)
**Contracts:**
- SwapRouter: `0xf5b509bb0909a69b1c207e495f687a596c168e12`
- Quoter: `0xa15F0D7377B2A0C0c10db057f641beD21028FC89`
- Factory: `0x411b0fAcC3489691f28ad58c47006AF5E3Ab3A28`

---

## Pre-flight Dependencies

Before using this plugin, verify both dependencies are working:

```bash
# 1. Verify plugin binary
quickswap-plugin --version
# Expected: quickswap-plugin 0.1.2

# 2. Verify onchainos wallet is configured for Polygon
onchainos wallet addresses --chain 137
# Expected: JSON with at least one address
```

If `onchainos wallet addresses` returns no addresses, configure a Polygon wallet first before proceeding.

---

## Data Trust Boundary

| Source | Used For | Trust Level |
|--------|----------|-------------|
| Polygon RPC (`polygon.publicnode.com`) | Token balances, decimals, allowances, quotes | On-chain — authoritative |
| QuickSwap subgraph (`api.thegraph.com`) | Pool TVL and volume rankings | Off-chain indexer — informational only |
| onchainos CLI | Token resolution, tx broadcast | Local CLI — trusted |

**Rule:** Never use subgraph data for trade execution amounts. All swap parameters (amountIn, amountOut, slippage) are derived from on-chain Quoter calls only.

> ⚠️ **Partial-state risk**: `swap --confirm` executes up to 3 sequential transactions (wrap → approve → swap). If a transaction confirms but the binary times out waiting for the next (60-second poll), the swap halts mid-sequence. In that case: check your WMATIC/token balance to determine what was completed, then re-run the command — the approve step is idempotent (skipped if allowance is sufficient). Do not resubmit without checking state first.

---

## Proactive Onboarding

When a user signals they are **new or just installed** this plugin — e.g. "I just installed quickswap-plugin", "how do I swap on Polygon", "what can I do with this" — **do not wait for them to ask specific questions.** Proactively walk them through the Quickstart in order, one step at a time, waiting for confirmation before proceeding to the next:

1. **Check wallet** — run `onchainos wallet addresses --chain 137`. If no address, direct them to connect via `onchainos wallet login`. Do not proceed to write operations until a wallet is confirmed.
2. **Check balance** — run `onchainos wallet balance --chain 137`. The user needs POL/MATIC for gas (~0.01 POL) plus the token they want to swap. If balance is zero, explain how to bridge via a CEX or cross-chain bridge.
3. **Token address tip** — explain that QuickSwap has two USDC tokens: **USDC.e** (`0x2791Bca1...4174`, bridged, more liquid) and native **USDC** (`0x3c499c...3359`, CCTP-bridged). Use the full address to be unambiguous.
4. **Get a quote first** — run `quickswap-plugin quote --token-in <address> --token-out <address> --amount <amount>` so the user sees the expected output before committing.
5. **Preview the swap** — run `quickswap-plugin swap` without `--confirm` so the user sees all steps (approve, swap) and calldata.
6. **Execute** — once they confirm, re-run with `--confirm`.

Do not dump all steps at once. Guide conversationally — confirm each step before moving on.

---

## Quickstart

New to QuickSwap V3? Follow these steps to go from zero to your first swap on Polygon.

### Step 1 — Connect wallet

```bash
onchainos wallet login your@email.com
onchainos wallet addresses --chain 137
```

### Step 2 — Check balances

```bash
onchainos wallet balance --chain 137
```

You need POL/MATIC for gas (~0.01 POL per swap). If balance is zero, bridge from a CEX or use a cross-chain bridge.

### Step 3 — Browse top pools

```bash
quickswap-plugin pools --limit 10
```

This shows the most liquid QuickSwap V3 pairs. When the subgraph is available it includes TVL and volume; otherwise shows top known pairs.

### Step 4 — Get a quote

Use `MATIC`/`POL` as shorthand for native MATIC, or pass token addresses directly. For USDC, note that Polygon has two versions — use the full address:

```bash
# Quote using symbols (MATIC → native USDC)
quickswap-plugin quote --token-in MATIC --token-out USDC --amount 10

# Quote using addresses (USDC.e → WETH, more liquid pair)
quickswap-plugin quote \
  --token-in 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174 \
  --token-out 0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619 \
  --amount 5
```

Expected output (USDC.e → WETH example):
```json
{
  "ok": true,
  "tokenIn": "0x2791...4174",
  "tokenOut": "0x7ceb...f619",
  "amountIn": "5.000000",
  "amountOut": "0.002187",
  "price": "0.000437",
  "chain": "Polygon"
}
```

### Step 5 — Preview a swap (safe, no broadcast)

```bash
# Preview (safe — no tx sent, no gas spent)
quickswap-plugin swap \
  --token-in 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174 \
  --token-out 0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619 \
  --amount 1
```

Output shows `"preview": true` and all planned steps (approve + swap calldata) without broadcasting.

### Step 6 — Execute the swap

```bash
# Execute (add --confirm):
quickswap-plugin swap \
  --token-in 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174 \
  --token-out 0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619 \
  --amount 1 \
  --confirm
```

Expected output: `"ok": true`, `"explorerLink": "https://polygonscan.com/tx/0x..."`. The plugin approves the SwapRouter and waits for approval to mine before submitting the swap.

> **MATIC auto-wrap:** If `--token-in MATIC` or `--token-in POL`, the plugin automatically wraps native MATIC → WMATIC first (3 transactions total: wrap + approve + swap).

### Step 7 — Verify on PolygonScan

Open the `explorerLink` from the output to verify the transaction. Gas used is typically 250,000–350,000 gas (~0.03–0.05 POL at standard gas prices).

---

## Commands

### `quickswap-plugin swap`

Swap tokens on QuickSwap V3 via the Algebra CLMM SwapRouter.

**Flags:**

| Flag | Required | Default | Description |
|------|----------|---------|-------------|
| `--token-in` | Yes | — | Input token: symbol (MATIC, USDC, WETH) or address (0x...) |
| `--token-out` | Yes | — | Output token: symbol or address |
| `--amount` | Yes | — | Amount of tokenIn (human-readable, e.g. `10.5`) |
| `--slippage` | No | `0.5` | Max slippage in percent (0–50) |
| `--from` | No | wallet default | Override sender address |
| `--confirm` | No | false | Broadcast on-chain (omit for dry-run) |

**Examples:**

```bash
# Dry-run preview (default)
quickswap-plugin swap --token-in MATIC --token-out USDC --amount 10

# Execute with custom slippage
quickswap-plugin swap --token-in USDC --token-out WETH --amount 100 --slippage 1.0 --confirm

# Swap using token addresses
quickswap-plugin swap \
  --token-in 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174 \
  --token-out 0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619 \
  --amount 50 --confirm
```

**MATIC auto-wrap:** If `--token-in` is `MATIC` or `POL`, the plugin will automatically wrap native MATIC → WMATIC before swapping. This adds one extra transaction.

---

### `quickswap-plugin quote`

Get a price quote from the on-chain Quoter without executing a transaction. No wallet required.

**Flags:**

| Flag | Required | Description |
|------|----------|-------------|
| `--token-in` | Yes | Input token symbol or address |
| `--token-out` | Yes | Output token symbol or address |
| `--amount` | Yes | Amount of tokenIn |

**Example:**

```bash
quickswap-plugin quote --token-in MATIC --token-out USDC --amount 10
quickswap-plugin quote --token-in WETH --token-out USDT --amount 0.1
```

---

### `quickswap-plugin pools`

List top QuickSwap V3 pools ordered by TVL. Data sourced from the QuickSwap subgraph; falls back to hardcoded top pools if the subgraph is unavailable.

**Flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--limit` | `10` | Number of pools to return (max 20) |

**Example:**

```bash
quickswap-plugin pools
quickswap-plugin pools --limit 5
```

---

## Execution Mode Reference

| Command | Requires `--confirm` | On-chain Effect | Notes |
|---------|---------------------|-----------------|-------|
| `quote` | No | None | Pure read — calls Quoter contract via RPC |
| `pools` | No | None | Pure read — queries subgraph |
| `swap` (no flag) | No | None | Dry-run preview only |
| `swap --confirm` | Yes | Up to 3 txs | Wrap (if MATIC) + Approve + Swap |

---

## Token Reference (Polygon)

| Symbol | Address |
|--------|---------|
| WMATIC | `0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270` |
| USDC.e | `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174` |
| USDT | `0xc2132D05D31c914a87C6611C10748AEb04B58e8F` |
| WETH | `0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619` |
| QUICK | `0x831753DD7087CaC61aB5644b308642cc1c33Dc13` |

---

## Install

```bash
LOCAL_VER="0.1.2"
curl -L "https://github.com/skylavis-sky/plugin-store/releases/download/quickswap-plugin-v${LOCAL_VER}/quickswap-plugin-linux-x86_64" \
  -o ~/.local/bin/quickswap-plugin && chmod +x ~/.local/bin/quickswap-plugin

# Verify
quickswap-plugin --version
```

For macOS (Apple Silicon):
```bash
LOCAL_VER="0.1.2"
curl -L "https://github.com/skylavis-sky/plugin-store/releases/download/quickswap-plugin-v${LOCAL_VER}/quickswap-plugin-macos-arm64" \
  -o ~/.local/bin/quickswap-plugin && chmod +x ~/.local/bin/quickswap-plugin
```
