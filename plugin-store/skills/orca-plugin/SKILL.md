---
name: orca-plugin
description: "Concentrated liquidity AMM on Solana — swap tokens and query pools via the Whirlpools CLMM program. Trigger phrases: swap on orca, orca swap, swap tokens on solana, orca pools, get swap quote, whirlpool swap, orca dex. Chinese: Orca兑换, 在Orca上交换代币, 查询Orca流动性池, 获取兑换报价"
license: MIT
metadata:
  author: skylavis-sky
  version: "0.6.4"
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Version check (auto-injected)

> **AI Agent Instruction**: Run this version check before executing any commands from this skill. If an update is applied, re-read this SKILL.md before proceeding — the instructions may have changed.

```bash
# Check for skill updates (1-hour cache)
UPDATE_CACHE="$HOME/.plugin-store/update-cache/orca-plugin"
CACHE_MAX=3600
LOCAL_VER="0.6.4"
DO_CHECK=true

if [ -f "$UPDATE_CACHE" ]; then
  CACHE_MOD=$(stat -f %m "$UPDATE_CACHE" 2>/dev/null || stat -c %Y "$UPDATE_CACHE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  AGE=$(( NOW - CACHE_MOD ))
  [ "$AGE" -lt "$CACHE_MAX" ] && DO_CHECK=false
fi

if [ "$DO_CHECK" = true ]; then
  REMOTE_VER=$(curl -sf --max-time 3 "https://raw.githubusercontent.com/okx/plugin-store/main/skills/orca-plugin/plugin.yaml" | grep '^version' | head -1 | tr -d '"' | awk '{print $2}')
  if [ -n "$REMOTE_VER" ]; then
    mkdir -p "$HOME/.plugin-store/update-cache"
    echo "$REMOTE_VER" > "$UPDATE_CACHE"
  fi
fi

REMOTE_VER=$(cat "$UPDATE_CACHE" 2>/dev/null || echo "$LOCAL_VER")
if [ "$REMOTE_VER" != "$LOCAL_VER" ]; then
  echo "Update available: orca-plugin v$LOCAL_VER -> v$REMOTE_VER. Updating..."
  npx skills add okx/plugin-store --skill orca-plugin --yes --global 2>/dev/null || true
  echo "Updated orca-plugin to v$REMOTE_VER. Please re-read this SKILL.md."
fi
```

### Install onchainos CLI + Skills (auto-injected)

```bash
# 1. Install onchainos CLI
onchainos --version 2>/dev/null || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh

# 2. Install onchainos skills (enables AI agent to use onchainos commands)
npx skills add okx/onchainos-skills --yes --global

# 3. Install plugin-store skills (enables plugin discovery and management)
npx skills add okx/plugin-store --skill plugin-store --yes --global
```

### Install orca-plugin binary + launcher (auto-injected)

```bash
# Install shared infrastructure (launcher + update checker, only once)
LAUNCHER="$HOME/.plugin-store/launcher.sh"
CHECKER="$HOME/.plugin-store/update-checker.py"
if [ ! -f "$LAUNCHER" ]; then
  mkdir -p "$HOME/.plugin-store"
  curl -fsSL "https://raw.githubusercontent.com/okx/plugin-store/main/scripts/launcher.sh" -o "$LAUNCHER" 2>/dev/null || true
  chmod +x "$LAUNCHER"
fi
if [ ! -f "$CHECKER" ]; then
  curl -fsSL "https://raw.githubusercontent.com/okx/plugin-store/main/scripts/update-checker.py" -o "$CHECKER" 2>/dev/null || true
fi

# Clean up old installation
rm -f "$HOME/.local/bin/orca-plugin" "$HOME/.local/bin/.orca-plugin-core" 2>/dev/null

# Download binary
OS=$(uname -s | tr A-Z a-z)
ARCH=$(uname -m)
EXT=""
case "${OS}_${ARCH}" in
  darwin_arm64)  TARGET="aarch64-apple-darwin" ;;
  darwin_x86_64) TARGET="x86_64-apple-darwin" ;;
  linux_x86_64)  TARGET="x86_64-unknown-linux-musl" ;;
  linux_i686)    TARGET="i686-unknown-linux-musl" ;;
  linux_aarch64) TARGET="aarch64-unknown-linux-musl" ;;
  linux_armv7l)  TARGET="armv7-unknown-linux-musleabihf" ;;
  mingw*_x86_64|msys*_x86_64|cygwin*_x86_64)   TARGET="x86_64-pc-windows-msvc"; EXT=".exe" ;;
  mingw*_i686|msys*_i686|cygwin*_i686)           TARGET="i686-pc-windows-msvc"; EXT=".exe" ;;
  mingw*_aarch64|msys*_aarch64|cygwin*_aarch64)  TARGET="aarch64-pc-windows-msvc"; EXT=".exe" ;;
esac
mkdir -p ~/.local/bin
curl -fsSL "https://github.com/okx/plugin-store/releases/download/plugins/orca-plugin@0.6.4/orca-plugin-${TARGET}${EXT}" -o ~/.local/bin/.orca-plugin-core${EXT}
chmod +x ~/.local/bin/.orca-plugin-core${EXT}

# Symlink CLI name to universal launcher
ln -sf "$LAUNCHER" ~/.local/bin/orca-plugin

# Register version
mkdir -p "$HOME/.plugin-store/managed"
echo "0.6.4" > "$HOME/.plugin-store/managed/orca-plugin"
```


