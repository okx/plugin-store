---
name: orca
description: "Concentrated liquidity AMM on Solana — swap tokens and query pools via the Whirlpools CLMM program. Trigger phrases: swap on orca, orca swap, swap tokens on solana, orca pools, get swap quote, whirlpool swap, orca dex. Chinese: Orca兑换, 在Orca上交换代币, 查询Orca流动性池, 获取兑换报价"
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

### Install orca binary (auto-injected)

```bash
if ! command -v orca >/dev/null 2>&1; then
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
  curl -fsSL "https://github.com/MigOKG/plugin-store/releases/download/plugins/orca@0.1.0/orca-${TARGET}${EXT}" -o ~/.local/bin/orca${EXT}
  chmod +x ~/.local/bin/orca${EXT}
fi
```

### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/orca"
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
    -d '{"name":"orca","version":"0.1.0"}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"orca","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


## Architecture

- Read ops (`get-pools`, `get-quote`) → direct Orca REST API calls (`https://api.orca.so/v1`); no wallet needed, no confirmation required
- Write ops (`swap`) → after user confirmation, submits via `onchainos dex swap execute --chain 501`
- Chain: Solana mainnet (chain ID 501)
- Program: Orca Whirlpools (`whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`)

## Commands

### get-pools — Query Whirlpool Pools

List all Orca Whirlpool pools for a token pair, sorted by TVL.

```bash
orca get-pools \
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
orca get-pools \
  --token-a So11111111111111111111111111111111111111112 \
  --token-b EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
```

**Output fields:** `address`, `token_a_symbol`, `token_b_symbol`, `fee_rate_pct`, `price`, `tvl_usd`, `volume_24h_usd`, `fee_apr_24h_pct`, `total_apr_24h_pct`

---

### get-quote — Get Swap Quote

Calculate an estimated swap output for a given input amount on Orca.

```bash
orca get-quote \
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
orca get-quote \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5 \
  --slippage-bps 50
```

**Output fields:** `estimated_amount_out`, `minimum_amount_out`, `slippage_bps`, `fee_rate_pct`, `price`, `pool_address`, `pool_tvl_usd`, `estimated_price_impact_pct`

---

### swap — Execute Token Swap

Execute a token swap on Orca via `onchainos dex swap execute`.

**Pre-swap safety checks:**
1. Security scan of output token via `onchainos security token-scan`
2. Price impact check: warns at >2%, blocks at >10%
3. **Ask user to confirm** before executing on-chain

```bash
orca swap \
  --from-token <MINT> \
  --to-token <MINT> \
  --amount <AMOUNT> \
  [--slippage-bps <BPS>] \
  [--dry-run] \
  [--skip-security-check]
```

**Parameters:**
- `--from-token`: Input token mint address
- `--to-token`: Output token mint address
- `--amount`: Amount in human-readable units
- `--slippage-bps`: Slippage tolerance in basis points (default: 50 = 0.5%)
- `--dry-run`: Simulate only; do not broadcast transaction
- `--skip-security-check`: Bypass token security scan (not recommended)

**Execution Flow:**
1. Run with `--dry-run` first to preview
2. **Ask user to confirm** the swap details (amount, tokens, slippage) before proceeding
3. Execute only after explicit user approval
4. Report transaction hash and Solscan link

**Example:**
```bash
# Step 1: Preview
orca --dry-run swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5

# Step 2: After user confirms, execute for real
orca swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5 \
  --slippage-bps 100
```

**Output fields:** `ok`, `tx_hash`, `solscan_url`, `from_token`, `to_token`, `amount`, `slippage_bps`, `estimated_price_impact_pct`

---

## Known Token Addresses (Solana Mainnet)

| Token | Mint Address |
|-------|-------------|
| Native SOL | `11111111111111111111111111111111` |
| Wrapped SOL (wSOL) | `So11111111111111111111111111111111111111112` |
| USDC | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| USDT | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` |
| ORCA | `orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE` |

## Safety Rules

- Never swap into a token flagged as `block` by security scan
- Swaps with estimated price impact > 10% are automatically rejected
- Always run `--dry-run` first and show the quote to the user before asking for confirmation
- If pool TVL < $10,000, warn user about high slippage risk
