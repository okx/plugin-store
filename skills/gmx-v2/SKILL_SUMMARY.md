
# gmx-v2 -- Skill Summary

## Overview
This skill enables trading leveraged perpetual positions and managing liquidity on GMX V2 protocol across Arbitrum and Avalanche networks. It provides comprehensive functionality for opening/closing positions, placing conditional orders, depositing/withdrawing from GM pools, and querying market data with built-in safety features including dry-run previews and automatic fee management.

## Usage
Install the plugin via OKX plugin store, ensure your onchainos wallet is connected, then use commands like `gmx-v2 --chain arbitrum open-position` to trade. Always run commands with `--dry-run` first to preview transactions before execution.

## Commands
- `list-markets` - View active perpetual markets with liquidity and rates
- `get-prices` - Get current oracle prices for tokens
- `get-positions` - Query open positions for a wallet
- `get-orders` - Query pending conditional orders
- `open-position` - Open leveraged long/short positions
- `close-position` - Close existing positions (full or partial)
- `place-order` - Place limit/stop-loss/take-profit orders
- `cancel-order` - Cancel pending conditional orders
- `deposit-liquidity` - Add liquidity to GM pools
- `withdraw-liquidity` - Remove liquidity from GM pools
- `claim-funding-fees` - Claim accrued funding fee rewards

## Triggers
Activate when users want to trade leveraged perpetuals, manage GMX positions, or interact with GM liquidity pools on Arbitrum/Avalanche. Common trigger phrases include "open position GMX", "GMX trade", "GMX leverage", "GMX liquidity", or "GMX stop loss".