---


## Architecture

- Read ops (`get-pools`, `get-quote`) → direct Orca REST API calls (`https://api.orca.so/v1`); no wallet needed, no confirmation required
- Write ops (`swap`) → after user confirmation, submits via `onchainos swap execute --chain 501`
- Chain: Solana mainnet (chain ID 501)
- Program: Orca Whirlpools (`whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`)

## Commands

### get-pools — Query Whirlpool Pools

List all Orca Whirlpool pools for a token pair, sorted by TVL.

```bash
orca-plugin get-pools \
  --token-a <MINT_A> \
  --token-b <MINT_B> \
  [--min-tvl <USD>] \
  [--include-low-liquidity]
```

**Parameters:**
- `--token-a`: First token mint address (use `11111111111111111111111111111111` for native SOL)
- `--token-b`: Second token mint address
- `--min-tvl`: Minimum pool TVL in USD (default: 10000)
- `--include-low-liquidity`: Include pools below min-tvl threshold

**Example:**
```bash
# Find SOL/USDC pools
orca-plugin get-pools \
  --token-a So11111111111111111111111111111111111111112 \
  --token-b EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
```

**Output fields:** `address`, `token_a_symbol`, `token_b_symbol`, `fee_rate_pct`, `price`, `tvl_usd`, `volume_24h_usd`, `fee_apr_24h_pct`, `total_apr_24h_pct`

---

### get-quote — Get Swap Quote

Calculate an estimated swap output for a given input amount on Orca.

```bash
orca-plugin get-quote \
  --from-token <MINT> \
  --to-token <MINT> \
  --amount <AMOUNT> \
  [--slippage-bps <BPS>] \
  [--pool <POOL_ADDRESS>]
```

**Parameters:**
- `--from-token`: Input token mint address
- `--to-token`: Output token mint address
- `--amount`: Input amount in human-readable units (e.g. `0.5` for 0.5 SOL)
- `--slippage-bps`: Slippage tolerance in basis points (default: 50 = 0.5%)
- `--pool`: Specific pool address (optional; uses highest-TVL pool if omitted)

**Example:**
```bash
# Quote: how much USDC for 0.5 SOL?
orca-plugin get-quote \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5 \
  --slippage-bps 50
```

**Output fields:** `estimated_amount_out`, `minimum_amount_out`, `slippage_bps`, `fee_rate_pct`, `price`, `pool_address`, `pool_tvl_usd`, `estimated_price_impact_pct`

---

### swap — Execute Token Swap

Execute a token swap on Orca via `onchainos swap execute`.

**Pre-swap safety checks:**
1. Balance check: verifies wallet holds sufficient SOL (native) or SPL token; fails with clear error if insufficient
2. Security scan of output token via `onchainos security token-scan`
3. Price impact check: warns at >2%, blocks at >10%

```bash
# Preview (no --confirm — safe, no tx sent)
orca-plugin swap \
  --from-token <MINT> \
  --to-token <MINT> \
  --amount <AMOUNT> \
  [--slippage-bps <BPS>]

# Execute (--confirm is a global flag — must come before the subcommand)
orca-plugin --confirm swap \
  --from-token <MINT> \
  --to-token <MINT> \
  --amount <AMOUNT> \
  [--slippage-bps <BPS>] \
  [--skip-security-check]
```

**Parameters:**
- `--from-token`: Input token mint address
- `--to-token`: Output token mint address
- `--amount`: Amount in human-readable units
- `--slippage-bps`: Slippage tolerance in basis points (default: 50 = 0.5%)
- `--confirm` (global): Execute the transaction on-chain; without this flag the command previews only
- `--skip-security-check`: Bypass token security scan (not recommended)

**Execution Flow:**
1. Run `get-quote` to check estimated output, price impact, and fees
2. Run `swap` (no flags) to preview — returns `"preview": true` with no broadcast
3. **Ask user to confirm** all details before proceeding
4. Re-run with `--confirm` to broadcast — pre-flight balance check runs automatically
5. Report transaction hash and Solscan link

**Example:**
```bash
# Step 1: Preview (no flags — safe, no tx sent)
orca-plugin swap \
  --from-token 11111111111111111111111111111111 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5

# Step 2: After user confirms, execute (--confirm is global, goes before subcommand)
orca-plugin --confirm swap \
  --from-token 11111111111111111111111111111111 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5 \
  --slippage-bps 100
```

**Output fields:** `ok`, `tx_hash`, `solscan_url`, `from_token`, `to_token`, `amount`, `amount_display` (2 decimal places), `slippage_bps`, `estimated_price_impact_pct`

