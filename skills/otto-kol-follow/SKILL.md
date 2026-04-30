---
name: otto-kol-follow
description: >
  Otto KOL Follow v0.1 — Mirror aggregated sentiment from the top 50 crypto KOLs on Twitter/X
  into a Hyperliquid perpetual with automatic TP/SL brackets. Natural-language intent in, one
  consensus-driven trade out. Dry-run is the default; `--confirm` required for live orders.
  Trigger when the user mentions KOL follow, crypto Twitter sentiment, follow the KOLs,
  consensus trade, crypto X signals, top-50 KOL conviction, who's bullish on {coin},
  mirror the KOLs, or wants to open a Hyperliquid position that tracks crypto-Twitter consensus.
version: "0.1.3"
author: "Otto AI"
updated: 2026-04-24
tags:
  - perpetuals
  - hyperliquid
  - onchainos
  - trading-strategy
  - kol-sentiment
  - social-signals
---

# Otto KOL Follow — Skill Protocol

> Real perpetual-futures trading with leverage on Hyperliquid. Use paper mode (`DRY_RUN = True`) until you understand the strategy. Live loss of capital is possible.

---

## Overview

Otto KOL Follow mirrors aggregated sentiment from the top 50 crypto KOLs on Twitter/X into a single Hyperliquid perpetual trade with automatic take-profit and stop-loss brackets. The Skill takes a user's plain-language request (optionally scoped to a coin), queries Otto AI's live KOL-sentiment feed, and only trades when the cohort shows statistically meaningful consensus.

- **Input.** Natural-language intent, optionally with `coin=BTC` or similar.
- **Decision rule.** Fire a trade only if `kol_count ≥ MIN_KOL_COUNT`, `confidence ≥ MIN_CONFIDENCE_KOL`, and `direction != "flat"`. Skip otherwise.
- **Execution.** All Hyperliquid actions flow through the Hyperliquid Basic Skill (`hyperliquid-plugin`). No raw signing.
- **Leverage.** Conservative cap (`MAX_LEVERAGE_KOL = 3`) — KOL consensus is a lagging signal and high leverage degrades risk-adjusted returns.
- **Entry mode.** Reactive — one trade per user intent. Optional `scripts/bot.py` poller for advanced users.

---

## Pre-flight Checks

### 1. Install onchainos CLI (≥ 2.0.0-beta)

```bash
onchainos --version 2>/dev/null || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh
npx skills add okx/onchainos-skills --yes --global
npx skills add okx/plugin-store --skill plugin-store --yes --global
```

### 2. Install the Hyperliquid Basic Skill (required)

```bash
npx skills add okx/plugin-store --skill hyperliquid-plugin --yes --global
```

### 3. Fund Hyperliquid + register signing address

```bash
hyperliquid-plugin quickstart              # tells you what to do next
hyperliquid-plugin register                # one-time — binds the signing key
hyperliquid-plugin deposit --amount 50 --confirm
```

### 4. Verify the Otto signal feed is reachable

```bash
curl -fsS "https://signals.useotto.xyz/v1/kol-sentiment?coin=BTC" | jq .updated_at
```

If this fails, abort and report "Otto signal feed unreachable" — the Skill does not fabricate signals.

---

## Commands

When the user fires a KOL Follow intent, execute this protocol in order.

### Step 1 — Readiness check

```bash
hyperliquid-plugin quickstart
```

- **When to use**: always, first thing.
- **Output**: JSON with `status: ready | needs_register | no_funds | low_balance`.
- **If not ready**: run `hyperliquid-plugin register` or `hyperliquid-plugin deposit` as indicated. Do NOT proceed.

### Step 2 — Fetch aggregated KOL sentiment

```bash
# Single-coin mode (user said "on ETH")
curl -fsS "https://signals.useotto.xyz/v1/kol-sentiment?coin={COIN}" | jq .

# Top-conviction mode (user didn't specify a coin)
curl -fsS "https://signals.useotto.xyz/v1/kol-sentiment" | jq .
```

- **When to use**: always, after readiness check.
- **Output**:
  ```json
  {
    "updated_at": "2026-04-24T10:00:00Z",
    "status": "ok",
    "coin": "ETH",
    "window_hours": 24,
    "bullish_pct": 0.84,
    "bearish_pct": 0.06,
    "neutral_pct": 0.10,
    "kol_count": 50,
    "mention_count": 312,
    "direction": "long",
    "confidence": 0.78,
    "top_bullish_kols": ["@handle1", "@handle2"],
    "top_bearish_kols": ["@handle3"]
  }
  ```
