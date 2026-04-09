---
name: pump-fun
description: "Interact with pump.fun bonding curves on Solana: buy tokens, sell tokens, create new tokens, and check prices/bonding progress. Trigger phrases: buy pump.fun token, sell pump.fun token, create token pump.fun, check pump.fun price, pump.fun bonding curve. Chinese: 购买pump.fun代币, 出售pump.fun代币, 创建pump.fun代币, 查询pump.fun价格"
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

### Install pump-fun binary (auto-injected)

```bash
if ! command -v pump-fun >/dev/null 2>&1; then
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
  curl -fsSL "https://github.com/MigOKG/plugin-store/releases/download/plugins/pump-fun@0.1.0/pump-fun-${TARGET}${EXT}" -o ~/.local/bin/pump-fun${EXT}
  chmod +x ~/.local/bin/pump-fun${EXT}
fi
```

### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/pump-fun"
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
    -d '{"name":"pump-fun","version":"0.1.0"}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"pump-fun","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


## Architecture

- **Read ops** (`get-token-info`, `get-price`) → query Solana RPC directly via `pumpfun` Rust crate; no confirmation needed
- **Write ops** (`buy`, `sell`, `create-token`) → build `VersionedTransaction` via `pumpfun` crate, serialize to base64, then after user confirmation, submit via `onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx>`
- Graduated tokens (`complete == true`) → redirected to `onchainos dex swap execute --chain 501`

## Chain

Solana mainnet (chain 501). Program: `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`

## Execution Flow for Write Operations

1. Run with `--dry-run` first to preview the operation
2. **Ask user to confirm** before executing on-chain
3. Execute only after explicit user approval
4. Report transaction hash (Solana signature) and outcome

---

## Operations

### get-token-info — Fetch bonding curve state

Reads on-chain `BondingCurveAccount` for a token and returns reserves, price, market cap, and graduation progress.

```bash
pump-fun get-token-info --mint <MINT_ADDRESS>
```

**Parameters:**
- `--mint` (required): Token mint address (base58)
- `--rpc-url` (optional): Solana RPC URL (default: mainnet-beta public; set `HELIUS_RPC_URL` env var for production)

**Output fields:**
- `virtual_token_reserves`, `virtual_sol_reserves`, `real_token_reserves`, `real_sol_reserves`
- `token_total_supply`, `complete` (bonding curve graduated?), `creator`
- `price_sol_per_token`, `market_cap_sol`, `final_market_cap_sol`
- `graduation_progress_pct` (0–100%), `status`

---

### get-price — Get buy or sell price

Calculates the expected output for a given buy (SOL→tokens) or sell (tokens→SOL) amount.

```bash
pump-fun get-price --mint <MINT_ADDRESS> --direction buy --amount 100000000
pump-fun get-price --mint <MINT_ADDRESS> --direction sell --amount 5000000
```

**Parameters:**
- `--mint` (required): Token mint address (base58)
- `--direction` (required): `buy` or `sell`
- `--amount` (required): SOL lamports for buy; token units for sell
- `--fee-bps` (optional): Fee basis points for sell calculation (default: 100)
- `--rpc-url` (optional): Solana RPC URL

---

### buy — Buy tokens on bonding curve

Purchases tokens on a pump.fun bonding curve. Run `--dry-run` to preview, then **ask user to confirm** before proceeding.

```bash
# Preview
pump-fun buy --mint <MINT> --sol-amount 100000000 --dry-run

# Execute after user confirms
pump-fun buy --mint <MINT> --sol-amount 100000000 --slippage-bps 200
```

**Parameters:**
- `--mint` (required): Token mint address (base58)
- `--sol-amount` (required): SOL in lamports (e.g. `100000000` = 0.1 SOL)
- `--slippage-bps` (optional): Slippage tolerance in bps (default: 100)
- `--priority-fee-unit-limit` (optional): Compute unit limit (default: 200000)
- `--priority-fee-unit-price` (optional): Micro-lamports per CU (default: 1000)
- `--rpc-url` (optional): Solana RPC URL
- `--dry-run` (optional): Preview without broadcasting

**Note:** If `complete == true`, the token has graduated — use `onchainos dex swap execute --chain 501` instead.

---

### sell — Sell tokens back to bonding curve

Sells tokens back to a pump.fun bonding curve for SOL. Run `--dry-run` to preview, then **ask user to confirm** before proceeding.

```bash
# Preview
pump-fun sell --mint <MINT> --token-amount 5000000 --dry-run

# Sell a specific amount after user confirms
pump-fun sell --mint <MINT> --token-amount 5000000

# Sell ALL tokens after user confirms
pump-fun sell --mint <MINT>
```

**Parameters:**
- `--mint` (required): Token mint address (base58)
- `--token-amount` (optional): Token units to sell; omit to sell all tokens
- `--slippage-bps` (optional): Slippage tolerance in bps (default: 100)
- `--priority-fee-unit-limit` (optional): Compute unit limit (default: 200000)
- `--priority-fee-unit-price` (optional): Micro-lamports per CU (default: 1000)
- `--rpc-url` (optional): Solana RPC URL
- `--dry-run` (optional): Preview without broadcasting

---

### create-token — Deploy a new token on pump.fun

Creates a new token with bonding curve and optionally makes an initial buy. Run `--dry-run` to preview, then **ask user to confirm** before proceeding.

```bash
# Preview
pump-fun create-token \
  --name "Moon Cat" --symbol "MCAT" \
  --description "The cats are going to the moon" \
  --image-path /tmp/cat.png \
  --initial-buy-sol 500000000 \
  --dry-run

# Execute after user confirms
pump-fun create-token \
  --name "Moon Cat" --symbol "MCAT" \
  --description "The cats are going to the moon" \
  --image-path /tmp/cat.png \
  --initial-buy-sol 500000000
```

**Parameters:**
- `--name` (required): Token name
- `--symbol` (required): Token ticker symbol
- `--description` (required): Token description
- `--image-path` (required): Local path or IPFS URI for token image
- `--twitter` (optional): Twitter/X URL
- `--telegram` (optional): Telegram URL
- `--website` (optional): Website URL
- `--initial-buy-sol` (optional): SOL in lamports for initial buy after create (default: 0)
- `--slippage-bps` (optional): Slippage for initial buy (default: 100)
- `--priority-fee-unit-limit` (optional): Compute unit limit (default: 200000)
- `--priority-fee-unit-price` (optional): Micro-lamports per CU (default: 1000)
- `--rpc-url` (optional): Solana RPC URL
- `--dry-run` (optional): Preview without broadcasting

**Note:** A fresh mint keypair is generated at runtime. The public key becomes the new token's mint address.

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `HELIUS_RPC_URL` | Helius RPC endpoint (recommended for production; higher rate limits than public mainnet-beta) |

## Configuration Defaults

| Parameter | Default | Description |
|-----------|---------|-------------|
| `slippage_bps` | 100 | 1% slippage tolerance |
| `priority_fee_unit_limit` | 200,000 | Compute unit limit |
| `priority_fee_unit_price` | 1,000 | Micro-lamports per compute unit |
| `fee_bps` | 100 | pump.fun trade fee (1%) |

