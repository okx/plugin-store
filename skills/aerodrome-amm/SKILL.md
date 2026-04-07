---
name: aerodrome-amm
description: Swap tokens and manage classic AMM (volatile/stable) LP positions on Aerodrome Finance on Base (chain 8453). Supports swap, quote, pools, positions, add-liquidity, remove-liquidity, claim-rewards.
version: 0.1.0
author: GeoGu360
tags:
  - dex
  - amm
  - aerodrome
  - classic-amm
  - stable
  - volatile
  - base
---

# Aerodrome AMM (Classic Pools)

Aerodrome Finance is the largest DEX on Base. This plugin covers the **classic AMM** module — volatile and stable pools using a Velodrome V2 / Uniswap V2 style constant-product formula. LP tokens are standard ERC-20 tokens (not NFTs).

**Key distinction from Aerodrome Slipstream:** The classic AMM uses `bool stable` to identify pool type, not `tickSpacing`. The router address is different (`0xcF77a3Ba...` vs `0xBE6D8f0d...`).

**Architecture:** Read-only operations (quote, pools, positions) use direct `eth_call` via JSON-RPC to `https://base-rpc.publicnode.com`. Write ops use `onchainos wallet contract-call --force` after user confirmation.

---

## Pre-flight Checks

```bash
# Ensure onchainos CLI is installed and wallet is configured
onchainos wallet addresses
```

The binary `aerodrome-amm` must be available in your PATH.

---

## Pool Types

| Type | `stable` flag | Formula | Best for |
|------|---------------|---------|----------|
| Volatile | `false` (default) | Constant-product x×y=k | WETH/USDC, WETH/AERO |
| Stable | `true` | Low-slippage curve | USDC/DAI, USDC/USDT |

---

## Commands

### 1. `quote` — Get Swap Quote

Queries Router.getAmountsOut via `eth_call` (no transaction). Auto-checks both volatile and stable pools unless `--stable` is specified.

```bash
aerodrome-amm quote \
  --token-in WETH \
  --token-out USDC \
  --amount-in 50000000000000
```

**Specify pool type:**
```bash
aerodrome-amm quote --token-in USDC --token-out DAI --amount-in 1000000 --stable true
```

**Output:**
```json
{"ok":true,"tokenIn":"0x4200...","tokenOut":"0x8335...","amountIn":50000000000000,"stable":false,"pool":"0x...","amountOut":118500}
```

**Notes:**
- Validates pool exists via PoolFactory before calling getAmountsOut
- Returns best amountOut across volatile and stable pools
- USDC uses 6 decimals, WETH uses 18 decimals

---

### 2. `swap` — Swap Tokens

Executes `swapExactTokensForTokens` on the Aerodrome classic AMM Router. Quotes first, then **asks user to confirm** before submitting.

```bash
aerodrome-amm swap \
  --token-in WETH \
  --token-out USDC \
  --amount-in 50000000000000 \
  --slippage 0.5
```

**With dry run (no broadcast):**
```bash
aerodrome-amm swap --token-in WETH --token-out USDC --amount-in 50000000000000 --dry-run
```

**Force stable pool:**
```bash
aerodrome-amm swap --token-in USDC --token-out DAI --amount-in 1000000 --stable true
```

**Output:**
```json
{"ok":true,"txHash":"0xabc...","tokenIn":"0x4200...","tokenOut":"0x8335...","amountIn":50000000000000,"stable":false,"amountOutMin":118000}
```

**Flow:**
1. PoolFactory lookup to find best pool (volatile + stable)
2. Router.getAmountsOut to get expected output
3. **Ask user to confirm** token amounts and slippage
4. Check ERC-20 allowance; approve Router if needed (3-second delay after approve)
5. Submit `wallet contract-call --force` to Router (selector `0xcac88ea9`)

**Important:** Max 0.00005 ETH (~0.1 USDC) per test transaction. Recipient is always the connected wallet. Never zero address in live mode.

---

### 3. `pools` — Query Pool Info

Lists classic AMM pool addresses and reserves for a token pair.

```bash
# Query both volatile and stable pools
aerodrome-amm pools --token-a WETH --token-b USDC

# Query only volatile pool
aerodrome-amm pools --token-a WETH --token-b USDC --stable false

# Query by direct pool address
aerodrome-amm pools --pool 0x...
```

**Output:**
```json
{
  "ok": true,
  "tokenA": "0x4200...",
  "tokenB": "0x8335...",
  "pools": [
    {"stable": false, "address": "0x...", "reserve0": "1234567890000000000", "reserve1": "3456789000", "deployed": true},
    {"stable": true, "address": "0x0000...", "deployed": false}
  ]
}
```

---

### 4. `positions` — View LP Positions

Shows ERC-20 LP token balances for common Aerodrome pools or a specific pool.

```bash
# Scan common pools for connected wallet
aerodrome-amm positions

# Scan for specific wallet
aerodrome-amm positions --owner 0xYourAddress

# Check specific pool
aerodrome-amm positions --pool 0xPoolAddress

# Check specific token pair
aerodrome-amm positions --token-a WETH --token-b USDC --stable false
```

**Output:**
```json
{
  "ok": true,
  "owner": "0x...",
  "positions": [
    {
      "pool": "0x...",
      "token0": "0x4200...",
      "token1": "0x8335...",
      "lpBalance": "1234567890000000",
      "poolSharePct": "0.001234",
      "estimatedToken0": "567890000000",
      "estimatedToken1": "1234000"
    }
  ]
}
```

**Notes:**
- Scans common pairs (WETH/USDC volatile, WETH/AERO volatile, USDC/DAI stable, etc.) by default
- LP tokens are ERC-20, not NFTs — balances are fungible
- `estimatedToken0/1` based on current pool reserves × LP share

