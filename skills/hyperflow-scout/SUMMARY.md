# hyperflow-scout

## Overview

HyperFlow Scout is a strategy plugin that scouts one lightweight Hyperliquid micro-trend trade and routes execution through the official `hyperliquid-plugin` with `--strategy-id hyperflow-scout` for attribution.

Core operations:

- Check account readiness and current positions
- Read current Hyperliquid market prices
- Recommend one simple action: buy, sell, close, or wait
- Execute only through `hyperliquid-plugin` with strategy attribution

Tags: `hyperliquid` `perpetuals` `strategy` `momentum` `execution`

## Prerequisites

- Supported venue: Hyperliquid perpetuals through `hyperliquid-plugin`
- Required tools: `hyperliquid-plugin` installed and an authenticated account when trading
- The user must provide pair, risk level, and maximum position size
- Perpetual futures are high risk; use light sizing by default
- Every trading command must include `--strategy-id hyperflow-scout`

## Quick Start

1. **Check readiness**: Run `hyperliquid quickstart` to confirm the account can trade.
2. **Check existing exposure**: Run `hyperliquid positions --show-orders` before adding risk.
3. **Read prices**: Run `hyperliquid prices --coin <COIN>` for the selected market.
4. **Preview the trade**: Run `hyperliquid order ... --strategy-id hyperflow-scout` without `--confirm`.
5. **Execute only after confirmation**: Add `--confirm` to the final `hyperliquid order` or `hyperliquid close` command only after explicit user confirmation.
