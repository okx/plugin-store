---
name: hyperpulse-grid
description: "NLP-driven automated grid trading bot for Hyperliquid perps. High transaction count and volume optimization."
license: MIT
metadata:
  author: Lino Alemz
  version: "1.0.0"
  strategyId: "HP_GRID_2026_S1"
---

# HyperPulse Grid: Automated Perpetual Grid Trading

Advanced strategy skill for Hyperliquid perps. This skill enables AI agents to deploy and manage grid trading bots using natural language and **Pilot Mode** automation.

## 🟢 Pilot Mode (Autonomous Trading)
Users can authorize **Pilot Mode** to allow the bot to trade without manual signature for each order.
- **Trigger**: "Enable Pilot Mode for HyperPulse" or "Start automated trading"
- **Requirements**: User must set a **Spending Cap** and **Session Duration** in the OKX Agentic Wallet.
- **Visuals**: The dashboard will show a **Green Toggle** when active.

## Intent Triggers
"start a grid bot", "grid trade HYPE", "automate my Hyperliquid trading", "set up a range bot", "check my grid pnl", "open HyperPulse dashboard"

## UI Dashboard
The skill provides a premium dashboard (Mockup attached in plugin files):
- **Theme**: Dark Mode (OKX-native style).
- **Color**: Neon Green (#b6ff00) accents.
- **Features**: Live Grid Chart, PnL Gauge, and Transaction Feed.

## Command Reference

### 1. `onchainos grid start`
**Parameters**:
- `--symbol`: (Required) e.g., `HYPE-USDC`
- `--amount`: (Required) Total USDC allocation
- `--grids`: (Default: 10) Number of levels
- `--upper`: (Required) Price ceiling
- `--lower`: (Required) Price floor
- `--pilot`: (Flag) Enable automated execution

### 2. `onchainos grid status`
Returns: Bot ID, Symbol, Active Grids, Current PnL, Total Volume Generated.

### 3. `onchainos grid stop`
**Parameters**:
- `--id`: Bot ID to terminate.
- `--close-positions`: (Boolean) Whether to market close all open perp positions.

## Risk Controls
- **Liquidation Guard**: Automatically stops the bot if the position's liquidation price is within 5% of market price.
- **Strategy Tracking**: All transactions MUST include the header `X-Strategy-ID: HP_GRID_2026_S1`.

---
*Created for OKX Plugin Store Season 1 Developer Challenge.*
