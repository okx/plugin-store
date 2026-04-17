
# gmx-v2-plugin -- Skill Summary

## Overview
This plugin enables AI agents to interact with GMX V2 decentralized perpetual trading protocol on Arbitrum and Avalanche. It provides comprehensive trading capabilities including opening/closing leveraged positions, placing conditional orders, managing liquidity in GM pools, and monitoring market data. The plugin uses a keeper-based execution model where orders are created on-chain and executed by automated keepers within 1-30 seconds.

## Usage
Install the plugin via OKX plugin store, ensure your wallet is connected with `onchainos wallet login`, then start with `gmx-v2 quickstart` to check your assets and get guided next steps. All write operations require explicit user confirmation via the `--confirm` flag after previewing with `--dry-run`.

## Commands
| Command | Description |
|---------|-------------|
| `quickstart` | Check wallet assets and get guided recommendations |
| `list-markets` | View all active perpetual markets with liquidity data |
| `get-prices` | Get current oracle prices for tokens |
| `get-positions` | Query open perpetual positions |
| `get-orders` | Query pending conditional orders |
| `open-position` | Open leveraged long/short positions |
| `close-position` | Close existing positions (full or partial) |
| `place-order` | Place limit/stop-loss/take-profit orders |
| `cancel-order` | Cancel pending orders |
| `deposit-liquidity` | Add liquidity to GM pools |
| `withdraw-liquidity` | Remove liquidity from GM pools |
| `claim-funding-fees` | Claim accrued funding fees |

## Triggers
Activate this skill when users mention GMX trading, perpetual positions, leverage trading, stop-loss/take-profit orders, or liquidity provision on Arbitrum/Avalanche networks. Also trigger for phrases like "open position GMX", "GMX trade", "GMX leverage", or "deposit GM pool".
