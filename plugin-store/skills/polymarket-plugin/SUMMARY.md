## Overview

Polymarket is a prediction market protocol on Polygon where users trade YES/NO outcome shares of real-world events. This skill lets you browse markets (including 5-minute crypto up/down markets), buy and sell outcome shares, check positions, cancel orders, redeem winning tokens, and optionally set up a proxy wallet for gasless trading.

## Prerequisites
- onchainos CLI installed and logged in with a Polygon address (chain 137)
- USDC.e on Polygon for trading (≥ $5 recommended for a first test trade)
- Recommended: run `setup-proxy` once for gasless trading (Polymarket's relayer pays gas). Fallback EOA mode needs POL on Polygon for every buy/sell approval
- Accessible region — Polymarket blocks the US and OFAC-sanctioned jurisdictions

## Quick Start
1. Check your current state and get a guided next step: `polymarket-plugin quickstart`
2. If you see `status: restricted` — switch to an accessible region and re-run `polymarket-plugin quickstart`
3. If you see `status: no_funds` / `low_balance` — send ≥ $5 USDC.e to your EOA wallet on Polygon (chain 137); view the address with `polymarket-plugin balance`
4. If you see `status: needs_setup` — create the Polymarket proxy wallet (one-time POL gas) for gasless trading: `polymarket-plugin setup-proxy`
5. If you see `status: needs_deposit` — deposit EOA USDC.e into your proxy wallet: `polymarket-plugin deposit --amount 50`
6. If you see `status: proxy_ready` — browse markets and place your first gasless order: `polymarket-plugin list-markets` → `polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5`
7. If you see `status: active` — review open positions and P&L: `polymarket-plugin get-positions`
8. Exit a position, or redeem winnings when the market resolves: `polymarket-plugin sell --market-id <SLUG> --outcome yes --amount 5` / `polymarket-plugin redeem --market-id <SLUG>`
