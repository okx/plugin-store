
# gmx-v2 -- Skill Summary

## Overview
The gmx-v2 skill enables trading perpetual contracts and managing liquidity on GMX V2 protocol. It supports opening leveraged positions (long/short), placing conditional orders with keeper execution, depositing/withdrawing from GM liquidity pools, and querying market data across Arbitrum and Avalanche networks. All write operations use a multicall pattern with execution fees and include dry-run capabilities for transaction preview.

## Usage
Install the binary via OKX plugin store, ensure your wallet is connected with `onchainos wallet login`, then use commands like `gmx-v2 --chain arbitrum open-position` to trade. Always run with `--dry-run` first to preview transactions before execution.

## Commands
| Command | Description |
|---------|-------------|
| `list-markets` | View active perpetual markets with liquidity and rates |
| `get-prices` | Get current oracle prices for tokens |
| `get-positions` | Query open leveraged positions for a wallet |
| `get-orders` | Query pending conditional orders |
| `open-position` | Open a leveraged long or short position |
| `close-position` | Close an existing position (full or partial) |
| `place-order` | Place limit, stop-loss, or take-profit orders |
| `cancel-order` | Cancel a pending conditional order |
| `deposit-liquidity` | Add tokens to GM liquidity pools |
| `withdraw-liquidity` | Remove liquidity from GM pools |
| `claim-funding-fees` | Claim accrued funding fees from positions |

## Triggers
Activate when users want to trade perpetuals with leverage, place conditional orders, manage GMX liquidity positions, or query GMX market data. Trigger phrases include "GMX trade", "open position GMX", "GMX leverage", "GMX stop loss", "deposit GM pool".
