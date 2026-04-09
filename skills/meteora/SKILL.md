---
name: meteora
description: "Meteora DLMM plugin for Solana — search liquidity pools, get swap quotes, view user positions, and execute token swaps. Trigger phrases: Meteora swap, swap on Meteora, find Meteora pool, Meteora DLMM, check my Meteora positions. Chinese: Meteora换币, 查询Meteora流动池, 在Meteora上兑换代币"
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Install onchainos CLI + Skills (auto-injected)

```bash
# 1. Install onchainos CLI
onchainos --version 2>/dev/null || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh

# 2. Install onchainos skills (enables AI agent to use onchainos commands)
npx skills add okx/onchainos-skills --yes --global

# 3. Install plugin-store skills (enables plugin discovery and management)
npx skills add MigOKG/plugin-store --skill plugin-store --yes --global
```

### Install meteora binary (auto-injected)

```bash
if ! command -v meteora >/dev/null 2>&1; then
  OS=$(uname -s | tr A-Z a-z)
  ARCH=$(uname -m)
  EXT=""
  case "${OS}_${ARCH}" in
    darwin_arm64)  TARGET="aarch64-apple-darwin" ;;
    darwin_x86_64) TARGET="x86_64-apple-darwin" ;;
    linux_x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
    linux_aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
    mingw*_x86_64|msys*_x86_64|cygwin*_x86_64) TARGET="x86_64-pc-windows-msvc"; EXT=".exe" ;;
  esac
  mkdir -p ~/.local/bin
  curl -fsSL "https://github.com/MigOKG/plugin-store/releases/download/plugins/meteora@0.1.0/meteora-${TARGET}${EXT}" -o ~/.local/bin/meteora${EXT}
  chmod +x ~/.local/bin/meteora${EXT}
fi
```

### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/meteora"
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
    -d '{"name":"meteora","version":"0.1.0"}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"meteora","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


## Architecture

- **Read operations** (`get-pools`, `get-pool-detail`, `get-swap-quote`, `get-user-positions`) → direct REST API calls to `https://dlmm.datapi.meteora.ag`; no wallet or confirmation needed
- **Write operations** (`swap`) → after user confirmation, executes via `onchainos swap execute --chain solana`; CLI handles signing and broadcast automatically (no `--force` needed on Solana)

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

**Examples:**
```
meteora get-swap-quote --from-token So11111111111111111111111111111111111111112 --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --amount 1.0
```

---

### get-user-positions — View LP positions

View a user's DLMM LP positions including token amounts, bin ranges, and unclaimed fees.

```
meteora get-user-positions [--wallet <address>] [--pool <pool_address>]
```

If `--wallet` is omitted, uses the currently logged-in onchainos wallet.

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
1. Run with `--dry-run` to preview the quote without submitting a transaction
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
