# polymarket-plugin

## Overview

Polymarket is a decentralized prediction market on Polygon where users trade YES/NO outcome tokens on real-world events.

Core operations:
- Buy and sell outcome tokens on any Polymarket market (sports, politics, crypto, daily series)
- Trade recurring crypto price series (BTC/ETH/SOL/XRP — 5m, 15m, 4h intervals)
- Manage orders: place resting GTC/GTD/POST_ONLY limit orders or market (FOK) orders
- Check positions and redeem winning tokens after market resolution
- Deploy a proxy wallet for gasless relayer-paid trading

Tags: `defi` `polygon` `prediction-market` `clob` `trading`

## Prerequisites

- No IP restrictions (geo-blocked regions can still set up a proxy wallet; trading commands will surface a warning)
- Supported chain: Polygon (137)
- Supported collateral: USDC.e (V1, pre-2026-04-28) / pUSD (V2, post-2026-04-28) — auto-wrapped on buy
- onchainos CLI installed and authenticated with a Polygon wallet
- At least $1 USDC.e on Polygon for a test trade; ~0.01 POL for gas (EOA mode) or ~$0.01 POL one-time for proxy setup

## Quick Start

1. **Check balances**: run `polymarket balance` — confirms your wallet address, USDC.e / pUSD / POL balances, and the current CLOB version (V1 or V2)
2. **Find a market**: run `polymarket get-series --series btc-5m` for the current BTC 5-minute slot, or `polymarket list-markets --limit 5` for active markets
3. **Place a trade**: run `polymarket buy --market-id btc-5m --outcome Up --amount 5 --dry-run` to preview, then remove `--dry-run` to execute
4. **Check your positions**: run `polymarket positions` to see open holdings and unrealised P&L
