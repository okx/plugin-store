---
name: meteora-plugin
description: "Meteora DLMM plugin for Solana — search liquidity pools, get swap quotes, view user positions, execute token swaps, add and remove liquidity, quickstart wallet check"
version: "0.3.7"
tags:
  - solana
  - dex
  - dlmm
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Version check (auto-injected)

> **AI Agent Instruction**: Run this version check before executing any commands from this skill. If an update is applied, re-read this SKILL.md before proceeding — the instructions may have changed.

```bash
# Check for skill updates (1-hour cache)
UPDATE_CACHE="$HOME/.plugin-store/update-cache/meteora-plugin"
CACHE_MAX=3600
LOCAL_VER="0.3.7"
DO_CHECK=true

if [ -f "$UPDATE_CACHE" ]; then
  CACHE_MOD=$(stat -f %m "$UPDATE_CACHE" 2>/dev/null || stat -c %Y "$UPDATE_CACHE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  AGE=$(( NOW - CACHE_MOD ))
  [ "$AGE" -lt "$CACHE_MAX" ] && DO_CHECK=false
fi

if [ "$DO_CHECK" = true ]; then
  REMOTE_VER=$(curl -sf --max-time 3 "https://raw.githubusercontent.com/okx/plugin-store/main/skills/meteora-plugin/plugin.yaml" | grep '^version' | head -1 | tr -d '"' | awk '{print $2}')
  if [ -n "$REMOTE_VER" ]; then
    mkdir -p "$HOME/.plugin-store/update-cache"
    echo "$REMOTE_VER" > "$UPDATE_CACHE"
  fi
fi

REMOTE_VER=$(cat "$UPDATE_CACHE" 2>/dev/null || echo "$LOCAL_VER")
if [ "$REMOTE_VER" != "$LOCAL_VER" ]; then
  echo "Update available: meteora-plugin v$LOCAL_VER -> v$REMOTE_VER. Updating..."
  npx skills add okx/plugin-store --skill meteora-plugin --yes --global 2>/dev/null || true
  echo "Updated meteora-plugin to v$REMOTE_VER. Please re-read this SKILL.md."
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

### Install meteora-plugin binary + launcher (auto-injected)

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
rm -f "$HOME/.local/bin/meteora-plugin" "$HOME/.local/bin/.meteora-plugin-core" 2>/dev/null

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
curl -fsSL "https://github.com/okx/plugin-store/releases/download/plugins/meteora-plugin@0.3.7/meteora-plugin-${TARGET}${EXT}" -o ~/.local/bin/.meteora-plugin-core${EXT}
chmod +x ~/.local/bin/.meteora-plugin-core${EXT}

# Symlink CLI name to universal launcher
ln -sf "$LAUNCHER" ~/.local/bin/meteora-plugin

# Register version
mkdir -p "$HOME/.plugin-store/managed"
echo "0.3.7" > "$HOME/.plugin-store/managed/meteora-plugin"
```

### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/meteora-plugin"
if [ ! -f "$REPORT_FLAG" ]; then
  mkdir -p "$HOME/.plugin-store/reported"
  # Device fingerprint → SHA256 → 32-char device ID
  DEV_RAW="$(hostname):$(uname -s):$(uname -m):$HOME"
  DEV_ID=$(echo -n "$DEV_RAW" | shasum -a 256 | head -c 32)
  # HMAC signature (obfuscated key, same as CLI binary)
  _K=$(echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | base64 -d 2>/dev/null || echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | openssl base64 -d)
  HMAC_SIG=$(echo -n "${_K}${DEV_ID}" | shasum -a 256 | head -c 8)
  DIV_ID="${DEV_ID}${HMAC_SIG}"
  unset _K
  # Report to Vercel stats
  curl -s -X POST "https://plugin-store-dun.vercel.app/install" \
    -H "Content-Type: application/json" \
    -d '{"name":"meteora-plugin","version":"0.3.7"}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"meteora-plugin","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


## Architecture

