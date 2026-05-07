## Overview

Pendle is a yield tokenization protocol that splits yield-bearing assets into Principal Tokens (PT, fixed yield) and Yield Tokens (YT, floating yield). This skill lets you browse markets, buy/sell PT for fixed yield, buy/sell YT for floating yield, mint/redeem PT+YT pairs, and add/remove liquidity on Ethereum, Arbitrum, BSC, and Base.

## Prerequisites
- onchainos CLI installed and logged in
- ETH (or BNB on BSC) for gas on the target chain
- A stablecoin (e.g. USDC) or yield-bearing asset (e.g. weETH, wstETH) on the target chain to trade

## Quick Start
1. Check your current state and get a guided next step: `pendle-plugin quickstart`
2. If you see `status: no_funds` / `needs_gas` / `needs_funds` — fund the wallet address shown in the output (ETH for gas + USDC to trade)
3. Browse active markets — note `pt` address and `address` (= LP address); look for high `impliedApy` and `liquidity.usd > $1M`: `pendle-plugin --chain 42161 list-markets --active-only --limit 10`
4. Search markets by asset (e.g. ETH-derivatives, stablecoins): `pendle-plugin --chain 42161 list-markets --search weETH --active-only`
5. Buy PT for fixed yield — preview first (no `--confirm`): `pendle-plugin --chain 42161 buy-pt --token-in <USDC_ADDR> --amount-in 5000000 --pt-address <PT_ADDR>`
6. Re-run with `--confirm` to execute: `pendle-plugin --chain 42161 --confirm buy-pt --token-in <USDC_ADDR> --amount-in 5000000 --pt-address <PT_ADDR>`
7. Check your positions (allow 15–30s for the Pendle indexer): `pendle-plugin --chain 42161 get-positions`
8. For leveraged floating yield, buy YT instead of PT: `pendle-plugin --chain 42161 --confirm buy-yt --token-in <USDC_ADDR> --amount-in 5000000 --yt-address <YT_ADDR>`
9. Exit before expiry: `pendle-plugin --chain 42161 --confirm sell-pt --pt-address <PT_ADDR> --amount-in <PT_WEI> --token-out <USDC_ADDR>`
