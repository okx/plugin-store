
# polymarket-plugin
Trade prediction markets on Polymarket — buy and sell YES/NO outcome tokens on Polygon with automated signing and gasless proxy trading.

## Highlights
- Trade binary (YES/NO) and categorical prediction markets on Polygon
- Two trading modes: direct EOA trading or gasless proxy wallet trading
- Automated EIP-712 order signing via onchainos wallet integration
- Support for 5-minute crypto up/down markets with dedicated commands
- Real-time market data, order books, and position tracking
- Automatic API credential derivation and caching
- Built-in region restrictions and access verification
- Winning token redemption after market resolution

---SEPARATOR---

# polymarket-plugin -- Skill Summary

## Overview
This skill enables AI agents to trade prediction markets on Polymarket, a decentralized platform on Polygon where users buy and sell outcome tokens for real-world events. The plugin supports both binary (YES/NO) and categorical markets, with prices representing implied probabilities. It offers two trading modes: direct EOA trading (requires POL gas for approvals) and gasless proxy wallet trading (one-time setup, then trade without gas costs). The plugin automatically handles EIP-712 signing, API credential management, and order settlement through Polymarket's CLOB system.

## Usage
Install the plugin, connect an onchainos wallet with Polygon support, verify regional access with `check-access`, then either fund your EOA directly or set up a proxy wallet for gasless trading. Use `list-markets` to browse opportunities and `buy`/`sell` to place trades.

## Commands
| Command | Description |
|---------|-------------|
| `quickstart` | Check wallet state and get guided next-step command |
| `check-access` | Verify region is not restricted |
| `list-markets` | Browse active prediction markets with filtering |
| `list-5m` | List 5-minute crypto up/down markets |
| `get-market` | Get market details and order book |
| `get-positions` | View open positions and P&L |
| `balance` | Show POL and USDC.e balances for EOA and proxy wallets |
| `buy` | Buy YES/NO outcome shares |
| `sell` | Sell outcome shares |
| `cancel` | Cancel an open order |
| `redeem` | Redeem winning tokens after market resolves |
| `setup-proxy` | Deploy proxy wallet for gasless trading |
| `deposit` | Transfer USDC.e from EOA to proxy wallet |
| `switch-mode` | Switch default trading mode (eoa/proxy) |

## Triggers
An AI agent should activate this skill when users want to trade prediction markets, check Polymarket positions, browse political/sports/crypto betting markets, place bets on future events, or when they mention "polymarket", "prediction market", "bet on", "5-minute market", or related trading terminology.