- **Read operations** (`get-pools`, `get-pool-detail`, `get-swap-quote`) → direct REST API calls to `https://dlmm.datapi.meteora.ag`; no wallet or confirmation needed
- **`get-user-positions`** → queries on-chain via Solana `getProgramAccounts` + BinArray accounts; computes token amounts directly from chain state; no wallet or confirmation needed
- **Swap** (`swap`) → after user confirmation, executes via `onchainos swap execute --chain solana`; CLI handles signing and broadcast automatically
- **Add liquidity** (`add-liquidity`) → builds a Solana transaction natively in Rust (initialize position + add liquidity instructions), submits via `onchainos wallet contract-call --chain 501`; uses SpotBalanced strategy distributing tokens across 70-bin position centered at active bin; auto-wraps SOL to WSOL when needed; retries once on simulation errors
- **Remove liquidity** (`remove-liquidity`) → builds `removeLiquidityByRange` + optional `claimFee` + `closePositionIfEmpty` instructions, submits via `onchainos wallet contract-call --chain 501`; 600k compute budget requested

## Supported Operations

### get-pools — List liquidity pools

Search and list Meteora DLMM pools. Supports filtering by token pair, sorting by TVL, APY, volume, and fee/TVL ratio.

```
meteora get-pools [--page <n>] [--page-size <n>] [--sort-key tvl|volume|apr|fee_tvl_ratio] [--order-by asc|desc] [--search-term <token_symbol_or_address>]
```

**Examples:**
```
meteora get-pools --search-term SOL-USDC --sort-key tvl --order-by desc
meteora get-pools --sort-key apr --order-by desc --page-size 5
```

---

### get-pool-detail — Get pool details

Retrieve full details for a specific DLMM pool: configuration, TVL, fee structure, reserves, APY.

```
meteora get-pool-detail --address <pool_address>
```

**Example:**
```
meteora get-pool-detail --address 5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6
```

---

### get-swap-quote — Get swap quote

Get an estimated swap quote for a token pair using the onchainos DEX aggregator on Solana.

```
meteora get-swap-quote --from-token <mint> --to-token <mint> --amount <readable_amount>
```

**Output fields:** `from_token`, `from_symbol`, `to_token`, `to_symbol`, `from_amount_readable`, `from_amount_raw`, `to_amount_readable` (human-readable, e.g. `"84.132157"`), `to_amount_raw`, `price_impact_pct`, `price_impact_warning`

**Examples:**
```
meteora get-swap-quote --from-token So11111111111111111111111111111111111111112 --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --amount 1.0
```

---

### get-user-positions — View LP positions

View a user's DLMM LP positions with token amounts computed from on-chain BinArray data.

```
meteora get-user-positions [--wallet <address>] [--pool <pool_address>]
```

If `--wallet` is omitted, uses the currently logged-in onchainos wallet.

**Output fields per position:** `position_address`, `pool_address`, `owner`,
  `token_x_mint`, `token_y_mint`, `token_x_amount`, `token_y_amount`,
  `token_x_decimals`, `token_y_decimals`,
  `bin_range` (lower_bin_id / upper_bin_id), `active_bins`, `source`

> Use `position_address` directly as `--position` when calling `remove-liquidity`.

**Examples:**
```
meteora get-user-positions
meteora get-user-positions --wallet GbE9k66MjLRQC7RnMCkRuSgHi3Lc8LJQXWdCmYFtGo2
meteora get-user-positions --pool 5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6
```

---

### swap — Execute a token swap

Execute a token swap on Solana via the onchainos DEX aggregator. Supports dry run mode.

```
meteora swap --from-token <mint> --to-token <mint> --amount <readable_amount> [--slippage <pct>] [--wallet <address>] [--dry-run]
```

**Execution Flow:**
1. Run with `--dry-run` to preview the quote — outputs `estimated_output` (human-readable), `estimated_output_raw`, `price_impact_pct`
2. **Ask user to confirm** the swap details (from/to tokens, amount, estimated output, slippage)
3. Execute after explicit user approval: `meteora swap --from-token ... --to-token ... --amount ...`
4. Report transaction hash and Solscan link

**Examples:**
```
# Preview swap (dry run)
meteora --dry-run swap --from-token So11111111111111111111111111111111111111112 --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --amount 1.0

# Execute swap (after user confirmation)
meteora swap --from-token So11111111111111111111111111111111111111112 --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --amount 1.0 --slippage 0.5
```

**Risk warnings:**
- Price impact > 5%: warning displayed, recommend splitting the trade
- APY > 50% on a pool: high-risk warning displayed

---

### add-liquidity — Add liquidity to a DLMM pool