- **Selection rule**:
  - Reject if `kol_count < MIN_KOL_COUNT` (config default 40) — insufficient cohort sample.
  - Reject if `confidence < MIN_CONFIDENCE_KOL` (config default 0.70).
  - Reject if `direction == "flat"` — no meaningful consensus.
  - Otherwise, proceed with `direction` + `confidence`.

### Step 3 — Confirm parameters with the user

Summarize explicitly before placing anything:

> I'm about to open a **{direction}** **{coin}** perp on Hyperliquid based on KOL consensus.
> • Size: **${size_usd}** at **{leverage}x** leverage (≈ {notional_usd} notional)
> • Stop-loss: **{sl_pct}%** ({sl_price})
> • Take-profit: **{tp_pct}%** ({tp_price})
> • Cohort: **{kol_count} KOLs** over **{window_hours}h**, **{bullish_pct:.0%}** bullish / **{bearish_pct:.0%}** bearish
> • Confidence: **{confidence}** (Otto threshold: {MIN_CONFIDENCE_KOL})
> • Top bullish voices: {top_bullish_kols[:3]}
> Dry run? {DRY_RUN}
>
> Reply "confirm" to execute live, "paper" to run dry-only, or "cancel" to abort.

**Do NOT proceed to Step 4 without explicit user confirmation.**

### Step 4 — Read current price

```bash
hyperliquid-plugin prices --coin {COIN}
```

- **When to use**: once per trade, for TP/SL anchoring.
- **Output**: JSON with `mark_px`.

### Step 5 — Place the order

```bash
hyperliquid-plugin order --coin {COIN} --side {buy|sell} --size {size} --leverage {leverage} --strategy-id otto-kol-follow --confirm
```

- **Size conversion**: `size_tokens = size_usd / mark_px`, rounded down to lot size. `size_usd` is the position **notional**; `--leverage` controls posted margin (margin = notional / leverage).
- **Leverage**: capped at `MAX_LEVERAGE_KOL = 3`. Never above the coin's protocol cap.
- **Dry-run**: if `DRY_RUN = True`, omit `--confirm`.
- **Strategy attribution**: always include `--strategy-id otto-kol-follow`.

### Step 6 — Attach TP/SL bracket

```bash
hyperliquid-plugin tpsl --coin {COIN} --sl-px {sl_price} --tp-px {tp_price} --confirm
```

- Atomic — pass both prices in the same call.
- `sl_price = mark_px × (1 ∓ SL_PCT)`, mirrored for shorts.

### Step 7 — Report back

```
✓ Otto KOL Follow
  {side} {size} {coin} @ ~${mark_px}  ({leverage}x, ${notional_usd} notional)
  SL ${sl_price} ({sl_pct}%)  TP ${tp_price} ({tp_pct}%)
  Consensus: {bullish_pct:.0%} bullish from {kol_count} KOLs over {window_hours}h
  Confidence: {confidence}
  Entry tx: {tx_hash}
  Bracket tx: {bracket_tx_hash}
```

### Configuration commands

Tunable parameters live in `scripts/config.py`:

```bash
grep -E "^[A-Z_]+ " scripts/config.py     # view defaults
```

### Optional autonomous poller

```bash
python3 scripts/bot.py --interval 600 --coin BTC         # dry-run by default
python3 scripts/bot.py --interval 600 --live             # live, still gated by config.DRY_RUN
```

- **When to use**: user explicitly opts into hands-free hourly polling.
- **Output**: JSONL log of fetches + decisions.

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| `hyperliquid-plugin: command not found` | Basic Skill not installed | Run Pre-flight step 2. |
| `status: needs_register` | Signing address not bound | `hyperliquid-plugin register`. |
| `status: no_funds` / `low_balance` | Insufficient USDC on HL | `hyperliquid-plugin deposit --amount N --confirm`. |
| Signal feed HTTP 5xx / 503 | Otto backend transient failure | Retry ONCE after 3s. If still failing, abort. |
| Signal feed HTTP 429 | Rate limit | Back off 30s, retry ONCE. Tell user to space out calls. |
| `kol_count < MIN_KOL_COUNT` | Cohort too small this window | Abort: "KOL cohort too small to trade right now." |
| `confidence < MIN_CONFIDENCE_KOL` | Consensus too weak | Abort: "KOL consensus not strong enough — no trade." |
| `direction == "flat"` | Mixed sentiment | Abort: "KOLs are split right now — no directional trade." |
| `hyperliquid-plugin order` fails | Order rejected | Return plugin error verbatim. Do NOT simulate via raw API. |
| `tpsl` fails after `order` | Partial bracket | Warn user + suggest manual bracket or `close`. |
| User explicitly aborts at Step 3 | Declined | Do not place. Do not retry. |
| `SESSION_MAX_DRAWDOWN_PCT` breached | Cumulative session loss | Refuse all new trades until reset. |

