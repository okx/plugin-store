
# gmx-v2 -- Skill Summary

## Overview
This skill enables trading perpetual futures and managing liquidity on GMX V2 protocol across Arbitrum and Avalanche networks. It provides comprehensive position management including opening/closing leveraged trades, placing conditional orders with trigger prices, depositing/withdrawing from GM liquidity pools, and monitoring market data. The skill implements safety checks for token balances, collateral requirements, and execution fees before broadcasting transactions.

## Usage
Install the binary via plugin store, connect your wallet with `onchainos wallet login`, then use commands like `gmx-v2 --chain arbitrum open-position` to trade or `gmx-v2 list-markets` to explore available markets. All write operations require `--confirm` flag after user approval.

## Commands
- `list-markets` - View active perpetual markets with liquidity and rates
- `get-prices` - Fetch current oracle prices for tokens
- `get-positions` - Query open positions for wallet address
- `get-orders` - Query pending conditional orders
- `open-position` - Open leveraged long/short position
- `close-position` - Close position fully or partially
- `place-order` - Create limit/stop-loss/take-profit orders
- `cancel-order` - Cancel pending order by key
- `deposit-liquidity` - Add tokens to GM pools
- `withdraw-liquidity` - Remove liquidity from GM pools
- `claim-funding-fees` - Claim accrued funding fee income

## Triggers
Activate when users want to trade perpetuals with leverage, manage positions on GMX V2, place stop-loss or take-profit orders, or provide liquidity to earn fees. Use for phrases like "open GMX position," "GMX leverage," "GMX stop loss," or "deposit GM pool."