Add liquidity to a Meteora DLMM pool using the SpotBalanced strategy. Creates a new position (width=70 bins, centered at the active bin) if one doesn't exist, and deposits token X and/or token Y into the specified bin range.

```
meteora add-liquidity --pool <pool_address> [--amount-x <float>] [--amount-y <float>] [--bin-range <n>] [--wallet <address>] [--dry-run]
```

**Parameters:**
- `--pool` — DLMM pool (LbPair) address (required)
- `--amount-x` — Amount of token X to deposit in human-readable units, e.g. `0.01` (default: 0)
- `--amount-y` — Amount of token Y to deposit in human-readable units, e.g. `1.5` (default: 0)
- `--bin-range` — Half-range in bins around the active bin for liquidity distribution; max 34 (default: 10)
- `--wallet` — Wallet address; omit to use the onchainos logged-in wallet
- `--dry-run` — Preview only; no transaction submitted

**Output fields:** `ok`, `pool`, `wallet`, `position`, `amount_x`, `amount_y`, `tx_hash`, `explorer_url`

**Execution Flow:**
1. Run with `--dry-run` to preview: shows position PDA, bin range, token accounts, estimated transaction
2. **Ask user to confirm** token amounts, pool, and that they understand liquidity provisioning risk
3. Execute after explicit user approval: `meteora add-liquidity --pool <addr> --amount-x ... --amount-y ...`
4. If position doesn't exist, it is initialized in the same transaction (requires ~0.06 SOL for rent)
5. Report position PDA and Solscan link

**Notes:**
- Position is always 70 bins wide (MAX_BIN_PER_POSITION), centered at the current active bin
- The wallet needs ~0.06 SOL for position account rent when creating a new position
- Liquidity distribution uses SpotBalanced strategy (proportional to current pool ratio)
- Both token amounts are maximums; actual deposited may be less depending on pool ratio

**Examples:**
```
# Preview adding liquidity to JitoSOL-USDC pool
meteora add-liquidity --pool 8skykrYgFFpQNMhqhKbZoVKXFss55uGPUXhVMfnCzqJv --amount-x 0.01 --amount-y 1.5 --dry-run

# Execute (after user confirmation)
meteora add-liquidity --pool 8skykrYgFFpQNMhqhKbZoVKXFss55uGPUXhVMfnCzqJv --amount-x 0.01 --amount-y 1.5

# Narrow range (5 bins each side instead of default 10)
meteora add-liquidity --pool <addr> --amount-x 0.1 --amount-y 10 --bin-range 5
```

---

### remove-liquidity — Remove liquidity from a DLMM position

Remove some or all liquidity from an existing Meteora DLMM position. Optionally close the position account afterwards to reclaim rent (~0.057 SOL).

```
meteora remove-liquidity --pool <pool_address> --position <position_address> [--pct <1-100>] [--close] [--wallet <address>] [--dry-run]
```

**Parameters:**
- `--pool` — DLMM pool (LbPair) address (required)
- `--position` — Position PDA address; obtain from `get-user-positions` output (required)
- `--pct` — Percentage of liquidity to remove, 1–100 (default: 100)
- `--close` — Close the position account after full removal (100%) to reclaim ~0.057 SOL rent
- `--wallet` — Wallet address; omit to use the onchainos logged-in wallet
- `--dry-run` — Preview only; no transaction submitted

**Output fields:** `ok`, `pool`, `position`, `wallet`, `pct_removed`, `position_closed`, `tx_hash`, `explorer_url`

> Use `position_address` from `get-user-positions` output directly as `--position`.

**Execution Flow:**
1. Run with `--dry-run` to preview: shows bin range, token accounts, and whether the position will be closed
2. **Ask user to confirm** — especially if `--close` is used (permanent, reclaims rent)
3. Execute after explicit user approval
4. Token X and token Y are returned to the wallet's associated token accounts (created on-chain if missing)
5. If `--close` is set and `--pct 100`, the position account is closed and ~0.057 SOL is returned

**Notes:**
- Attempting to remove from an empty position without `--close` returns `"ok": false` with a helpful tip; no on-chain call is made
- `--close` only takes effect when `--pct 100` (full removal); partial removals cannot close the position
- If the position is already empty (liquidity withdrawn) and `--close` is set, the binary automatically claims any pending fees (`claim_fee`) then closes the account (`close_position_if_empty`) in a single transaction, reclaiming rent

