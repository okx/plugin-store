# market-structure-analyzer -- Skill Summary

## Overview
Market Structure Analyzer v3.0 is a crypto research agent that fetches, analyzes, and presents 24+ institutional-grade indicators across derivatives, options, on-chain, smart money flows, DEX activity, and macro sentiment. Powered by OKX CeFi CLI + OnchainOS CLI + direct HTTP for options chain. Includes a live auto-refreshing dashboard with K-line charts, TA overlays (RSI, MACD, Bollinger Bands), and a 12-signal composite scoring engine.

## Usage

### Live Dashboard (recommended)
```bash
python3 msa_server.py
# Opens http://localhost:8420
```
Auto-refreshing SPA with candlestick charts (TradingView Lightweight Charts v4), timeframe selector (5m/15m/1H/4H/1D), token selector (BTC/ETH/SOL + 7 more), composite signal score, and all indicator panels.

### CLI-Only Mode
```bash
python3 scripts/fetch_market_data.py BTC ETH SOL 2>/dev/null
```
Outputs JSON to stdout. Backward compatible with v2.x.

## Commands
| Command | Description |
|---|---|
| `python3 msa_server.py` | Start live dashboard on port 8420 |
| `python3 scripts/fetch_market_data.py BTC` | Fetch all indicators for BTC (JSON) |
| `python3 scripts/fetch_market_data.py BTC ETH SOL` | Multi-token fetch |
| Dune query 6988944-6988949 | Optional: ETH CEX flows, whale transfers, stablecoin flows |

## Triggers
Activates when the user mentions market structure, derivatives analysis, gamma wall, options skew, funding rates, open interest, MVRV, smart money signals, whale tracking, fear and greed, macro overview, is the market overleveraged, is BTC about to move, what does the market look like, CEX inflows/outflows, stablecoin flows, composite score, DEX hot tokens.
