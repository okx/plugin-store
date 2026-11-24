## Overview

PancakeSwap V3 is a concentrated liquidity DEX. This skill lets you get swap quotes, swap tokens via SmartRouter, browse pools across fee tiers, and manage concentrated liquidity positions (add, view, remove) on BNB Chain, Base, and Arbitrum.

## Prerequisites
- onchainos CLI installed and logged in
- Gas token on the target chain: BNB on BSC (chain 56, default), ETH on Base (8453) or Arbitrum (42161)
- Tokens to swap or provide as liquidity (e.g. WBNB / USDT / USDC / WETH)

## Quick Start
1. Check your BNB Chain state and get a guided next step: `pancakeswap-v3-plugin quickstart`
2. If you see `status: no_funds` / `needs_gas` / `needs_funds` — fund the wallet address shown in the output (BNB for gas + USDT/USDC to trade)
3. Get a swap quote (read-only, no gas): `pancakeswap-v3-plugin quote --from WBNB --to USDT --amount 0.1 --chain 56`
4. Execute a swap (preview first without `--confirm`, then re-run with it): `pancakeswap-v3-plugin swap --from WBNB --to USDT --amount 0.1 --chain 56 --confirm`
5. Browse pools for a pair across all fee tiers: `pancakeswap-v3-plugin pools --token0 WBNB --token1 USDT --chain 56`
6. Provide concentrated liquidity (auto ±10% range if ticks omitted): `pancakeswap-v3-plugin add-liquidity --token-a WBNB --token-b USDT --fee 500 --amount-a 0.1 --amount-b 60 --chain 56`
7. If `status: active` — review your LP positions: `pancakeswap-v3-plugin positions --owner <YOUR_ADDR> --chain 56`
8. Remove liquidity and collect accrued fees: `pancakeswap-v3-plugin remove-liquidity --token-id <TOKEN_ID> --chain 56`
