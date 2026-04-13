
# polymarket
Trade prediction markets on Polymarket — buy and sell YES/NO outcome tokens on Polygon.

## Highlights
- Trade YES/NO and categorical prediction market tokens
- Direct integration with Polymarket CLOB on Polygon
- Auto-derived API credentials from onchainos wallet
- EIP-712 signed orders with gasless execution
- Real-time market data and order book access
- Position tracking and PnL monitoring
- USDC.e collateral with automatic approvals
- Support for limit orders, market orders, and maker rebates

---SEPARATOR---

# polymarket -- Skill Summary

## Overview
This skill enables AI agents to trade prediction markets on Polymarket, a decentralized platform where users buy and sell outcome tokens representing real-world events. Markets can be binary (YES/NO) or categorical (multiple outcomes), with tokens resolving to $1 for winners and $0 for losers. The skill handles order placement, position management, and market discovery through Polymarket's CLOB API on Polygon, using USDC.e as collateral and EIP-712 signatures for gasless trading.

## Usage
Connect an onchainos wallet with USDC.e on Polygon, verify regional access with `polymarket check-access`, then browse markets with `list-markets` and place trades with `buy` or `sell` commands. The plugin automatically derives API credentials and handles on-chain approvals.

## Commands
| Command | Description |
|---------|-------------|
| `check-access` | Verify region is not restricted for trading |
| `list-markets [--limit N] [--keyword text]` | Browse active prediction markets |
| `get-market --market-id <id>` | Get market details and order book |
| `get-positions [--address addr]` | View open positions and PnL |
| `buy --market-id <id> --outcome <outcome> --amount <usdc> [options]` | Buy outcome shares with USDC.e |
| `sell --market-id <id> --outcome <outcome> --shares <amount> [options]` | Sell outcome shares for USDC.e |

## Triggers
Activate this skill when users want to trade prediction markets, place bets on events, buy YES/NO tokens, check Polymarket positions, or explore prediction market opportunities. Also trigger for setup requests like "new to polymarket" or "get started with polymarket".
