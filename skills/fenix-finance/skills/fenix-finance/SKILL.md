---
name: fenix-finance
description: >-
  Use when the user asks about Fenix Finance, Fenix DEX, Fenix V3,
  'swap on Fenix', 'add liquidity Fenix', 'remove liquidity Fenix',
  'Fenix LP', 'Fenix positions', 'concentrated liquidity Blast',
  'Algebra AMM Blast', 'Blast DEX swap', 'Blast liquidity',
  or mentions Fenix, FNX token, Fenix Finance on Blast.
  Covers: swap, price, positions, add-liquidity, remove-liquidity, balance.
  Do NOT use for Uniswap — use uniswap skill instead.
  Do NOT use for Thruster Finance on Blast — use thruster skill instead.
  Do NOT use for Juice Finance or Ring Protocol on Blast.
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

### Install fenix-finance binary (auto-injected)

```bash
if ! command -v fenix-finance >/dev/null 2>&1; then
  OS=$(uname -s | tr A-Z a-z)
  ARCH=$(uname -m)
  case "${OS}_${ARCH}" in
    darwin_arm64)  TARGET="aarch64-apple-darwin" ;;
    darwin_x86_64) TARGET="x86_64-apple-darwin" ;;
    linux_x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
    linux_aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
  esac
  curl -fsSL "https://github.com/MigOKG/plugin-store/releases/download/plugins/fenix-finance@0.1.0/fenix-finance-${TARGET}" -o ~/.local/bin/fenix-finance
  chmod +x ~/.local/bin/fenix-finance
fi
```

### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/fenix-finance"
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
    -d '{"name":"fenix-finance","version":"0.1.0"}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"fenix-finance","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


## Architecture

- Read ops (price, positions) — direct `eth_call` via public Blast RPC or Goldsky subgraph; no confirmation needed
- Write ops (swap, add-liquidity, remove-liquidity) — after user confirmation, submits via `onchainos wallet contract-call` with `--force`
- ERC-20 approve — encoded manually as calldata, submitted via `onchainos wallet contract-call --force`

## Execution Flow for Write Operations

1. Run with `--dry-run` first to preview calldata
2. **Ask user to confirm** before executing on-chain
3. Execute only after explicit user approval
4. Report transaction hash and outcome

---

## Commands

### price — Get swap quote

Query QuoterV2 for how much `token_out` you receive for a given `token_in` amount.

```bash
fenix-finance price \
  --token-in WETH \
  --token-out USDB \
  --amount 1
```

- No gas cost (read-only eth_call)
- Uses QuoterV2 at `0x94Ca5B835186A37A99776780BF976fAB81D84ED8`
- Validates pool exists via `AlgebraFactory.poolByPair`

---

### swap — Token swap

Swap tokens via Fenix SwapRouter using `exactInputSingle` (Algebra Integral V1, no fee tier).

```bash
# Dry run first
fenix-finance --dry-run swap \
  --token-in WETH \
  --token-out USDB \
  --amount 0.1 \
  --slippage-bps 50

# Ask user to confirm, then execute
fenix-finance swap \
  --token-in WETH \
  --token-out USDB \
  --amount 0.1 \
  --slippage-bps 50
```

Steps:
1. Validate pool via `AlgebraFactory.poolByPair`
2. Get quote from `QuoterV2.quoteExactInputSingle`
3. Check ERC-20 allowance; approve if needed (waits 3s)
4. **Ask user to confirm** before proceeding
5. Execute `SwapRouter.exactInputSingle` — selector `0xbc651188`

Token symbols: WETH, USDB, BLAST, FNX (or pass raw addresses)

---

### positions — List LP positions

Show all Fenix LP NFT positions for a wallet.

```bash
fenix-finance positions
fenix-finance positions --owner 0xYourAddress
fenix-finance positions --onchain   # force on-chain query instead of subgraph
```

- Queries Goldsky V3 subgraph by default
- Falls back to on-chain `NFPM.balanceOf` + `tokenOfOwnerByIndex` + `positions`

---

### add-liquidity — Mint LP position

Add concentrated liquidity by minting a new NFPM position.

```bash
# Dry run first
fenix-finance --dry-run add-liquidity \
  --token0 USDB \
  --token1 WETH \
  --amount0 100 \
  --amount1 0.05 \
  --tick-lower -887220 \
  --tick-upper 887220

# Ask user to confirm, then execute
fenix-finance add-liquidity \
  --token0 USDB \
  --token1 WETH \
  --amount0 100 \
  --amount1 0.05
```

Steps:
1. Sort tokens by address (token0 < token1 required)
2. Approve token0 and token1 to NFPM (5s wait between)
3. **Ask user to confirm** before proceeding
4. Execute `NFPM.mint` — selector `0x9cc1a283`

Default tick range: `-887220` to `887220` (full range).

---

### remove-liquidity — Remove LP position and collect fees

Remove all liquidity from a position and collect accrued fees.

```bash
# Dry run first
fenix-finance --dry-run remove-liquidity --token-id 1234

# Ask user to confirm, then execute
fenix-finance remove-liquidity --token-id 1234
```

Steps:
1. Fetch position data via `NFPM.positions(tokenId)`
2. If liquidity > 0: execute `NFPM.decreaseLiquidity` — selector `0x0c49ccbe` (waits 5s)
3. **Ask user to confirm** before proceeding
4. Execute `NFPM.collect` — selector `0xfc6f7865`

---

### balance — Show wallet balances

```bash
fenix-finance balance
```

Shows all token balances on Blast chain via `onchainos wallet balance`.

---

## Contract Addresses (Blast, Chain ID 81457)

| Contract | Address |
|----------|---------|
| SwapRouter | `0x2df37Cb897fdffc6B4b03d8252d85BE7C6dA9d00` |
| QuoterV2 | `0x94Ca5B835186A37A99776780BF976fAB81D84ED8` |
| AlgebraFactory | `0x7a44CD060afC1B6F4c80A2B9b37f4473E74E25Df` |
| NFPM | `0x8881b3Fb762d1D50e6172f621F107E24299AA1Cd` |
| WETH | `0x4300000000000000000000000000000000000004` |
| USDB | `0x4300000000000000000000000000000000000003` |

## Key Differences from Uniswap V3

- **No fee tier**: Algebra Integral V1 has one pool per token pair with dynamic fees
- **ExactInputSingleParams**: 7 fields `(tokenIn, tokenOut, recipient, deadline, amountIn, amountOutMinimum, limitSqrtPrice)` — no `fee` field
- **Factory**: uses `poolByPair(tokenA, tokenB)` not `getPool(tokenA, tokenB, fee)`
