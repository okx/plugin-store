
# polymarket -- Skill Summary

## Overview
The Polymarket skill enables trading on prediction markets where users can buy and sell YES/NO outcome tokens for real-world events on Polygon. Markets resolve to $1.00 for winners or $0.00 for losers, with prices representing implied probabilities. The skill provides full market browsing, position tracking, and order management capabilities through Polymarket's CLOB (Central Limit Order Book) system.

## Usage
Install the plugin and connect your wallet to Polygon (chain 137). For read-only operations like browsing markets and checking positions, no additional setup is required. For trading, set up Polymarket API credentials via environment variables.

## Commands
| Command | Description |
|---------|-------------|
| `list-markets` | Browse active prediction markets with optional filtering |
| `get-market` | Get detailed market information and order book data |
| `get-positions` | View open positions with P&L calculations |
| `buy` | Purchase YES or NO outcome tokens |
| `sell` | Sell existing outcome token positions |
| `cancel` | Cancel open orders by ID, market, or all orders |

## Triggers
Activate when users want to trade prediction markets, check market prices, view their positions, or manage orders on Polymarket. Use phrases like "buy polymarket shares," "check my positions," or "list prediction markets."
