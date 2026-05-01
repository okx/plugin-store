---
name: polymarket-liquidity-sniper
description: "AI-powered autonomous sniper that detects and trades mispriced thin-liquidity markets on Polymarket — multi-LLM (Groq → OpenRouter fallback), built for transaction-count dominance"
version: "1.0.0"
author: "mestarkris"
tags:
  - polymarket
  - prediction-markets
  - liquidity-sniper
  - autonomous
  - groq
  - openrouter
  - multi-llm
  - okx
---

# Polymarket Liquidity Sniper

## Overview

This skill makes an AI agent **autonomously scan every active Polymarket prediction
market**, score each one for pricing inefficiency, and execute rapid buy-then-sell
trades to capture spread — all while cycling through multiple LLMs (Groq primary →
OpenRouter fallback with 8+ models) to ensure uninterrupted autonomous operation.

The agent uses the `polymarket-plugin` CLI (from OKX plugin-store) for all
on-chain trade execution via the OKX Agentic Wallet, and a Cloudflare Worker proxy
to handle Polymarket CLOB authentication without IP restrictions.

> ⚠️ **RISK DISCLAIMER — READ BEFORE USE**
> This plugin executes real USDC.e trades on Polygon. All prediction market trading
> involves total loss of capital. Never deposit more than you can afford to lose.
> Always validate with `--dry-run` before live trading. This is not financial advice.

---

## Pre-flight Checks

### Step 1 — Install dependencies
```bash
pip install requests python-dotenv colorama groq openai --break-system-packages
```

### Step 2 — Install polymarket-plugin CLI
```bash
npx skills add okx/plugin-store --skill polymarket-plugin
```

### Step 3 — Create `.env` file
```bash
cp .env.example .env
nano .env
```

Fill in all required values (see `.env.example`).

### Step 4 — Refresh Polymarket credentials
```bash
python3 scripts/refresh_creds.py
```
Expected output: `Got key via derive` or `Got key via create` then `Credentials refreshed!`

### Step 5 — Run dry-run to verify
```bash
python3 scripts/auto_trader.py --dry-run
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                 POLYMARKET LIQUIDITY SNIPER                  │
│                                                              │
│  ┌─────────────┐   ┌──────────────┐   ┌──────────────────┐  │
│  │ MARKET SCAN │──▶│ LLM ANALYZER │──▶│ TRADE DECISION   │  │
│  │ polymarket- │   │ Groq primary │   │ pick_trade()     │  │
│  │ plugin CLI  │   │ → OpenRouter │   │ YES/NO binary    │  │
│  └─────────────┘   └──────────────┘   └────────┬─────────┘  │
│                                                │              │
│  ┌─────────────┐   ┌──────────────┐   ┌────────▼─────────┐  │
│  │ CREDENTIALS │   │  BUY → SELL  │◀──│   EXECUTOR       │  │
│  │ Cloudflare  │   │  immediate   │   │ polymarket-plugin│  │
│  │ Worker proxy│   │  scalp       │   │ EOA mode         │  │
│  └─────────────┘   └──────────────┘   └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**LLM Fallback Chain** (auto-managed by `scripts/llm_router.py`):
```
1. Groq  → llama-3.3-70b-versatile      (primary, fastest)
2. Groq  → llama-3.1-8b-instant         (backup Groq)
3. OpenRouter → meta-llama/llama-3.3-70b-instruct:free
4. OpenRouter → anthropic/claude-3-haiku
5. OpenRouter → deepseek/deepseek-chat
6. OpenRouter → mistralai/mistral-small
7. OpenRouter → meta-llama/llama-3.1-70b-instruct
8. OpenRouter → google/gemini-pro
```

---

## Commands

### `auto_trader.py` — Main autonomous loop

```bash
python3 scripts/auto_trader.py [--dry-run]
```

**When to use:** Primary entry point. Continuously scans markets, buys and
immediately sells shares to capture spread. Refreshes credentials automatically.

**Flags:**

| Flag | Description |
|------|-------------|
| `--dry-run` | Simulate trades, never execute real orders |

**Run in background (recommended):**
```bash
cd ~/plugin-store/polymarket-sniper
nohup python3 scripts/auto_trader.py > logs/auto_trader.log 2>&1 &
echo "PID: $!"
```

**Monitor live:**
```bash
tail -f logs/auto_trader.log
```

**Stop:**
```bash
kill $(cat logs/trader.pid)
# or
pkill -f auto_trader.py
```

---

### `refresh_creds.py` — Refresh Polymarket API credentials

```bash
python3 scripts/refresh_creds.py
```

**When to use:** Run before starting the trader. Also run when you see
`STALE_CREDENTIALS` errors. Uses a Cloudflare Worker proxy to bypass
IP restrictions on the Polymarket CLOB auth endpoint.

**Expected output:**
```
Got key via derive
Credentials refreshed!
```

---

### `sniper.py` — Alternative sniper with edge scoring

```bash
python3 scripts/sniper.py --dry-run --scan-limit 30 --cycles 1
```

**When to use:** Alternative to `auto_trader.py`. Uses LLM confidence scoring
and edge detection. Good for validating the setup.

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--dry-run` | on | Simulate, never execute |
| `--live` | off | Enable real trading |
| `--cycles N` | ∞ | Stop after N cycles (0=forever) |
| `--scan-limit N` | 100 | Markets per cycle |
| `--interval S` | 45 | Seconds between cycles |

