
# polymarket -- Skill Summary

## Overview
The Polymarket skill enables AI agents to interact with prediction markets on the Polygon blockchain, allowing users to trade YES/NO outcome tokens on real-world events. Users can browse markets, check positions, and execute trades using USDC.e as collateral. Each market resolves to either $1.00 (winning outcome) or $0.00 (losing outcome) per share, with prices representing implied probabilities of events occurring.

## Usage
Install the plugin and connect your wallet to Polygon (chain 137). For read-only operations like browsing markets and checking positions, no additional setup is required. For trading operations, you'll need to set Polymarket API credentials via environment variables.

## Commands
| Command | Description |
|---------|-------------|
| `list-markets` | Browse active prediction markets with optional filtering |
| `get-market` | Get detailed market information and order book data |
| `get-positions` | View open positions and P&L for a wallet address |
| `buy` | Purchase YES or NO shares with USDC.e |
| `sell` | Sell existing YES or NO shares |
| `cancel` | Cancel open orders by ID, market, or all orders |

## Triggers
Activate this skill when users want to trade prediction markets, check Polymarket positions, browse prediction markets, or ask about buying/selling YES/NO tokens on specific events. Also trigger for queries about Polymarket prices, market odds, or prediction market trading in general.
