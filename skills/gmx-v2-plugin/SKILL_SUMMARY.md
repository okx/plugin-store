
# gmx-v2-plugin -- Skill Summary

## Overview
This plugin enables trading perpetual futures and managing liquidity on GMX V2 decentralized exchange. It provides comprehensive functionality for opening/closing leveraged positions, placing conditional orders, depositing/withdrawing from GM liquidity pools, and monitoring market data across Arbitrum and Avalanche networks. All write operations use a keeper model where transactions are executed 1-30 seconds after submission.

## Usage
Install via OKX plugin store, connect wallet with `onchainos wallet login`, then use commands like `gmx-v2 open-position` for trading or `gmx-v2 list-markets` for market data. Always run with `--dry-run` first to preview operations before confirming with `--confirm`.

## Commands
| Command | Description |
|---------|-------------|
| `list-markets` | View active perpetual markets with liquidity and rates |
| `get-prices` | Get current oracle prices for tokens |
| `get-positions` | Query open positions for a wallet |
| `get-orders` | Query pending conditional orders |
| `open-position` | Open leveraged long/short position |
| `close-position` | Close position fully or partially |
| `place-order` | Create limit/stop-loss/take-profit orders |
| `cancel-order` | Cancel pending order by key |
| `deposit-liquidity` | Add tokens to GM liquidity pools |
| `withdraw-liquidity` | Remove liquidity from GM pools |
| `claim-funding-fees` | Claim accrued funding fee rewards |

## Triggers
Activate when users want to trade perpetuals with leverage, set stop-losses or take-profits, manage GMX liquidity positions, or need real-time GMX market data and position monitoring.
