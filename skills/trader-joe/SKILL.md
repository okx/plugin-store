---
name: trader-joe
description: "Trader Joe Liquidity Book DEX on Arbitrum. Swap tokens, get quotes, and explore liquidity pools using Trader Joe's bin-based LB protocol (V2.1/V2.2). Trigger phrases: swap tokens on Trader Joe, get Trader Joe quote, list Trader Joe pools, Trader Joe USDT to WETH, swap on LB DEX. Chinese: 在Trader Joe上兑换代币, 查询Trader Joe报价, 查看Trader Joe流动池."
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

## Architecture

- **Read ops** (quote, pools) → direct `eth_call` via Arbitrum public RPC; no confirmation needed
- **Write ops** (swap) → after user confirmation, submits via `onchainos wallet contract-call --force`
- Supports Trader Joe Liquidity Book V2.1 and V2.2 on Arbitrum (chain 42161)
- Uses LBQuoter for routing, LBFactory for pool discovery, LBRouter for swap execution

## Execution Flow for Write Operations

1. Fetch best quote from LBQuoter (`findBestPathFromAmountIn`)
2. Run with `--dry-run` first to preview calldata
3. **Ask user to confirm** before executing on-chain
4. Check ERC-20 allowance; approve LBRouter if needed (ask user to confirm approve)
5. Execute swap via `onchainos wallet contract-call --force`
6. Report transaction hash and estimated output

---

## Commands

### quote — Get swap quote

Get the best available quote for a token swap on Trader Joe Liquidity Book.

**Usage:**
```
trader-joe quote --from <TOKEN_IN> --to <TOKEN_OUT> --amount <AMOUNT> [--decimals <DECIMALS>] [--chain 42161]
```

**Parameters:**
- `--from`: Input token (symbol: USDT, WETH, USDC, WBTC, ARB; or 0x address)
- `--to`: Output token (symbol or 0x address)
- `--amount`: Human-readable amount (e.g. `0.01` for 0.01 USDT)
- `--decimals`: Decimals of input token (default: 18; use 6 for USDT/USDC)
- `--chain`: Chain ID (default: 42161)

**Example:**
```
trader-joe quote --from USDT --to WETH --amount 0.01 --decimals 6
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "tokenIn": "USDT",
    "tokenOut": "WETH",
    "amountIn": 0.01,
    "amountOut": 0.0000049,
    "amountOutRaw": "4888160864748",
    "binStep": 15,
    "version": "V2_1",
    "pair": "0xd387c40a72703B38A5181573724bcaF2Ce6038a5",
    "feeBps": 15.0
  }
}
```

---

### pools — List liquidity pools

List all Liquidity Book pools for a given token pair.

**Usage:**
```
trader-joe pools --token-x <TOKEN> --token-y <TOKEN> [--chain 42161]
```

**Parameters:**
- `--token-x`: First token (symbol or 0x address)
- `--token-y`: Second token (symbol or 0x address)

**Example:**
```
trader-joe pools --token-x WETH --token-y USDT
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "tokenX": "WETH",
    "tokenY": "USDT",
    "poolCount": 4,
    "pools": [
      {
        "binStep": 10,
        "pairAddress": "0x055f2cf6da90f14598d35c1184ed535c908de737",
        "activeId": 8375259,
        "createdByOwner": true,
        "ignoredForRouting": false
      }
    ]
  }
}
```

---

### swap — Swap tokens

Swap tokens on Trader Joe Liquidity Book using the best available route.

**Write operation** — requires user confirmation before execution.

**Usage:**
```
trader-joe swap --from <TOKEN_IN> --to <TOKEN_OUT> --amount <AMOUNT> [--decimals <DECIMALS>] [--slippage-bps <BPS>] [--chain 42161] [--dry-run]
```

**Parameters:**
- `--from`: Input token (symbol or 0x address)
- `--to`: Output token (symbol or 0x address)
- `--amount`: Human-readable amount (e.g. `0.01`)
- `--decimals`: Decimals of input token (default: 18; use 6 for USDT/USDC)
- `--slippage-bps`: Slippage tolerance in basis points (default: 50 = 0.5%)
- `--chain`: Chain ID (default: 42161)
- `--dry-run`: Preview calldata without broadcasting

**Example (preview):**
```
trader-joe swap --from USDT --to WETH --amount 0.01 --decimals 6 --dry-run
```

**Example (on-chain — ask user to confirm before running):**
```
trader-joe swap --from USDT --to WETH --amount 0.01 --decimals 6
```

**Flow:**
1. Get best quote via LBQuoter
2. Check USDT allowance for LBRouter
3. If allowance insufficient: ask user to confirm approve, then submit approve tx via `onchainos wallet contract-call --force`
4. Wait 3 seconds for approve to confirm
5. Ask user to confirm swap, then submit swap via `onchainos wallet contract-call --force`
6. Return txHash

**Output:**
```json
{
  "ok": true,
  "dry_run": false,
  "data": {
    "txHash": "0xabc123...",
    "tokenIn": "USDT",
    "tokenOut": "WETH",
    "amountIn": 0.01,
    "amountOutMin": "4864"
  }
}
```

---

## Supported Tokens (Arbitrum)

| Symbol | Address |
|--------|---------|
| WETH | `0x82aF49447D8a07e3bd95BD0d56f35241523fBab1` |
| USDT | `0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9` |
| USDC | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` |
| WBTC | `0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f` |
| ARB | `0x912CE59144191C1204E64559FE8253a0e49E6548` |

Pass any of these as `--from`/`--to` symbols, or use the full 0x address.

## Notes

- All swap operations use `--force` flag (required for DEX operations in onchainos)
- Trader Joe Liquidity Book uses a discrete bin model (not tick-based like Uniswap V3)
- `binStep` represents the price precision: binStep=15 means 0.15% price spread per bin
- The LBQuoter automatically routes through V2.1 and V2.2 pools for best price
