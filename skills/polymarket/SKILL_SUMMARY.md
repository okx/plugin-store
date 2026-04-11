
# polymarket -- Skill Summary

## Overview
The Polymarket skill enables trading on prediction markets where users buy and sell outcome tokens for real-world events. Markets can be binary (YES/NO) or categorical (multiple outcomes), with each token resolving to $1.00 for winners or $0.00 for losers. Prices represent implied probabilities, operating on Polygon with USDC.e as collateral through Polymarket's Central Limit Order Book (CLOB) system.

## Usage
Install the polymarket binary and connect an onchainos wallet with USDC.e on Polygon. Use read-only commands to browse markets and positions, or trading commands to buy/sell outcome shares with automatic credential derivation.

## Commands
| Command | Description |
|---------|-------------|
| `list-markets` | Browse active prediction markets with optional keyword filtering |
| `get-market` | Get detailed market information and order book by ID or slug |
| `get-positions` | View open positions, PnL, and portfolio performance |
| `buy` | Buy outcome shares with USDC.e (limit or market orders) |
| `sell` | Sell outcome shares (limit or market orders) |
| `cancel` | Cancel specific orders, market orders, or all open orders |

## Triggers
Activate when users want to trade prediction markets, check Polymarket positions, browse event outcomes, or place bets on real-world events like elections, sports, or crypto prices. Use for any Polymarket-related trading activity on Polygon.
