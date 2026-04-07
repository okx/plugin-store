---
name: velodrome-v2
description: Swap tokens and manage classic AMM (volatile/stable) LP positions on Velodrome V2 on Optimism (chain 10). Supports swap, quote, pools, positions, add-liquidity, remove-liquidity, claim-rewards.
version: 0.1.0
author: GeoGu360
tags:
  - dex
  - amm
  - velodrome
  - classic-amm
  - stable
  - volatile
  - optimism
---

# Velodrome V2 (Classic AMM Pools)

Velodrome V2 is the largest DEX on Optimism. This plugin covers the classic AMM module - volatile and stable pools using a Uniswap V2 style constant-product formula. LP tokens are standard ERC-20 tokens (not NFTs).

**Architecture:** Read-only operations (quote, pools, positions) use direct eth_call via JSON-RPC to Optimism. Write ops use `onchainos wallet contract-call --force` after user confirmation.

---

## Pre-flight Checks

```bash
# Ensure onchainos CLI is installed and wallet is configured
onchainos wallet addresses
```

The binary `velodrome-v2` must be available in your PATH.

---

## Pool Types

| Type | stable flag | Formula | Best for |
|------|-------------|---------|----------|
| Volatile | false (default) | Constant-product xyk | WETH/USDC, WETH/VELO |
| Stable | true | Low-slippage curve | USDC/DAI, USDC/USDT |

---

## Commands

### 1. `quote` - Get Swap Quote

Queries Router.getAmountsOut via eth_call (no transaction). Auto-checks both volatile and stable pools unless --stable is specified.

```bash
velodrome-v2 quote \
  --token-in WETH \
  --token-out USDC \
  --amount-in 50000000000000
```

**Specify pool type:**
```bash
velodrome-v2 quote --token-in USDC --token-out DAI --amount-in 1000000 --stable true
```

**Output:**
```json
{"ok":true,"tokenIn":"0x4200...","tokenOut":"0x0b2C...","amountIn":50000000000000,"stable":false,"pool":"0x...","amountOut":118500}
```

**Notes:**
- Validates pool exists via PoolFactory before calling getAmountsOut
- Returns best amountOut across volatile and stable pools
- USDC uses 6 decimals, WETH uses 18 decimals

---

### 2. `swap` - Swap Tokens

Executes swapExactTokensForTokens on the Velodrome V2 Router. Quotes first, then **asks user to confirm** before submitting.

```bash
velodrome-v2 swap \
  --token-in WETH \
  --token-out USDC \
  --amount-in 50000000000000 \
  --slippage 0.5
```

**With dry run (no broadcast):**
```bash
velodrome-v2 swap --token-in WETH --token-out USDC --amount-in 50000000000000 --dry-run
```

**Force stable pool:**
```bash
velodrome-v2 swap --token-in USDC --token-out DAI --amount-in 1000000 --stable true
```

**Output:**
```json
{"ok":true,"txHash":"0xabc...","tokenIn":"0x4200...","tokenOut":"0x0b2C...","amountIn":50000000000000,"stable":false,"amountOutMin":118000}
```

**Flow:**
1. PoolFactory lookup to find best pool (volatile + stable)
2. Router.getAmountsOut to get expected output
3. **Ask user to confirm** token amounts and slippage
4. Check ERC-20 allowance; approve Router if needed (3-second delay after approve)
5. Submit `wallet contract-call --force` to Router (selector `0xcac88ea9`)

**Important:** Max 0.00005 ETH per test transaction. Recipient is always the connected wallet. Never zero address in live mode.

---

### 3. `pools` - Query Pool Info

Lists classic AMM pool addresses and reserves for a token pair.

```bash
# Query both volatile and stable pools
velodrome-v2 pools --token-a WETH --token-b USDC

# Query only volatile pool
velodrome-v2 pools --token-a WETH --token-b USDC --stable false

# Query by direct pool address
velodrome-v2 pools --pool 0x...
```

**Output:**
```json
{
  "ok": true,
  "tokenA": "0x4200...",
  "tokenB": "0x0b2C...",
  "pools": [
    {"stable": false, "address": "0x...", "reserve0": "1234567890000000000", "reserve1": "3456789000", "deployed": true},
    {"stable": true, "address": "0x0000...", "deployed": false}
  ]
}
```

---

### 4. `positions` - View LP Positions

Shows ERC-20 LP token balances for common Velodrome pools or a specific pool.

```bash
# Scan common pools for connected wallet
velodrome-v2 positions

# Scan for specific wallet
velodrome-v2 positions --owner 0xYourAddress

# Check specific pool
velodrome-v2 positions --pool 0xPoolAddress

# Check specific token pair
velodrome-v2 positions --token-a WETH --token-b USDC --stable false
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
      "token1": "0x0b2C...",
      "lpBalance": "1234567890000000",
      "poolSharePct": "0.001234",
      "estimatedToken0": "567890000000",
      "estimatedToken1": "1234000"
    }
  ]
}
```

**Notes:**
- Scans common pairs (WETH/USDC volatile, WETH/VELO volatile, USDC/DAI stable, etc.) by default
- LP tokens are ERC-20, not NFTs - balances are fungible

---

### 5. `add-liquidity` - Add Liquidity

Adds liquidity to a classic AMM pool (ERC-20 LP tokens). **Ask user to confirm** before submitting.

```bash
velodrome-v2 add-liquidity \
  --token-a WETH \
  --token-b USDC \
  --stable false \
  --amount-a-desired 50000000000000 \
  --amount-b-desired 118000
```

