
# polymarket-plugin
Trade prediction markets on Polymarket — buy and sell YES/NO outcome tokens for real-world events on Polygon.

## Highlights
- Trade prediction markets on current events, elections, sports, and crypto price targets
- Two trading modes: direct EOA trading or gasless proxy wallet trading
- 5-minute crypto up/down markets for short-term trading
- Real-time order book data and position tracking
- Automatic credential derivation from onchainos wallet
- Support for both binary (YES/NO) and categorical markets
- Gasless trading via POLY_PROXY mode after one-time setup
- Built-in region access verification before trading

---SEPARATOR---

# polymarket-plugin -- Skill Summary

## Overview
This skill enables AI agents to trade prediction markets on Polymarket, a decentralized prediction market platform on Polygon. Users can buy and sell outcome tokens (YES/NO or categorical) for real-world events including elections, sports, crypto prices, and breaking news. The plugin supports two trading modes: direct EOA trading (requires POL gas per transaction) or gasless proxy wallet trading (one-time setup, then trade without gas fees). All trades are settled in USDC.e on Polygon chain.

## Usage
Install the plugin, connect your onchainos wallet to Polygon, verify region access, top up with USDC.e, then start trading prediction markets with simple buy/sell commands.

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
| `cancel` | Cancel an open order |
| `redeem` | Redeem winning tokens after market resolves |
| `setup-proxy` | Deploy proxy wallet for gasless trading |
| `deposit` | Transfer USDC.e from EOA to proxy wallet |
| `switch-mode` | Switch between EOA and proxy trading modes |

## Triggers
Activate when users want to trade prediction markets, bet on outcomes, buy polymarket shares, check positions, or when they mention specific trigger phrases like "5-minute markets", "prediction trading", "bet on", or express interest in trading on current events, elections, sports outcomes, or crypto price movements.
