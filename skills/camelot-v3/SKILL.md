---
name: camelot-v3
description: Swap tokens and manage concentrated liquidity positions on Camelot V3 (Algebra V1 AMM on Arbitrum)
---

# Camelot V3 Skill

Camelot V3 is Arbitrum's native concentrated liquidity DEX, built on the Algebra V1 protocol. It supports token swaps, price quotes, and LP position management on Arbitrum (chain 42161).

## Key Differences from Uniswap V3
- **Single pool per token pair** — no fee tier selection needed
- **Algebra protocol** — slightly different contract ABIs
- All operations are on **Arbitrum** (chain ID 42161)

## Available Commands

### quote — Get a swap price quote (read-only)

Get the estimated output amount for swapping tokens on Camelot V3.

**Trigger examples:**
- "How much USDT can I get for 0.001 ETH on Camelot?"
- "Quote WETH to USDT on Camelot V3"
- "Check the price of swapping USDT for WETH on Arbitrum"

**Usage:**
```
camelot-v3 quote --token-in <TOKEN> --token-out <TOKEN> --amount-in <RAW_AMOUNT> [--chain 42161]
```

**Parameters:**
- `--token-in` — Input token symbol (WETH, USDT, USDC, ARB) or hex address
- `--token-out` — Output token symbol or hex address
- `--amount-in` — Amount in raw units (e.g. `1000000` for 1 USDT with 6 decimals)
- `--chain` — Chain ID (default: 42161 for Arbitrum)

**Example:**
```
camelot-v3 quote --token-in WETH --token-out USDT --amount-in 1000000000000000 --chain 42161
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "pool": "0x...",
    "token_in": "0x82aF...",
    "token_out": "0xfd08...",
    "amount_in": "1000000000000000",
    "amount_in_human": "0.001000",
    "amount_out": "2036000",
    "amount_out_human": "2.036000",
    "chain_id": 42161
  }
}
```

---

### swap — Execute a token swap

Swap tokens on Camelot V3. Requires user confirmation before broadcasting.

**Trigger examples:**
- "Swap 1 USDT for WETH on Camelot"
- "Buy WETH with USDT on Arbitrum using Camelot V3"
- "Execute a swap on Camelot V3"

**Usage:**
```
camelot-v3 swap --token-in <TOKEN> --token-out <TOKEN> --amount-in <RAW_AMOUNT> [--slippage 0.5] [--chain 42161] [--dry-run]
```

**Parameters:**
- `--token-in` — Input token symbol or hex address
- `--token-out` — Output token symbol or hex address
- `--amount-in` — Amount in raw units
- `--slippage` — Slippage tolerance percent (default: 0.5)
- `--deadline-minutes` — Transaction deadline in minutes (default: 20)
- `--chain` — Chain ID (default: 42161)
- `--dry-run` — Preview calldata without broadcasting

**Write operation — ask user to confirm the swap details before executing.**

The binary will:
1. Verify the pool exists via AlgebraFactory
2. Get a quote via QuoterV2
3. Check and set ERC-20 allowance if needed
4. Execute via `onchainos wallet contract-call --force` to Camelot V3 SwapRouter

**Example:**
```
camelot-v3 swap --token-in USDT --token-out WETH --amount-in 1000000 --chain 42161
```

---

### positions — List your LP positions

View all your Camelot V3 concentrated liquidity positions.

**Trigger examples:**
- "Show my Camelot V3 positions"
- "What liquidity positions do I have on Camelot?"
- "Check my LP positions on Arbitrum Camelot"

**Usage:**
```
camelot-v3 positions [--owner <ADDRESS>] [--chain 42161]
```

**Parameters:**
- `--owner` — Wallet address (defaults to logged-in wallet)
- `--chain` — Chain ID (default: 42161)

