# rwa-alpha

## 1. Overview
Real World Asset intelligence trading skill that detects macro events (Fed rate decisions, CPI releases, gold moves, SEC rulings) via NewsNow headlines + Polymarket probability confirmation, then auto-trades 15 tokenized treasury/gold/yield/governance tokens via OKX DEX with dual exit systems. Features 3-layer macro event detection (keyword regex → LLM confirm → LLM discover), 3 strategy modes, and composite sentiment scoring. Paper mode + PAUSED=True by default.

## 2. Prerequisites
- Python 3.8+
- OnchainOS CLI (`onchainos`) at `~/.local/bin/onchainos` — for wallet operations and DEX swaps
- OKX Agentic Wallet logged in (`onchainos wallet status`)
- Optional: `ANTHROPIC_API_KEY` env var for LLM headline classification (falls back to keyword-only without it)
- No pip dependencies required (Python stdlib only)

## 3. Quick Start
```bash
# Start in paper mode (default)
cd skills/rwa-alpha
python3 rwa_alpha.py
# → Dashboard at http://localhost:3249

# Switch to live mode (edit config.py)
# MODE = "live"
# PAUSED = False
```

Supports 15 RWA tokens across Ethereum and Solana: treasury (USDY, OUSG, sDAI, bIB01), gold (PAXG, XAUT), DeFi yield (USDe), governance (ONDO, CFG, MPL, PENDLE, PLUME, OM, GFI, TRU). 3 strategy modes: Yield Optimizer / Macro Trader / Full Alpha.
