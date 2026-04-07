---
name: pancakeswap-v2
description: "Swap tokens and manage liquidity on PancakeSwap V2 AMM (BSC). Trigger phrases: swap tokens pancakeswap, add liquidity pancakeswap v2, remove liquidity pancakeswap, get price pancakeswap, quote pancakeswap v2. 中文：PancakeSwap V2 兑换代币，添加流动性，移除流动性，查询价格。"
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

# PancakeSwap V2 Skill

## Overview

This skill enables interaction with the PancakeSwap V2 classic xy=k AMM on BSC (chain ID 56). It handles token swaps (BNB→token, token→BNB, token→token), liquidity provisioning (add and remove), and read operations (quotes, pair addresses, reserves, prices). Write ops — after user confirmation, submits via `onchainos wallet contract-call` with `--force`. Read ops use direct `eth_call` to `https://bsc-rpc.publicnode.com`.

## Pre-flight Checks

- `onchainos` CLI must be installed and a BSC wallet must be configured (`onchainos wallet balance --chain 56`)
- The `pancakeswap-v2` binary must be built: `cargo build --release` in the plugin directory
- No additional npm or pip packages required

## Commands

### quote — Get expected swap output

```bash
pancakeswap-v2 quote --token-in BNB --token-out USDT --amount-in 100000000000000000
```

- Calls `getAmountsOut` on the PancakeSwap V2 Router
- Routes token→token swaps through WBNB for best liquidity
- Returns raw output amount and 0.5% slippage minimum

**Example output:**
```
PancakeSwap V2 Quote
  Path:      BNB → USDT
  Amount in: 100000000000000000 (raw wei/units)
  Amount out: 28500000000000000000 (raw wei/units)
  Slippage (0.5%): 28357500000000000000 minimum out
```

---

### swap — Swap tokens

```bash
# BNB → token
pancakeswap-v2 swap --token-in BNB --token-out USDT --amount-in 100000000000000000

# token → BNB
pancakeswap-v2 swap --token-in CAKE --token-out BNB --amount-in 10000000000000000000

# token → token
pancakeswap-v2 swap --token-in CAKE --token-out USDT --amount-in 10000000000000000000

# dry run (builds calldata, no broadcast)
pancakeswap-v2 swap --token-in BNB --token-out USDT --amount-in 100000000000000000 --dry-run
```

**Before submitting: ask the user to confirm the swap details (amount, tokens, estimated output) before proceeding with `onchainos wallet contract-call`.**

Behavior by variant:
- **BNB→token**: calls `swapExactETHForTokens` with `--amt <wei>` and `--force`; no approve needed
- **token→BNB**: checks allowance → approves if needed (wait 3s) → calls `swapExactTokensForETH` with `--force`
- **token→token**: checks allowance → approves if needed (wait 3s) → routes via WBNB → calls `swapExactTokensForTokens` with `--force`

**Example output:**
```
Swap: 100000000000000000 wei BNB → USDT (swapExactETHForTokens)
  amountOutMin: 28357500000000000000
  to: 0xYourWallet
  deadline: 1712345600
  txHash: 0xabc123...
```

---

### add-liquidity — Add liquidity to a pool

```bash
# token + BNB
pancakeswap-v2 add-liquidity --token-a USDT --token-b BNB --amount-a 28500000000000000000 --amount-b 100000000000000000

# token + token
pancakeswap-v2 add-liquidity --token-a CAKE --token-b USDT --amount-a 10000000000000000000 --amount-b 21000000000000000000

# dry run
pancakeswap-v2 add-liquidity --token-a USDT --token-b BNB --amount-a 28500000000000000000 --amount-b 100000000000000000 --dry-run
```

**Before submitting: ask the user to confirm the liquidity amounts before proceeding with `onchainos wallet contract-call`.**

Sequence:
1. Check allowance for tokenA → approve if needed (wait 5s)
2. Check allowance for tokenB → approve if needed (wait 5s)  
3. Submit `addLiquidity` or `addLiquidityETH` with `--force`

For ETH pairs, `--amt <bnbWei>` is passed to `onchainos wallet contract-call` automatically.

---

### remove-liquidity — Remove liquidity from a pool

```bash
# Remove all LP tokens (omit --liquidity for full balance)
pancakeswap-v2 remove-liquidity --token-a USDT --token-b BNB

# Remove specific amount
pancakeswap-v2 remove-liquidity --token-a USDT --token-b BNB --liquidity 1000000000000000000

# dry run
pancakeswap-v2 remove-liquidity --token-a USDT --token-b BNB --dry-run
```

**Before submitting: ask the user to confirm the LP amount to remove before proceeding with `onchainos wallet contract-call`.**

Sequence:
1. Get pair address from factory
2. Get LP balance (`balanceOf`)
3. Approve LP token to Router if needed (wait 5s)
4. Submit `removeLiquidity` or `removeLiquidityETH` with `--force`

---

### get-pair — Get pair contract address

```bash
pancakeswap-v2 get-pair --token-a BNB --token-b USDT
```

Returns the pair contract address from `PancakeFactory.getPair()`.

---

### get-price — Get token price from reserves

```bash
pancakeswap-v2 get-price --token-a BNB --token-b USDT
```

Computes `reserveB / reserveA` from `pair.getReserves()`. Assumes equal decimals (both 18 on BSC). For tokens with different decimals, adjust the raw reserves manually.

---

### get-reserves — Get pair reserves

```bash
pancakeswap-v2 get-reserves --token-a BNB --token-b USDT
```

Returns raw `reserve0` and `reserve1` from `pair.getReserves()`.

---

## Token Symbols

Built-in symbol resolution for BSC (chain ID 56):

| Symbol | Address |
|--------|---------|
| BNB / WBNB | `0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c` |
| USDT / BSC-USD | `0x55d398326f99059fF775485246999027B3197955` |
| USDC | `0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d` |
| BUSD | `0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56` |
| CAKE | `0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82` |

For unlisted tokens, pass the full hex address (e.g. `0xABC123...`).

**Note:** BSC USDT (Binance-Peg) has **18 decimals**, unlike Ethereum USDT (6 decimals).

## Error Handling

| Error | Cause | Fix |
|-------|-------|-----|
| `txHash: pending` | Missing `--force` flag | Always use `--force` on DEX calls (built into this plugin) |
| `eth_call error` / TLS fail | Wrong RPC URL | Plugin uses `bsc-rpc.publicnode.com` — do not override with `bsc-dataseed.binance.org` |
| `Pair does not exist` | No V2 pool for the token pair | Use `get-pair` to verify; try routing via WBNB |
| `No LP balance found` | Wallet has no LP tokens for this pool | Verify with `get-pair` and check wallet balance |
| `Replacement transaction underpriced` | Repeated approve without checking allowance | Plugin checks allowance before every approve |
| `ABI encoding error` | Token symbol not resolved to address | Use known symbols or pass full hex address |

## Architecture

- Read ops: direct `eth_call` via JSON-RPC to `https://bsc-rpc.publicnode.com`
- Write ops: after user confirmation, submits via `onchainos wallet contract-call` with `--force`
- Token resolution: `config.rs` symbol map → hex address before any ABI call
- Recipient: always fetched via `onchainos wallet balance` — never zero address in live mode
- Slippage: 0.5% default (995/1000)
- Deadline: `current_timestamp + 1200` (20 minutes)