**Examples:**
```
# Preview removing all liquidity from a position
meteora remove-liquidity --pool 8skykrYgFFpQNMhqhKbZoVKXFss55uGPUXhVMfnCzqJv --position <position_addr> --dry-run

# Remove 50% of liquidity
meteora remove-liquidity --pool 8skykrYgFFpQNMhqhKbZoVKXFss55uGPUXhVMfnCzqJv --position <position_addr> --pct 50

# Remove all liquidity and close the position (reclaims rent)
meteora remove-liquidity --pool 8skykrYgFFpQNMhqhKbZoVKXFss55uGPUXhVMfnCzqJv --position <position_addr> --close
```

---

### quickstart — Check wallet balances and get a recommended deposit command

Check your SOL and USDC balances against the current pool state and receive a ready-to-run `add-liquidity` command based on what you can afford.

```
meteora quickstart --pool <pool_address> [--wallet <address>]
```

**Parameters:**
- `--pool` — DLMM pool (LbPair) address (required)
- `--wallet` — Wallet address; omit to use the onchainos logged-in wallet

**Output fields:** `ok`, `wallet`, `pool`, `sol_balance`, `usdc_balance`, `active_id`, `bin_step`, `token_x_mint`, `token_y_mint`, `suggestion` (one of `two_sided` / `x_only` / `y_only` / `insufficient_funds`), `recommended_command`

**Execution Flow:**
1. Reads SOL and USDC balances from the logged-in wallet
2. Fetches current pool state to determine active bin and token pair
3. Computes the maximum deposit amounts affordable with current balances
4. Returns a ready-to-run `add-liquidity` command

**Example:**
```
meteora quickstart --pool 5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6
```

---

## Token Addresses (Solana Mainnet)

| Token | Mint Address |
|-------|-------------|
| SOL (native) | `11111111111111111111111111111111` |
| Wrapped SOL | `So11111111111111111111111111111111111111112` |
| USDC | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| USDT | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` |

---

## Typical User Scenarios

### Scenario 1: Swap SOL for USDC on Meteora

```
# Step 1: Find best SOL-USDC pool
meteora get-pools --search-term SOL-USDC --sort-key tvl --order-by desc --page-size 3

# Step 2: Get swap quote
meteora get-swap-quote --from-token So11111111111111111111111111111111111111112 --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --amount 1.0

# Step 3: Preview swap (dry run)
meteora --dry-run swap --from-token So11111111111111111111111111111111111111112 --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --amount 1.0

# Step 4: Ask user to confirm, then execute
meteora swap --from-token So11111111111111111111111111111111111111112 --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --amount 1.0 --slippage 0.5
```

### Scenario 2: Check LP positions

```
# View all positions for logged-in wallet
meteora get-user-positions

# Filter by specific pool
meteora get-user-positions --pool 5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6
```

### Scenario 3: Find high-yield pools

```
# Top pools by APY
meteora get-pools --sort-key apr --order-by desc --page-size 10
```

### Scenario 4: Add liquidity to a pool

```
# Step 1: Find the pool
meteora get-pools --search-term JitoSOL-USDC --sort-key tvl --order-by desc --page-size 3

# Step 2: Preview the liquidity position
meteora add-liquidity --pool 8skykrYgFFpQNMhqhKbZoVKXFss55uGPUXhVMfnCzqJv --amount-x 0.01 --amount-y 1.5 --dry-run

# Step 3: Ask user to confirm, then execute
meteora add-liquidity --pool 8skykrYgFFpQNMhqhKbZoVKXFss55uGPUXhVMfnCzqJv --amount-x 0.01 --amount-y 1.5
```

### Scenario 5: Remove liquidity from a position

```
# Step 1: Find your positions
meteora get-user-positions

# Step 2: Preview removal (dry run)
meteora remove-liquidity --pool 8skykrYgFFpQNMhqhKbZoVKXFss55uGPUXhVMfnCzqJv --position <position_addr> --dry-run

# Step 3: Ask user to confirm, then remove all and close position
meteora remove-liquidity --pool 8skykrYgFFpQNMhqhKbZoVKXFss55uGPUXhVMfnCzqJv --position <position_addr> --close
```




