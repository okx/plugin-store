# kairos-floor-strategy

Autonomous AI trading strategy for Hyperliquid perpetual futures. Runs a 4-phase pipeline — regime detection, signal scoring, risk validation, execution — with conviction-based position sizing and adaptive trailing stops.

## Highlights

- **4-phase pipeline** — SENTINEL (regime) → SCANNER (9-factor confluence) → GUARDIAN (risk gates) → EXECUTOR (conviction sizing)
- **Dry-run by default** — paper trading only; switching to live requires `config.mjs set-mode live --confirm` (no CLI mode override)
- **Stop-loss always on** — every position has SL placed atomically; if SL placement fails, the engine emergency-closes the position
- **Circuit breakers** — 5% daily loss cap, 5 consecutive losses cooldown, 15% max drawdown, 6-position cap, 120-min time stop
- **Step-lock** — every $2 of unrealized profit is locked; SL can never retrace past the last locked level
- **Kill switch** — create `.kairos-data/HALT` to halt at the next cycle boundary; delete to resume
- **Atomic state writes** — tmp + rename so a crash mid-write cannot corrupt `state.json`
- **Safe argv handling** — all CLI arguments to the Hyperliquid binary are passed as discrete argv entries (no shell, no split-on-whitespace)

## Prerequisites

- Node.js 18+
- `hyperliquid` CLI available in PATH (installed from the OKX `hyperliquid-plugin`)
- USDC balance on Hyperliquid (for live mode only)

## Quick Start

```
# Check status (dry-run mode by default)
node scripts/config.mjs status

# Verify connectivity
node scripts/market-data.mjs ping

# Run one analysis + execution cycle (paper trading)
node scripts/kairos-engine.mjs cycle --symbols BTC,ETH,SOL

# Run autonomously for up to 8 hours (duration in minutes, max 480)
node scripts/kairos-engine.mjs auto --duration 120

# Manage open positions (run every 30s during trading)
node scripts/risk-manager.mjs check-positions

# Emergency halt (remove the file to resume)
touch .kairos-data/HALT
```

## Risk Disclaimer

Advanced risk level. This strategy trades perpetual futures with leverage. You can lose all your invested capital. Not financial advice. Never trade with funds you cannot afford to lose.
