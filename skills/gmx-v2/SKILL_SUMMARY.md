
# gmx-v2 -- Skill Summary

## Overview
This skill enables trading on GMX V2, a decentralized perpetual exchange supporting leveraged positions and liquidity provision. Users can open/close positions with up to 100x leverage, place conditional orders, manage GM pool liquidity, and claim funding fees across Arbitrum and Avalanche networks. All operations use GMX's keeper system for delayed execution within 1-30 seconds.

## Usage
Connect your wallet via `onchainos wallet login`, then use commands like `gmx-v2 open-position` for trading or `gmx-v2 list-markets` for market data. Always run `--dry-run` first and confirm details before executing write operations.

## Commands
| Command | Description |
|---------|-------------|
| `list-markets` | View active perpetual markets with liquidity data |
| `get-prices` | Get current oracle prices for tokens |
| `get-positions` | Query open leveraged positions |
| `get-orders` | Query pending conditional orders |
| `open-position` | Open long/short leveraged position |
| `close-position` | Close existing position (full/partial) |
| `place-order` | Create limit/stop-loss/take-profit orders |
| `cancel-order` | Cancel pending conditional order |
| `deposit-liquidity` | Add tokens to GM pools |
| `withdraw-liquidity` | Remove tokens from GM pools |
| `claim-funding-fees` | Claim accrued funding fee rewards |

## Triggers
Activate when users want to trade perpetuals with leverage, manage GMX positions, place stop-losses or take-profits, or provide liquidity to GM pools on Arbitrum/Avalanche networks.
