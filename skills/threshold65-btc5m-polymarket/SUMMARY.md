# threshold65-btc5m-polymarket

## Overview

Threshold65 BTC5M Polymarket is a strategy plugin adapted from a live-tested BTC 5-minute Polymarket system. It watches the current BTC Up/Down market, applies a high-confidence threshold and entry band, and routes all execution through the official `polymarket-plugin` with `--strategy-id threshold65-btc5m-polymarket` for attribution.

Core operations:

- Resolve the current BTC 5-minute Up/Down market
- Enter only when one side reaches the configured threshold and entry band
- Enforce one entry per 5-minute window with small per-window sizing
- Exit through profit protection, final forced exit, or settlement redeem

Tags: `polymarket` `btc` `five-minute` `strategy` `risk-control`

## Prerequisites

- Polymarket access is region restricted; run `polymarket-plugin check-access` before any trading recommendation
- Supported venue: Polymarket BTC 5-minute Up/Down markets through `polymarket-plugin`
- Required tools: `polymarket-plugin` installed and an authenticated onchainos wallet when trading
- The user must approve the per-window budget and live-loop behavior
- Every `buy` and `sell` command must include `--strategy-id threshold65-btc5m-polymarket`

## Quick Start

1. **Check access and wallet readiness**: Run `polymarket-plugin check-access`, then `polymarket-plugin quickstart` or `polymarket-plugin balance`.
2. **Resolve the current BTC 5m slot**: Run `polymarket-plugin get-series --series btc-5m`.
3. **Apply entry filters**: Enter only when one side reaches the threshold, remains inside the live entry band, and the current 5-minute window has no prior entry.
4. **Preview and execute entry**: Run `polymarket-plugin buy ... --dry-run --strategy-id threshold65-btc5m-polymarket`, then execute only after authorization.
5. **Manage exit**: Use `polymarket-plugin sell ... --strategy-id threshold65-btc5m-polymarket` for profit-window or final exits; use `polymarket-plugin redeem ... --strategy-id threshold65-btc5m-polymarket` only after official settlement if needed.