---

### Manual sell — Close a position

If the auto trader leaves an open position, sell it manually:

```bash
# Step 1 — refresh credentials
python3 scripts/refresh_creds.py

# Step 2 — check positions
polymarket-plugin get-positions

# Step 3 — sell (replace values with actual position data)
polymarket-plugin sell \
  --market-id MARKET_SLUG \
  --outcome yes \
  --shares N \
  --order-type FOK
```

---

### Balance check

```bash
polymarket-plugin balance
onchainos wallet balance --chain polygon
```

---

## Strategy: Buy-Then-Immediately-Sell

The core scalping loop in `auto_trader.py`:

1. Scan 50 active markets via `polymarket-plugin list-markets`
2. Filter: only binary YES/NO markets, price between 0.10–0.90, skip sports/esports
3. Pick the best opportunity (lowest price side = most shares per dollar)
4. **BUY** shares via `polymarket-plugin buy --market-id ... --outcome ... --amount 2`
5. Wait 5 seconds
6. **SELL** the same shares via `polymarket-plugin sell --market-id ... --outcome ... --shares N`
7. Log PnL to `logs/auto_trader.log`
8. Rotate to next market (never repeat same market in a session)
9. Repeat every 120 seconds

---

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| `Could not derive api key` | IP blocked by Cloudflare | Run `python3 scripts/refresh_creds.py` — uses worker proxy |
| `STALE_CREDENTIALS` | API key expired | Run `python3 scripts/refresh_creds.py` then retry |
| `Insufficient balance` | Not enough USDC.e | Send USDC.e to EOA wallet on Polygon |
| `invalid amount, min size: $1` | Trade size too small | Set `MAX_TRADE_USDC=2` in `.env` |
| `Outcome not found` | Market has non-binary outcomes | Filter is automatic — skip sports/team markets |
| `Groq 429 rate limit` | Groq quota hit | LLM Router auto-falls to OpenRouter |
| `OpenRouter 404` | Dead model endpoint | Router skips and tries next model |
| `TX_NOT_CONFIRMED` | Polygon congestion | Re-run `polymarket-plugin setup-proxy` |
| `have 0.000000 shares` | Shares in wrong wallet mode | Check mode: `polymarket-plugin switch-mode --mode eoa` |

---

## Security Notices

- **Risk Level: ADVANCED** — executes autonomous trades without per-trade confirmation
- Private key is read from `.env` only — never hardcoded
- Add `.env` to `.gitignore` — never commit credentials
- `MAX_SESSION_USDC` is a hard kill switch — set to amount you can afford to lose
- `MAX_TRADE_USDC` caps each individual trade size
- Dry-run mode (`--dry-run`) is always safe — no real orders placed
- All trades logged to `logs/auto_trader.log` and `logs/trades.jsonl`
- Credentials stored in `~/.config/polymarket/creds.json` (chmod 600)
- No wallet data sent to any server beyond Polymarket and OKX APIs

---

## Skill Routing

- For bridging USDC to Polygon → use `onchainos swap` or send from OKX exchange
- For wallet balance → `onchainos wallet balance --chain polygon`
- For market data only (no trading) → `polymarket-plugin list-markets`
- For checking leaderboard → `onchainos leaderboard list --chain polygon --time-frame 7 --sort-by volume`