---

## Security Notices

**Risk level: `advanced`**. This Skill places leveraged perpetual trades on the user's behalf; loss of capital is possible.

### Safeguards enforced by this Skill

- **Dry-run default.** `DRY_RUN = True` in `scripts/config.py`. `--confirm` is required for every live order.
- **Hard position cap.** `MAX_POSITION_PCT_EQUITY = 0.10`.
- **Stop-loss.** Every order attaches an atomic bracket with `SL_PCT = 0.02` (2% default).
- **Low leverage cap.** `MAX_LEVERAGE_KOL = 3`. KOL consensus is a lagging signal and the cap reflects that.
- **Cohort-size filter.** Rejects trades when `kol_count < MIN_KOL_COUNT = 40` — prevents thin-sample mirror trades.
- **Confidence threshold.** Rejects trades below `MIN_CONFIDENCE_KOL = 0.70`.
- **Session drawdown halt.** Cumulative `-15%` session P&L halts new trades.
- **No key handling.** All signing flows through `hyperliquid-plugin`'s TEE-backed Agentic Wallet.
- **No credentials in source.** No API keys, tokens, or secrets committed.
- **Declared network surface.** Only `signals.useotto.xyz` and `api.hyperliquid.xyz` (via `hyperliquid-plugin`).

### Things this Skill will NOT do

- **Never** place a live order without explicit user "confirm" at Step 3.
- **Never** bypass the Hyperliquid Basic Skill. All Hyperliquid actions MUST flow through `hyperliquid-plugin`.
- **Never** fabricate KOL sentiment. If the signal feed is unreachable or `status: degraded`, the Skill aborts.
- **Never** trade on a cohort smaller than `MIN_KOL_COUNT` — small-sample mirror trades are noise.
- **Never** exceed `MAX_LEVERAGE_KOL` without explicit user override, capped at the coin's protocol max.
- **Never** reconstruct EIP-712 / L1 signatures by hand.
- **Never** stack a second position on the same coin silently — ask the user.

### Risk disclaimer

**This Skill is provided solely for educational research and technical reference purposes. It does not constitute investment advice.**

1. **High risk.** Perpetual futures are leveraged. Liquidation is possible.
2. **KOL signals are noisy.** Crypto Twitter sentiment lags price, is vulnerable to manipulation, and can be reflexively wrong at tops and bottoms. Otto's filters reduce but do not eliminate this.
3. **Parameters are reference-only.** Defaults in `scripts/config.py` are not tuned for your risk tolerance.
4. **Hyperliquid-specific risks.** Geofencing, insurance-fund haircuts, keeper failures, L1 outages. User must confirm jurisdictional eligibility.
5. **No profit guarantee.** Past KOL accuracy ≠ future accuracy.
6. **Regulatory risk.** User is solely responsible for compliance, taxes, KYC.
7. **Assumption of responsibility.** Strategy is AS-IS. Authors, Otto AI, OKX, and affiliates are not liable for losses.

### No claim of OKX endorsement

This Skill is authored by Otto AI, a community developer. Not an OKX-endorsed product. "Hyperliquid" and "OKX Onchain OS" are referenced as execution venues, not as affiliated entities.

---

## Config reference

See `scripts/config.py`. Key defaults:

- `DRY_RUN = True`
- `DEFAULT_SIZE_USD = 25`
- `MAX_POSITION_PCT_EQUITY = 0.10`
- `MAX_LEVERAGE_KOL = 3`
- `MAX_LEVERAGE_ABSOLUTE = 10`
- `SL_PCT = 0.02` (2%)
- `TP_PCT = 0.04` (4%)
- `MIN_KOL_COUNT = 40`
- `MIN_CONFIDENCE_KOL = 0.70`
- `MIN_VOLUME_USD = 10_000_000`
- `SESSION_MAX_DRAWDOWN_PCT = 0.15`
- `WINDOW_HOURS_DEFAULT = 24`

---

## Onchain OS Integration

This Skill runs inside Onchain OS Agentic Wallet. All Hyperliquid interactions go through `hyperliquid-plugin`, which uses the TEE-backed signing context of the user's connected wallet. No private keys leave Onchain OS.

---

## Links

- Otto AI: https://useotto.xyz
- Signal feed contract: [SIGNAL_FEED_CONTRACT.md](../SIGNAL_FEED_CONTRACT.md)
- Docs: https://docs.useotto.xyz
- Source: https://github.com/useOttoAI/plugin-store
