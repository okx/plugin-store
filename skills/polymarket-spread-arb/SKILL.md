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
- Places post-only maker bids through `polymarket-plugin buy`.
- Uses GTD expirations by default so resting orders auto-expire before market end.
- Cancels stale market orders through `polymarket-plugin cancel --market`.
- Stops when the market is near expiry, price is extreme, tick size changes,
  or the configured budget is reached.
- Provides setup diagnostics, opportunity reports, and optional JSONL telemetry.
- Includes a `vidarx` public-profile preset based only on public Polymarket
  Data API activity: BTC 5m, small laddered BUY entries, strict budget caps.

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
| `--profile` | Preset: `spread` or `vidarx` | `spread` |
| `--order-shares` | Shares per submitted order | `5` |
| `--order-ttl` | GTD order lifetime in seconds; `<=0` uses GTC | `120` |
| `--expiry-buffer` | Seconds before market end to stop GTD expiries | `20` |
| `--max-seconds` | Optional hard runtime cap for testing sessions | none |
| `--report` | Print an opportunity report before trading | `false` |
| `--jsonl` | Append structured telemetry to a JSONL path | none |
| `--dry-run` | Simulate decisions without real orders | `false` |
| `--strategy-id` | Strategy attribution passed to `polymarket-plugin` | `polymarket-spread-arb` |
| `--account` | Optional onchainos wallet account ID | none |

Examples:

```bash
# OKX-compliant dry run
python3 scripts/fast_arb.py run --coin btc --tf 5m --budget 25 --dry-run --report

# OKX-compliant live run through polymarket-plugin
python3 scripts/fast_arb.py run --coin eth --tf 15m --budget 50 --min-gap 1 --slots 2 --jsonl telemetry/spread-arb.jsonl

# Public-profile preset inspired by @vidarx public activity
python3 scripts/fast_arb.py run --profile vidarx --budget 25 --dry-run --report --max-seconds 20

# Inspect the public profile sample used by the preset
python3 scripts/fast_arb.py profile-report --profile-user vidarx --sample 500

# Pre-flight environment and API diagnostics
python3 scripts/fast_arb.py doctor --dry-run-only --skip-balance

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
  --expires <unix-timestamp> \
  --post-only \
  --round-up \
  --strategy-id polymarket-spread-arb
```

If a safe GTD expiry is unavailable because the market is too close to
resolution, the strategy stops instead of leaving a fresh resting order near
expiry. Passing `--order-ttl 0` falls back to GTC.

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
- Only post-only maker bids are submitted in plugin mode.
- GTD auto-expiry is used by default for resting orders.
- No market orders are used.
- Per-slot filled budget is enforced with `--budget`.
- The strategy stops near market expiry.
- The strategy stops when a side reaches an extreme price or tick-size changes.
- Existing orders for the current market are cancelled before each new pair.
- JSONL telemetry excludes credentials and raw wallet secrets.

## Output

The script prints a JSON summary:

```json
{
  "mode": "v5_single_slot",
  "execution": "plugin",
  "profile": "spread",
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
