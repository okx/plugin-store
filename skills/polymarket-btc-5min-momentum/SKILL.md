---
name: polymarket-btc-5min-momentum
version: "0.1.0"
description: >
  Polymarket BTC 5-minute Up/Down momentum strategy — bets on the next 5-min
  BTC prediction-market slot based on recent short-term BTC price momentum
  fetched from Binance public klines. Routes all buy orders through
  polymarket-plugin with --strategy-id so every trade is attributed to this
  strategy on the OKX backend.
updated: 2026-04-23
triggers: >
  polymarket btc strategy, btc 5min strategy, btc 5 minute momentum,
  momentum strategy polymarket, polymarket bot, btc updown bot,
  btc-5m strategy, polymarket btc 自动下单, 5 分钟动量策略,
  polymarket 策略
---

# Polymarket BTC 5-min Momentum Strategy

A small demonstration strategy that bets the direction of Polymarket's **rolling 5-minute BTC Up/Down markets** based on recent short-term BTC price momentum pulled from Binance public spot klines.

> ⚠️ **Demo, not a profit engine.** A single momentum window is not a serious edge on a ±$0.50 binary market. Treat this skill as a reference implementation of the `category: strategy` pattern — it shows how to chain a Python orchestrator, a dependent plugin (`polymarket-plugin`), and the new `--strategy-id` attribution path.

---

## Architecture

```
┌─────────────────────────┐    polymarket-plugin get-series    ┌──────────────────────────┐
│   strategy.py (Python)  │───────────────────────────────────▶│  polymarket-plugin       │
│                         │                                    │   (dependent plugin)     │
│   1. read next slot     │    binance klines (public HTTP)    │                          │
│   2. fetch 1m klines    │◀───────────────────────────────────│                          │
│   3. decide Up/Down/skip│                                    │                          │
│   4. buy --strategy-id  │    polymarket-plugin buy           │                          │
│                         │───────────────────────────────────▶│  CLOB order placement    │
└─────────────────────────┘                                    └──────────────────────────┘
```

All trading credentials, signing, on-chain settlement, and attribution reporting live inside `polymarket-plugin`. This skill only decides **which direction to bet** and **invokes the plugin**. No private keys. No network ops beyond Binance + the `polymarket-plugin` subprocess.

---

## Dependencies

- **Runtime**: Python ≥ 3.9 (stdlib only — no pip installs required)
- **Dependent plugin**: [`polymarket-plugin`](../polymarket-plugin) ≥ 0.4.10 (must be on `PATH`; ships `--strategy-id` attribution)
- **Public API**: `https://api.binance.com/api/v3/klines` (no auth, spot public)

Install the dependent plugin if it's not already present:

```bash
npx skills add okx/plugin-store --skill polymarket-plugin --yes --global
```

Polymarket itself additionally needs:
- An onchainos wallet with USDC.e on Polygon
- Polymarket trading mode configured (`polymarket-plugin setup-proxy` or `switch-mode`)
- Not being in a geo-restricted region (US is blocked)

See `polymarket-plugin`'s SKILL.md for full onboarding.

---

## Usage

### Preview the next slot's decision (no trade)

```bash
python3 strategy.py --dry-run
```

Output shape:

```json
{
  "strategy_id": "polymarket-btc-5min-momentum",
  "slot_condition_id": "0x...",
  "slot_question": "Bitcoin Up or Down - April 23, ...",
  "slot_up_price": 0.505,
  "slot_down_price": 0.495,
  "slot_seconds_remaining": 312,
  "momentum_pct": 0.082,
  "momentum_window_min": 15,
  "threshold_pct": 0.05,
  "decision": "Up",
  "trade": { ... polymarket-plugin buy --dry-run output ... }
}
```

### Place one real bet on the next slot

```bash
python3 strategy.py --amount 2.5
```

### Run continuously (one slot per 5 minutes)

```bash
python3 strategy.py --loop --amount 2.5
```

The loop sleeps until 10 seconds past each UTC 5-minute boundary before evaluating the next slot, so Polymarket has time to list the market.

### Tuning

| Flag | Default | Meaning |
|------|---------|---------|
| `--amount` | `2.5` | USDC.e per bet. Minimum ~$2.50 for 5-share minimum at ~$0.50 outcome price. |
| `--threshold` | `0.05` | Skip the slot when `|momentum_pct|` is below this (percent). Higher = fewer trades, stronger conviction. |
| `--window` | `15` | Minutes of 1m klines used to compute momentum. |
| `--dry-run` | off | Preview the buy without submitting. |
| `--loop` | off | Keep running across slots. |

---

## Strategy attribution

Every buy submitted by `strategy.py` invokes `polymarket-plugin buy` with:

```
--strategy-id polymarket-btc-5min-momentum
```

After the CLOB accepts the order, `polymarket-plugin` pushes a report-plugin-info payload to the OKX backend (see its SKILL.md for the exact schema). All bets placed by this skill aggregate under the strategy ID `polymarket-btc-5min-momentum`, so realized PnL can be attributed cleanly.

**Write ops in this skill:**
- `strategy.py` line ~93: `polymarket-plugin buy … --strategy-id polymarket-btc-5min-momentum` ✅

**Read-only ops** (no `--strategy-id` needed):
- `strategy.py` line ~67: `polymarket-plugin get-series --series btc-5m`

---

## Output and logging

- **stdout**: a single JSON report per `run_once()` invocation — decision + trade result. Machine-parseable.
- **stderr**: human-readable progress markers (when outcome is placed) and any errors; non-fatal in `--loop` mode.

Exit codes: `0` on successful decision (including intentional skip); non-zero only when the CLI arguments themselves are invalid.

---

## Safety notes

- Uses `polymarket-plugin`'s **FOK market order** with `--round-up` — orders fill immediately at the worst ask or fail cleanly; they do not rest on the book across slots.
- No `--confirm` equivalent in this skill: each `run_once()` call is one-shot. The human safety gate is at invocation (or at the `--loop` step).
- `--dry-run` forwards to `polymarket-plugin buy --dry-run`, so no wallet interaction happens.

---

## Known limitations

- **Single-signal strategy.** No filtering for market regime, liquidity, or recent slot outcomes. Real attribution-grade strategies should combine multiple signals and size accordingly.
- **No position sizing** beyond a fixed `--amount`. A bet-to-equity ratio would be a natural follow-up.
- **No REDEEM automation.** Winners must call `polymarket-plugin redeem --strategy-id polymarket-btc-5min-momentum` manually (or via a separate scheduled worker) to convert winning tokens to USDC. The OKX attribution backend reverse-looks REDEEM events by `(proxyAddress, market_id)` so the redemption is credited to the strategy even if the user forgets the flag.

---

## Files

| File | Purpose |
|------|---------|
| `plugin.yaml` | Plugin Store manifest (declares `category: strategy` + `dependent_plugin: polymarket-plugin`) |
| `strategy.py` | Main orchestrator (stdlib only) |
| `SKILL.md` | This document |
| `SKILL_SUMMARY.md` | Short summary for the plugin registry |
| `README.md` | Developer-facing quickstart |
| `LICENSE` | MIT |

---

## Changelog

### v0.1.0 (2026-04-23)

- **feat**: Initial release — single-window BTC momentum on 5-min Polymarket slots, always attributed via `--strategy-id polymarket-btc-5min-momentum` on each `polymarket-plugin buy` call.
