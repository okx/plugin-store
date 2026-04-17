
# gmx-v2-plugin -- Skill Summary

## Overview
This plugin enables AI agents to interact with GMX V2 decentralized perpetual exchange, allowing users to trade leveraged positions, manage conditional orders, and provide liquidity to GM pools on Arbitrum and Avalanche. It provides comprehensive trading functionality with built-in safety checks, dry-run capabilities, and real-time market data queries.

## Usage
Start by running `gmx-v2 quickstart` to check your wallet status and get personalized recommendations. All write operations require user confirmation and support `--dry-run` for previewing transactions before execution.

## Commands
- `quickstart` - Check wallet assets and get guided next steps
- `list-markets` - View active perpetual markets with liquidity data
- `get-prices` - Fetch current oracle prices for tokens
- `get-positions` - Query open leveraged positions
- `get-orders` - List pending conditional orders
- `open-position` - Open leveraged long/short positions
- `close-position` - Close positions (full or partial)
- `place-order` - Create limit/stop-loss/take-profit orders
- `cancel-order` - Cancel pending orders by key
- `deposit-liquidity` - Add tokens to GM pools
- `withdraw-liquidity` - Remove liquidity from GM pools
- `claim-funding-fees` - Claim accrued funding fees

## Triggers
Activate when users want to trade perpetuals with leverage, manage GMX positions, place conditional orders, or interact with GMX liquidity pools. Key phrases include "GMX trade", "leverage position", "stop loss", "take profit", "GM pool", and "funding fees".
