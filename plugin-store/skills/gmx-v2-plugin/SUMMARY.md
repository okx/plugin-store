## Overview

GMX V2 is a decentralized perpetuals and spot exchange with leveraged positions and GM pool liquidity on Arbitrum and Avalanche. This skill lets you open/close long/short positions, place limit / stop-loss / take-profit orders, add/remove GM pool liquidity, check positions/orders/prices, and claim funding fees.

## Prerequisites
- onchainos CLI installed and logged in
- ETH for execution fees on Arbitrum (chain 42161, default) or AVAX on Avalanche (43114)
- USDC (≥ $10 recommended) on the target chain as collateral

## Quick Start
1. Check your state and get a guided next step: `gmx-v2-plugin quickstart` (Arbitrum default; use `gmx-v2-plugin --chain avalanche quickstart` for Avalanche)
2. If you see `status: no_funds` / `needs_fee` / `needs_collateral` — fund the wallet address shown in the output (ETH/AVAX for fees + USDC as collateral)
3. Browse active markets with liquidity and rates: `gmx-v2-plugin --chain arbitrum list-markets`
4. Get current oracle prices: `gmx-v2-plugin --chain arbitrum get-prices --token ETH`
5. If `status: ready` — open a leveraged long (preview first without `--confirm`, then re-run with it): `gmx-v2-plugin --chain arbitrum open-position --market ETH/USD --collateral-token <USDC_ADDR> --collateral-amount 10000000 --size-usd 50 --long --confirm`
6. If `status: active` — review open positions and pending orders: `gmx-v2-plugin --chain arbitrum get-positions` / `gmx-v2-plugin --chain arbitrum get-orders`
7. Attach a stop-loss or take-profit (use `stop-loss` or `limit-decrease` as `--order-type`): `gmx-v2-plugin --chain arbitrum place-order --order-type stop-loss --market-token <MKT_ADDR> --collateral-token <USDC_ADDR> --size-usd 50 --collateral-amount 10000000 --trigger-price-usd 3000 --acceptable-price-usd 2990 --long --confirm`
8. Close a position: `gmx-v2-plugin --chain arbitrum close-position --market-token <MKT_ADDR> --collateral-token <USDC_ADDR> --size-usd 50 --collateral-amount 10000000 --long --confirm`
