
# polymarket -- Skill Summary

## Overview
The Polymarket skill enables trading on prediction markets where users buy and sell YES/NO outcome tokens for real-world events on Polygon. Each market resolves to $1.00 for winners or $0.00 for losers, with prices representing implied probabilities. The skill provides read-only market browsing and position checking without authentication, plus full trading capabilities including buying, selling, and order management when API credentials are configured.

## Usage
Install the plugin and connect your wallet to Polygon (chain 137). For trading functionality, set up Polymarket API credentials via environment variables. Use commands to browse markets, check positions, and execute trades.

## Commands
| Command | Description |
|---------|-------------|
| `list-markets` | Browse active prediction markets with optional keyword filtering |
| `get-market` | Get detailed market information and order book data |
| `get-positions` | View current positions and PnL for a wallet address |
| `buy` | Buy YES or NO shares with specified amount and optional limit price |
| `sell` | Sell existing shares with specified quantity and optional limit price |
| `cancel` | Cancel orders by ID, market, or all open orders |

## Triggers
Activate when users want to trade prediction markets, check Polymarket positions, browse available markets, or manage existing orders on the Polymarket platform. Use for any prediction market trading activity on Polygon.
