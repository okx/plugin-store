
# polymarket-plugin

Trade prediction markets on Polymarket — buy and sell YES/NO outcome tokens with automatic signing and order management on Polygon.

## Highlights
- Trade binary (YES/NO) and categorical prediction markets on Polygon
- Two trading modes: direct EOA trading or gasless proxy wallet
- Browse markets by category (sports, elections, crypto) or breaking news
- 5-minute crypto up/down markets with real-time price feeds
- Automatic order signing via onchainos wallet integration
- Check positions, manage orders, and redeem winning tokens
- No Polymarket account signup required - wallet-based authentication
- Region restriction checking before trading

---SEPARATOR---

# polymarket-plugin -- Skill Summary

## Overview
This skill enables trading on Polymarket prediction markets through CLI commands. It supports both read-only operations (browsing markets, checking positions) and authenticated trading operations (buying/selling outcome tokens, order management). The plugin integrates with onchainos wallet for transaction signing and offers two trading modes: direct EOA trading with per-transaction gas costs, or gasless proxy wallet trading after one-time setup.

## Usage
Connect an onchainos wallet to Polygon, verify regional access with `check-access`, then browse markets with `list-markets` or trade directly with `buy`/`sell` commands. For gasless trading, run `setup-proxy` once and fund via `deposit`.

## Commands
| Command | Description |
|---------|-------------|
| `check-access` | Verify region is not restricted |
| `list-markets` | Browse active prediction markets with filtering |
| `list-5m` | List 5-minute crypto up/down markets |
| `get-market` | Get market details and order book |
| `get-positions` | View open positions |
| `balance` | Show POL and USDC.e balances |
| `buy` | Buy YES/NO outcome shares |
| `sell` | Sell outcome shares |
| `cancel` | Cancel open orders |
| `redeem` | Redeem winning tokens |
| `setup-proxy` | Deploy proxy wallet for gasless trading |
| `deposit` | Transfer USDC.e to proxy wallet |
| `switch-mode` | Switch between EOA and proxy trading modes |

## Triggers
Activate when users want to trade prediction markets, check Polymarket positions, browse political/sports/crypto betting markets, or search for specific 5-minute crypto up/down trading opportunities. Also trigger on onboarding phrases like "just installed polymarket" or "how do I use polymarket".