---

### 5. `add-liquidity` — Add Liquidity

Adds liquidity to a classic AMM pool (ERC-20 LP tokens). **Ask user to confirm** before submitting.

```bash
aerodrome-amm add-liquidity \
  --token-a WETH \
  --token-b USDC \
  --stable false \
  --amount-a-desired 50000000000000 \
  --amount-b-desired 118000
```

**Auto-quote token B amount:**
```bash
# Leave --amount-b-desired at 0 to auto-quote
aerodrome-amm add-liquidity \
  --token-a WETH \
  --token-b USDC \
  --stable false \
  --amount-a-desired 50000000000000
```

**Output:**
```json
{"ok":true,"txHash":"0xdef...","tokenA":"0x4200...","tokenB":"0x8335...","stable":false,"amountADesired":50000000000000,"amountBDesired":118000}
```

**Flow:**
1. Verify pool exists via PoolFactory
2. Auto-quote amountB if not provided (Router.quoteAddLiquidity)
3. **Ask user to confirm** token amounts and pool type
4. Approve tokenA → Router if needed (5-second delay)
5. Approve tokenB → Router if needed (5-second delay)
6. Submit `wallet contract-call --force` for addLiquidity (selector `0x5a47ddc3`)

---

### 6. `remove-liquidity` — Remove Liquidity

Burns LP tokens to withdraw the underlying token pair. **Ask user to confirm** before submitting.

```bash
# Remove all LP tokens for WETH/USDC volatile pool
aerodrome-amm remove-liquidity \
  --token-a WETH \
  --token-b USDC \
  --stable false

# Remove specific LP amount
aerodrome-amm remove-liquidity \
  --token-a WETH \
  --token-b USDC \
  --stable false \
  --liquidity 1000000000000000
```

**Output:**
```json
{"ok":true,"txHash":"0x...","pool":"0x...","tokenA":"0x4200...","tokenB":"0x8335...","stable":false,"liquidityRemoved":1000000000000000}
```

**Flow:**
1. Lookup pool address from PoolFactory
2. Check LP token balance
3. **Ask user to confirm** the liquidity amount
4. Approve LP token → Router if needed (3-second delay)
5. Submit `wallet contract-call --force` for removeLiquidity (selector `0x0dede6c4`)

---

### 7. `claim-rewards` — Claim AERO Gauge Rewards

Claims accumulated AERO emissions from a pool gauge. **Ask user to confirm** before submitting.

```bash
# Claim from WETH/USDC volatile pool gauge
aerodrome-amm claim-rewards \
  --token-a WETH \
  --token-b USDC \
  --stable false

# Claim from known gauge address
aerodrome-amm claim-rewards --gauge 0xGaugeAddress
```

**Output:**
```json
{"ok":true,"txHash":"0x...","gauge":"0x...","wallet":"0x...","earnedAero":"1234567890000000000"}
```

**Flow:**
1. Lookup pool address → Voter.gauges(pool) → gauge address
2. Gauge.earned(wallet) to check pending AERO
3. If earned = 0, exit early with no-op message
4. **Ask user to confirm** the earned amount before claiming
5. Submit `wallet contract-call --force` for getReward(wallet) (selector `0xc00007b0`)

**Notes:**
- Gauge rewards require LP tokens to be staked in the gauge (separate from just holding LP tokens)
- Use `--gauge <address>` for direct gauge address if pool lookup fails

---

## Supported Token Symbols (Base mainnet)

| Symbol | Address |
|--------|---------|
| WETH / ETH | `0x4200000000000000000000000000000000000006` |
| USDC | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` |
| CBBTC | `0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf` |
| AERO | `0x940181a94A35A4569E4529A3CDfB74e38FD98631` |
| DAI | `0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb` |
| USDT | `0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2` |
| WSTETH | `0xc1CBa3fCea344f92D9239c08C0568f6F2F0ee452` |

For any other token, pass the hex address directly (e.g. `--token-in 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`).

---

## Contract Addresses (Base, chain ID 8453)

| Contract | Address |
|---------|---------|
| Router (Classic AMM) | `0xcF77a3Ba9A5CA399B7c97c74d54e5b1Beb874E43` |
| PoolFactory | `0x420DD381b31aEf6683db6B902084cB0FFECe40Da` |
| Voter | `0x16613524E02ad97eDfeF371bC883F2F5d6C480A5` |
| AERO Token | `0x940181a94A35A4569E4529A3CDfB74e38FD98631` |

**Note:** These are the classic AMM contracts, distinct from Aerodrome Slipstream (CLMM) contracts.

---

## Error Handling

| Error | Likely Cause | Fix |
|-------|-------------|-----|
| `No valid pool or quote found` | Pool not deployed | Use `pools` to verify; try opposite stable flag |
| `Pool does not exist for .../stable=...` | Factory returns zero address | Pool not deployed; use existing pool |
| `No gauge found for pool` | Pool has no gauge | Pool may not have emissions; check Aerodrome UI |
| `No LP token balance to remove` | No LP tokens held | Add liquidity first or check positions |
| `onchainos: command not found` | onchainos CLI not installed | Install and configure onchainos CLI |
| `txHash: "pending"` | Missing `--force` flag | Internal error — should not occur |
| Swap reverts | Insufficient allowance or amountOutMin too high | Plugin auto-approves; increase slippage tolerance |

---

## Skill Routing

- For CLMM / concentrated liquidity on Aerodrome, use `aerodrome-slipstream` instead
- For portfolio tracking, use `okx-defi-portfolio`
- For cross-DEX aggregated swaps, use `okx-dex-swap`
- For token price data, use `okx-dex-token`