**Auto-quote token B amount:**
```bash
# Leave --amount-b-desired at 0 to auto-quote
velodrome-v2 add-liquidity \
  --token-a WETH \
  --token-b USDC \
  --stable false \
  --amount-a-desired 50000000000000
```

**Output:**
```json
{"ok":true,"txHash":"0xdef...","tokenA":"0x4200...","tokenB":"0x0b2C...","stable":false,"amountADesired":50000000000000,"amountBDesired":118000}
```

**Flow:**
1. Verify pool exists via PoolFactory
2. Auto-quote amountB if not provided (Router.quoteAddLiquidity)
3. **Ask user to confirm** token amounts and pool type
4. Approve tokenA - Router if needed (5-second delay)
5. Approve tokenB - Router if needed (5-second delay)
6. Submit `wallet contract-call --force` for addLiquidity (selector `0x5a47ddc3`)

---

### 6. `remove-liquidity` - Remove Liquidity

Burns LP tokens to withdraw the underlying token pair. **Ask user to confirm** before submitting.

```bash
# Remove all LP tokens for WETH/USDC volatile pool
velodrome-v2 remove-liquidity \
  --token-a WETH \
  --token-b USDC \
  --stable false

# Remove specific LP amount
velodrome-v2 remove-liquidity \
  --token-a WETH \
  --token-b USDC \
  --stable false \
  --liquidity 1000000000000000
```

**Output:**
```json
{"ok":true,"txHash":"0x...","pool":"0x...","tokenA":"0x4200...","tokenB":"0x0b2C...","stable":false,"liquidityRemoved":1000000000000000}
```

**Flow:**
1. Lookup pool address from PoolFactory
2. Check LP token balance
3. **Ask user to confirm** the liquidity amount
4. Approve LP token - Router if needed (3-second delay)
5. Submit `wallet contract-call --force` for removeLiquidity (selector `0x0dede6c4`)

---

### 7. `claim-rewards` - Claim VELO Gauge Rewards

Claims accumulated VELO emissions from a pool gauge. **Ask user to confirm** before submitting.

```bash
# Claim from WETH/USDC volatile pool gauge
velodrome-v2 claim-rewards \
  --token-a WETH \
  --token-b USDC \
  --stable false

# Claim from known gauge address
velodrome-v2 claim-rewards --gauge 0xGaugeAddress
```

**Output:**
```json
{"ok":true,"txHash":"0x...","gauge":"0x...","wallet":"0x...","earnedVelo":"1234567890000000000"}
```

**Flow:**
1. Lookup pool address - Voter.gauges(pool) - gauge address
2. Gauge.earned(wallet) to check pending VELO
3. If earned = 0, exit early with no-op message
4. **Ask user to confirm** the earned amount before claiming
5. Submit `wallet contract-call --force` for getReward(wallet) (selector `0xc00007b0`)

**Notes:**
- Gauge rewards require LP tokens to be staked in the gauge (separate from just holding LP tokens)
- Use --gauge <address> for direct gauge address if pool lookup fails

---

## Supported Token Symbols (Optimism mainnet)

| Symbol | Address |
|--------|---------|
| WETH / ETH | `0x4200000000000000000000000000000000000006` |
| USDC | `0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85` |
| USDT | `0x94b008aA00579c1307B0EF2c499aD98a8ce58e58` |
| DAI | `0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1` |
| VELO | `0x9560e827aF36c94D2Ac33a39bCE1Fe78631088Db` |
| WBTC | `0x68f180fcCe6836688e9084f035309E29Bf0A2095` |
| OP | `0x4200000000000000000000000000000000000042` |
| WSTETH | `0x1F32b1c2345538c0c6f582fCB022739c4A194Ebb` |
| SNX | `0x8700dAec35aF8Ff88c16BdF0418774CB3D7599B4` |

For any other token, pass the hex address directly.

---

## Contract Addresses (Optimism, chain ID 10)

| Contract | Address |
|---------|---------|
| Router (Classic AMM) | `0xa062aE8A9c5e11aaA026fc2670B0D65cCc8B2858` |
| PoolFactory | `0xF1046053aa5682b4F9a81b5481394DA16BE5FF5a` |
| Voter | `0x41C914ee0c7E1A5edCD0295623e6dC557B5aBf3C` |
| VELO Token | `0x9560e827aF36c94D2Ac33a39bCE1Fe78631088Db` |

---

## Error Handling

| Error | Likely Cause | Fix |
|-------|-------------|-----|
| No valid pool or quote found | Pool not deployed | Use `pools` to verify; try opposite stable flag |
| Pool does not exist | Factory returns zero address | Pool not deployed; use existing pool |
| No gauge found for pool | Pool has no gauge | Pool may not have emissions; check Velodrome UI |
| No LP token balance to remove | No LP tokens held | Add liquidity first or check positions |
| onchainos: command not found | onchainos CLI not installed | Install and configure onchainos CLI |
| txHash: "pending" | Missing --force flag | Internal error - should not occur |
| Swap reverts | Insufficient allowance or amountOutMin too high | Plugin auto-approves; increase slippage tolerance |

---

## Skill Routing

- For concentrated liquidity (CLMM) on Optimism, use `velodrome-slipstream` if available
- For portfolio tracking, use `okx-defi-portfolio`
- For cross-DEX aggregated swaps, use `okx-dex-swap`
- For token price data, use `okx-dex-token`
