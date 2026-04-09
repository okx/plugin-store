
# gmx-v2 -- Skill Summary

## Overview
This skill enables trading perpetual futures and managing liquidity on GMX V2, a decentralized exchange for leveraged trading. It supports opening/closing positions, placing conditional orders, providing liquidity to GM pools, and querying market data across Arbitrum and Avalanche networks. All operations use GMX's keeper-based execution model where transactions are submitted immediately but executed by keeper bots within 1-30 seconds.

## Usage
Install the plugin and connect your wallet with `onchainos wallet login`. Use `--dry-run` flag to preview any write operations before execution, then confirm with the user before submitting transactions.

## Commands
| Command | Description |
|---------|-------------|
| `list-markets` | View active perpetual markets with liquidity and rates |
| `get-prices` | Get current oracle prices for tokens |
| `get-positions` | Query open leveraged positions |
| `get-orders` | Query pending conditional orders |
| `open-position` | Open long/short leveraged position |
| `close-position` | Close existing position (full/partial) |
| `place-order` | Place limit/stop-loss/take-profit order |
| `cancel-order` | Cancel pending conditional order |
| `deposit-liquidity` | Add liquidity to GM pools |
| `withdraw-liquidity` | Remove liquidity from GM pools |
| `claim-funding-fees` | Claim accrued funding fees |

## Triggers
Activate when users want to trade leveraged positions on GMX, place stop-losses or take-profits, manage GMX liquidity pools, or query GMX market data. Key phrases include "GMX trade", "open position", "leverage", "stop loss", "GM pool", and "funding fees".