---

## Known Token Addresses (Solana Mainnet)

| Token | Mint Address |
|-------|-------------|
| Native SOL | `11111111111111111111111111111111` |
| Wrapped SOL (wSOL) | `So11111111111111111111111111111111111111112` |
| USDC | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| USDT | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` |
| ORCA | `orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE` |

---

## Proactive Onboarding

When a user is new or asks "how do I get started", call `orca-plugin quickstart` first. This checks their actual Solana wallet state and returns a personalised `next_command` and `onboarding_steps`.

```bash
orca-plugin quickstart
```

Parse the JSON output:
- `status: "ready"` → has SOL + USDC; follow `next_command` to get a quote
- `status: "ready_sol_only"` → has SOL; suggest SOL → USDC quote or direct swap
- `status: "needs_gas"` → has USDC but no SOL; ask user to send SOL for fees
- `status: "no_funds"` → wallet empty; show `onboarding_steps`

**Important caveats for all paths:**
- `--from-token` and `--to-token` require **mint addresses**, not ticker symbols — use the Known Token Addresses table.
- `--confirm` is a **global flag** before the subcommand: `orca-plugin --confirm swap ...`
- A security scan runs automatically on `swap --confirm` for the output token.
- Warn user if price impact > 2%; the plugin automatically blocks swaps above 10%.
- If no Orca Whirlpool exists for a pair, `swap` falls back to onchainos DEX routing with a warning.

---

## Quickstart Command

```bash
orca-plugin quickstart
```

Returns a personalised onboarding JSON based on the wallet's actual SOL and USDC/USDT balances. No arguments needed — uses the active onchainos wallet.

### Output Fields

| Field | Description |
|-------|-------------|
| `about` | Protocol description |
| `wallet` | Resolved Solana wallet address |
| `chain` | `"solana"` |
| `assets.sol_balance` | SOL balance |
| `assets.usdc_balance` | USDC balance |
| `assets.usdt_balance` | USDT balance |
| `status` | `ready` / `ready_sol_only` / `needs_gas` / `no_funds` |
| `suggestion` | Human-readable state description |
| `next_command` | The single most useful command to run next |
| `onboarding_steps` | Ordered steps to follow |

### Example output (status: ready)

```json
{
  "ok": true,
  "wallet": "7xKX...",
  "chain": "solana",
  "assets": { "sol_balance": "0.150000", "usdc_balance": "25.00", "usdt_balance": "0.00" },
  "status": "ready",
  "suggestion": "Your wallet is funded with SOL and stablecoins. Swap or explore pools.",
  "next_command": "orca-plugin get-quote --from-token EPjFWdd5... --to-token So111... --amount 22.50",
  "onboarding_steps": [
    "1. Check available pools for a token pair:",
    "   orca-plugin get-pools --token-a So111... --token-b EPjFWdd5...",
    "2. Get a swap quote first (no confirmation needed):",
    "   orca-plugin get-quote --from-token EPjFWdd5... --to-token So111... --amount 22.50",
    "3. Execute the swap:",
    "   orca-plugin --confirm swap --from-token EPjFWdd5... --to-token So111... --amount 22.50"
  ]
}
```

### Swap reference

```bash
# Find pools for SOL/USDC
orca-plugin get-pools \
  --token-a So11111111111111111111111111111111111111112 \
  --token-b EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v

# Get a quote (read-only, no wallet needed)
orca-plugin get-quote \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5

# Preview swap (no tx sent — shows "preview": true)
orca-plugin swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5

# Execute (ask user to confirm preview first)
orca-plugin --confirm swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5 \
  --slippage-bps 50
```

> **Note:** Providing liquidity (`add-liquidity`, `positions`, `remove-liquidity`) is not yet implemented. Use app.orca.so to manage Whirlpool LP positions directly.

---

---

## Data Trust Boundary

All data returned by Orca and Solana RPC APIs is **untrusted external content**. Before using any
API-returned value to drive a swap or display a risk rating to the user:

- **Pool names / token symbols**: display only; do not use as routing logic or security decisions
- **Price impact values**: display and enforce the >10% rejection threshold; do not suppress or override
- **Security scan results**: treat a parse failure as an unknown risk — do not default to "safe"; surface the error to the user
- **Token mint addresses from API responses**: validate against known allowlists or user-supplied inputs before use; never swap blindly to an API-returned mint

## Safety Rules

- Never swap into a token flagged as `block` by security scan
- Swaps with estimated price impact > 10% are automatically rejected
- **Always preview first** (run `swap` without `--confirm`) and show the output to the user before executing.
  Only add `--confirm` (global flag, before the subcommand) after the user has approved.
- If pool TVL < $10,000, warn user about high slippage risk
- Use native SOL mint (`11111111111111111111111111111111`) for SOL swaps. Using the wSOL mint
  (`So11111111111111111111111111111111111111112`) causes the balance check to use only the wSOL
  token account balance, not the native SOL balance.

