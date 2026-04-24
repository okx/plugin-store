## Overview

Event-driven spread arbitrage for Polymarket crypto Up/Down markets. The
strategy watches live UP/DOWN books and places paired post-only maker bids when
the combined bid cost is below $1.00.

The default execution path is OKX challenge compliant: live orders are routed
through `polymarket-plugin buy` with `--strategy-id polymarket-spread-arb`.

Core capabilities:

- BTC, ETH, SOL, XRP, BNB, DOGE, and HYPE markets
- 5-minute, 15-minute, and 1-hour Up/Down timeframes
- Market WebSocket book monitoring
- User WebSocket fill tracking
- Corridor math for balanced UP/DOWN exposure
- Post-only GTC maker orders through `polymarket-plugin`
- Market-scoped cancel before each new pair
- Dry-run mode, budget caps, depth checks, tick-size stop, expiry stop

Tags: `polymarket` `arbitrage` `spread` `market-making` `crypto` `polygon` `websocket` `strategy-id`

## Prerequisites

- `polymarket-plugin` installed and configured
- `onchainos` CLI installed and logged in
- Python 3.8+ with `websockets`
- Polymarket proxy/wallet configured with USDC.e on Polygon
- Accessible region for Polymarket

## Quick Start

1. Install dependencies: `pip3 install websockets`
2. Check setup: `polymarket-plugin balance`
3. Dry run: `python3 scripts/fast_arb.py run --coin btc --tf 5m --budget 25 --dry-run`
4. Live run: `python3 scripts/fast_arb.py run --coin eth --tf 15m --budget 50 --min-gap 1 --slots 2`
5. Confirm output includes `"execution": "plugin"` and `"strategy_id": "polymarket-spread-arb"`