**Example:**
```
camelot-v3 positions --chain 42161
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "owner": "0x87fb...",
    "positions": [
      {
        "token_id": 12345,
        "token0": "0x82aF...",
        "token1": "0xfd08...",
        "token0_symbol": "WETH",
        "token1_symbol": "USDT",
        "tick_lower": -887200,
        "tick_upper": 887200,
        "liquidity": "1000000000",
        "tokens_owed0": "0",
        "tokens_owed1": "0"
      }
    ],
    "total": 1,
    "chain_id": 42161
  }
}
```

---

### add-liquidity — Add concentrated liquidity

Add liquidity to a Camelot V3 pool. Requires user confirmation before broadcasting.

**Trigger examples:**
- "Add liquidity to Camelot V3 WETH/USDT pool"
- "Provide liquidity on Camelot with 10000 USDT"
- "Create LP position on Camelot V3"

**Usage:**
```
camelot-v3 add-liquidity --token0 <TOKEN> --token1 <TOKEN> --amount0 <RAW> --amount1 <RAW> [--tick-lower -887200] [--tick-upper 887200] [--chain 42161] [--dry-run]
```

**Parameters:**
- `--token0` — First token symbol or hex address
- `--token1` — Second token symbol or hex address
- `--amount0` — Desired amount of token0 (raw units)
- `--amount1` — Desired amount of token1 (raw units)
- `--tick-lower` — Lower price tick (default: -887200 full range)
- `--tick-upper` — Upper price tick (default: 887200 full range)
- `--amount0-min` — Minimum token0 accepted (slippage, default: 0)
- `--amount1-min` — Minimum token1 accepted (slippage, default: 0)
- `--chain` — Chain ID (default: 42161)
- `--dry-run` — Preview without broadcasting

**Write operation — ask user to confirm before executing add-liquidity.**

The binary will approve tokens and call NFPM.mint via `onchainos wallet contract-call --force`.

---

### remove-liquidity — Remove liquidity from a position

Remove liquidity from your Camelot V3 LP position. Requires user confirmation.

**Trigger examples:**
- "Remove my liquidity from Camelot V3 position 12345"
- "Exit my Camelot LP position"
- "Withdraw liquidity from Camelot V3"

**Usage:**
```
camelot-v3 remove-liquidity --token-id <ID> --liquidity <AMOUNT> [--amount0-min 0] [--amount1-min 0] [--chain 42161] [--dry-run]
```

**Parameters:**
- `--token-id` — NFT position token ID (from `positions` command)
- `--liquidity` — Amount of liquidity to remove
- `--amount0-min` — Minimum token0 to receive (slippage protection)
- `--amount1-min` — Minimum token1 to receive (slippage protection)
- `--chain` — Chain ID (default: 42161)
- `--dry-run` — Preview without broadcasting

**Write operation — ask user to confirm before executing remove-liquidity.**

The binary calls:
1. `NFPM.decreaseLiquidity` via `onchainos wallet contract-call --force`
2. `NFPM.collect` via `onchainos wallet contract-call --force`

---

## Supported Tokens (Arbitrum)

| Symbol | Address |
|--------|---------|
| WETH | 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 |
| USDT | 0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9 |
| USDC | 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 |
| ARB | 0x912CE59144191C1204E64559FE8253a0e49E6548 |
| GRAIL | 0x3d9907F9a368ad0a51Be60f7Da3b97cf940982D8 |

Pass a hex address directly for any other token.

## Contract Addresses (Arbitrum)

| Contract | Address |
|----------|---------|
| SwapRouter (V3) | 0x1F721E2E82F6676FCE4eA07A5958cF098D339e18 |
| Quoter (V3) | 0x0Fc73040b26E9bC8514fA028D998E73A254Fa76E |
| AlgebraFactory (V3) | 0x1a3c9B1d2F0529D97f2afC5136Cc23e58f1FD35B |
| NFPM (V3) | 0x00c7f3082833e796A5b3e4Bd59f6642FF44DCD15 |
