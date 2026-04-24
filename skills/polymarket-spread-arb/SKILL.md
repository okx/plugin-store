---
name: polymarket-spread-arb
description: "Event-driven spread arbitrage on Polymarket crypto Up/Down markets using polymarket-plugin execution with OKX strategy attribution"
version: "5.0.0"
author: "checkra1neth"
tags:
  - polymarket
  - prediction-market
  - arbitrage
  - spread
  - market-making
  - polygon
  - websocket
  - strategy-id
---

# Polymarket Spread Arbitrage v5

## Overview

This strategy monitors short-term Polymarket crypto Up/Down markets and places
paired maker limit bids when `UP bid + DOWN bid < 1.00`. If both sides fill at
a combined cost below $1.00, the position can settle profitably regardless of
which outcome wins.

The default execution path is compliant with the OKX OnchainOS Plugins
Challenge: all write operations go through `polymarket-plugin` and include
`--strategy-id polymarket-spread-arb`.

## What It Does

- Supports BTC, ETH, SOL, XRP, BNB, DOGE, and HYPE.
- Supports 5-minute, 15-minute, and 1-hour crypto Up/Down markets.
- Reads live order books from Polymarket market WebSocket.
- Tracks fills from Polymarket user WebSocket.
- Places post-only GTC maker bids through `polymarket-plugin buy`.
- Cancels stale market orders through `polymarket-plugin cancel --market`.
- Stops when the market is near expiry, price is extreme, tick size changes,
  or the configured budget is reached.

## Prerequisites

1. `polymarket-plugin` installed and configured:
   ```bash
   polymarket-plugin --version
   polymarket-plugin check-access
   ```

2. `onchainos` CLI installed and logged in:
   ```bash
   onchainos --version
   onchainos wallet login
   ```

3. Python dependencies:
   ```bash
   pip3 install websockets
   ```

4. A configured Polymarket wallet/proxy with USDC.e on Polygon:
   ```bash
   polymarket-plugin balance
   ```

## Main Command

```bash
python3 scripts/fast_arb.py run --coin <coin> --tf <timeframe> --budget <usd-per-slot> [--dry-run]
```

Important flags:

| Flag | Description | Default |
|------|-------------|---------|
| `--coin` | `btc`, `eth`, `sol`, `xrp`, `bnb`, `doge`, or `hype` | required |
| `--tf` | `5m`, `15m`, or `1h` | required |
| `--budget` | Max filled USD per market slot | `50` |
| `--min-gap` | Minimum spread gap in cents before bidding | `1` |
| `--min-depth` | Minimum best-bid depth in shares | `5` |
| `--slots` | Number of consecutive slots to monitor | `1` |
| `--dry-run` | Simulate decisions without real orders | `false` |
| `--strategy-id` | Strategy attribution passed to `polymarket-plugin` | `polymarket-spread-arb` |
| `--account` | Optional onchainos wallet account ID | none |

Examples:

```bash
# OKX-compliant dry run
python3 scripts/fast_arb.py run --coin btc --tf 5m --budget 25 --dry-run

# OKX-compliant live run through polymarket-plugin
python3 scripts/fast_arb.py run --coin eth --tf 15m --budget 50 --min-gap 1 --slots 2

# Optional account switch before running
python3 scripts/fast_arb.py run --coin sol --tf 1h --budget 30 --account <wallet-account-id>
```

## Execution Path

This is the path intended for the OKX challenge. The strategy reads market
data from Polymarket APIs/WebSockets, but all trading actions are
delegated to the dependent plugin:

```bash
polymarket-plugin buy \
  --token-id <token-id> \
  --outcome yes|no \
  --amount <usdc> \
  --price <limit-price> \
  --order-type GTC \
  --post-only \
  --round-up \
  --strategy-id polymarket-spread-arb
```

Market cleanup uses:

```bash
polymarket-plugin cancel --market <condition-id>
```

## Strategy Logic

1. Resolve current/upcoming crypto Up/Down market slugs.
2. Subscribe to live books for UP and DOWN tokens.
3. Wait until both sides have bids and asks.
4. Check gross pair cost: `UP best bid + DOWN best bid < 1.00`.
5. Check depth and rate limits.
6. Use corridor math to avoid unbalanced exposure.
7. Cancel stale orders for the market.
8. Submit one or two post-only maker limit bids through `polymarket-plugin`.
9. Track real fills from the user WebSocket.

## Risk Controls

- Dry-run mode is available and should be used first.
- Only post-only GTC maker bids are submitted in plugin mode.
- No market orders are used.
- Per-slot filled budget is enforced with `--budget`.
- The strategy stops near market expiry.
- The strategy stops when a side reaches an extreme price or tick-size changes.
- Existing orders for the current market are cancelled before each new pair.

## Output

The script prints a JSON summary:

```json
{
  "mode": "v5_single_slot",
  "execution": "plugin",
  "strategy_id": "polymarket-spread-arb",
  "coin": "BTC",
  "tf": "5m",
  "dry_run": true,
  "markets": []
}
```

## Notes

- Polymarket may restrict access in some jurisdictions.
- This is an advanced trading strategy and can lose money through partial fills,
  stale books, adverse selection, latency, API failures, or market resolution
  changes.
- The strategy does not guarantee profit; it only searches for bounded spread
  opportunities and applies position controls.
