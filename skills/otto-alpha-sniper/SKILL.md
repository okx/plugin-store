---
name: otto-alpha-sniper
description: >
  Otto Alpha Sniper v0.1 — Natural-language intent → live Hyperliquid perp execution, driven
  by Otto AI's multi-source alpha signals (trending altcoins, funding-rate skews, KOL sentiment,
  filtered crypto news). Opens perps with automatic TP/SL brackets inside Onchain OS Agentic
  Wallet. Dry-run is the default; `--confirm` required for live orders.
  Trigger when the user mentions Otto alpha, alpha sniper, data-driven perp, momentum trade,
  trending altcoin scalp, KOL follow, funding-rate fade, smart-money perp, Otto AI signals,
  or wants to automatically open a Hyperliquid position based on aggregated crypto intel.
version: "0.1.0"
author: "Otto AI"
updated: 2026-04-24
tags:
  - perpetuals
  - hyperliquid
  - onchainos
  - trading-strategy
  - alpha-signals
  - kol-sentiment
  - funding-rates
  - trending
---

# Otto Alpha Sniper — Skill Protocol

> Real perpetual-futures trading with leverage on Hyperliquid. Use paper mode (`DRY_RUN = True`) until you understand the strategy. Live loss of capital is possible.

---

## Overview

Otto Alpha Sniper turns a user's plain-language trading intent into a single Hyperliquid perpetual trade with automatic take-profit and stop-loss brackets, driven by Otto AI's production alpha signals.

Three signal modes, selectable per trade:

- **Trending** — live altcoin momentum from Otto's 24h trending-altcoins feed (sentiment-weighted).
- **KOL-Follow** — aggregated sentiment across the top 50 crypto KOLs on Twitter/X.
- **Funding-Fade** — captures extreme funding-rate skews (long the most-shorted, short the most-longed).

Every Hyperliquid action flows through the Hyperliquid Basic Skill (`hyperliquid-plugin`). No raw EIP-712 signing, no private keys, no bypass. The Skill defaults to `DRY_RUN = True`; a user `--confirm` is required before anything hits the exchange.

**Entry mode.** Reactive — the AI agent fires one trade per user intent. An optional `scripts/bot.py` autonomous poller is provided for advanced users.

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
hyperliquid-plugin deposit --amount 50 --confirm   # bridges USDC from Arbitrum
```

### 4. Verify the Otto signal feed is reachable

```bash
curl -fsS "https://signals.useotto.xyz/v1/trending?limit=1" | jq .updated_at
```

If this fails, abort and report "Otto signal feed unreachable" — the Skill will not fabricate signals.

---

## Commands

When the user fires an Otto Alpha Sniper intent, execute this 7-step protocol in order. Each command lists **when to use**, **output**, and a concrete example.

### Step 1 — Readiness check

```bash
hyperliquid-plugin quickstart
```

- **When to use**: Always, before any trade.
- **Output**: JSON with `status: ready | needs_register | no_funds | low_balance`.
- **If not ready**: run `hyperliquid-plugin register` or `hyperliquid-plugin deposit` as indicated. Do NOT proceed to Step 2.

### Step 2 — Fetch the signal for the requested mode

Pick ONE mode based on user intent. If ambiguous, ask the user.

#### Mode: `trending`
```bash
curl -fsS "https://signals.useotto.xyz/v1/trending?limit=5" | jq .
```
- **When to use**: user says "what's trending", "momentum", "strongest altcoin", "scalp".
- **Output**:
  ```json
  {
    "updated_at": "2026-04-24T10:00:00Z",
    "signals": [
      {"coin": "SOL", "score": 0.87, "direction": "long", "reason": "24h momentum + bullish KOL divergence", "confidence": 0.72},
      {"coin": "AVAX", "score": 0.81, "direction": "short", "reason": "breakdown + funding flipped positive", "confidence": 0.64}
    ]
  }
  ```
- **Selection rule**: pick the entry with the highest `score × confidence` product. If the top score < `MIN_SCORE` (config), abort with "no signal strong enough right now".

#### Mode: `kol-follow`
```bash
curl -fsS "https://signals.useotto.xyz/v1/kol-sentiment?coin=<USER_COIN>" | jq .
```
- **When to use**: user says "KOLs", "follow the crowd", "what are KOLs saying about {coin}".
- **Output**:
  ```json
  {
    "coin": "ETH",
    "bullish_pct": 0.84,
    "bearish_pct": 0.06,
    "neutral_pct": 0.10,
    "kol_count": 50,
    "updated_at": "2026-04-24T10:00:00Z",
    "direction": "long",
    "confidence": 0.78
  }
  ```
- **Selection rule**: use `direction` + `confidence`. Abort if `confidence < MIN_CONFIDENCE_KOL`.

#### Mode: `funding-fade`
```bash
curl -fsS "https://signals.useotto.xyz/v1/funding-extremes?limit=3" | jq .
```
- **When to use**: user says "fade funding", "most longed", "most shorted", "funding skew".
- **Output**:
  ```json
  {
    "most_longed": {"coin": "DOGE", "funding_8h": 0.0012, "action": "short"},
    "most_shorted": {"coin": "LINK", "funding_8h": -0.0009, "action": "long"}
  }
  ```
- **Selection rule**: pick the `action` on whichever side has the more extreme funding magnitude, or follow user's side if specified.

### Step 3 — Confirm parameters with the user

Before placing anything, summarize explicitly:

> I'm about to open a **{direction}** **{coin}** perp on Hyperliquid.
> • Size: **${size_usd}** at **{leverage}x** leverage (≈ {notional_usd} notional)
> • Stop-loss: **{sl_pct}%** below entry (${sl_price})
> • Take-profit: **{tp_pct}%** above entry (${tp_price})
> • Mode: **{mode}** (Otto signal score: {score}, confidence: {confidence})
> Dry run? {DRY_RUN}
>
> Reply "confirm" to execute live, "paper" to run dry-only, or "cancel" to abort.

**Do NOT proceed to Step 4 without explicit user confirmation.**

### Step 4 — Read current price

```bash
hyperliquid-plugin prices --coin {COIN}
```

- **When to use**: once per trade, for TP/SL anchoring.
- **Output**: JSON with `mark_px` (current mark price).
- **Usage**: use `mark_px` as the entry reference for TP/SL bracket calculation.

### Step 5 — Place the order

```bash
hyperliquid-plugin order --coin {COIN} --side {buy|sell} --size {size} --leverage {leverage} --strategy-id otto-alpha-sniper --confirm
```

- **When to use**: after Step 3 confirmation and Step 4 price read.
- **Output**: JSON with `order_id`, `filled_px`, `tx_hash`.
- **Size conversion**: `size = size_usd × leverage / mark_px`, rounded down to the coin's lot size. `--size` is in token units, not USD.
- **Leverage**: always pass `--leverage {leverage}` explicitly — never rely on the exchange default.
- **Dry-run**: if `DRY_RUN = True`, omit `--confirm`. Plugin will echo the intended order without submitting.
- **Strategy attribution**: every live order must include `--strategy-id otto-alpha-sniper` for leaderboard attribution on the OKX Plugin Store Developer Challenge.

### Step 6 — Attach TP/SL bracket

```bash
hyperliquid-plugin tpsl --coin {COIN} --sl-px {sl_price} --tp-px {tp_price} --strategy-id otto-alpha-sniper --confirm
```

- **When to use**: immediately after Step 5 (same turn).
- **Output**: JSON with `bracket_id`, `tx_hash`.
- **Calculation**: `sl_price = mark_px × (1 - SL_PCT)` for longs, `mark_px × (1 + SL_PCT)` for shorts. Mirror for `tp_price` with `TP_PCT`.
- **Atomic**: pass both `--sl-px` and `--tp-px` in the same call so the bracket lands atomically.

### Step 7 — Report back to the user

Return a compact trade card:

```
✓ Otto Alpha Sniper — {mode}
  {side} {size} {coin} @ ~${mark_px}  ({leverage}x, ${notional_usd} notional)
  SL ${sl_price} ({sl_pct}%)  TP ${tp_price} ({tp_pct}%)
  Signal score: {score}  Confidence: {confidence}
  Reason: {reason}
  Entry tx: {tx_hash}
  Bracket tx: {bracket_tx_hash}
```

### Configuration commands

Tunable parameters live in `scripts/config.py` — hot-reloaded on every trade. Common tweaks:

```bash
# View current values
grep -E "^[A-Z_]+ " scripts/config.py

# Switch to live mode (still requires --confirm per trade)
#   edit scripts/config.py: DRY_RUN = False

# Reduce position size cap
#   edit scripts/config.py: MAX_POSITION_PCT_EQUITY = 0.05
```

### Optional autonomous poller

For advanced users only:

```bash
python3 scripts/bot.py --mode trending --interval 300      # dry-run by default
python3 scripts/bot.py --mode funding-fade --interval 900 --live
```

- **When to use**: user explicitly opts into hands-free operation.
- **Output**: JSONL log of every signal fetch + trade decision to `otto_sniper_trades.jsonl`.
- **Precondition**: run at least 10 `--once` dry cycles and review logs before `--live`.

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| `hyperliquid-plugin: command not found` | Basic Skill not installed | Run Pre-flight step 2. |
| `status: needs_register` | Signing address not bound to HL account | `hyperliquid-plugin register` and have the user approve. |
| `status: no_funds` / `low_balance` | Insufficient USDC on Hyperliquid | `hyperliquid-plugin deposit --amount N --confirm`. Do NOT retry automatically. |
| Signal feed HTTP 5xx | Otto backend transient failure | Retry ONCE after 3s. If still failing, abort with "Otto signal feed unreachable". |
| Signal feed HTTP 429 | Rate limit from same IP | Back off 30s, retry ONCE. Tell user to space out calls. |
| Signal feed returns empty `signals` array | No asset met threshold this cycle | Abort: "No signal strong enough right now — try again later or a different mode." |
| `score < MIN_SCORE` after selection | Top signal below config threshold | Abort without placing. Do NOT lower threshold dynamically. |
| `hyperliquid-plugin order` non-zero exit | Order rejected (margin, liquidity, or exchange error) | Return the plugin's error verbatim. Do NOT attempt to simulate via raw API. |
| `hyperliquid-plugin tpsl` fails after `order` succeeds | Partial bracket — position open without SL/TP | Immediately warn user + suggest manual bracket or `hyperliquid-plugin close`. |
| User explicitly aborts at Step 3 | User declined the trade summary | Do not place. Do not retry. Do not ask a second time. |
| `SESSION_MAX_DRAWDOWN_PCT` breached | Cumulative P&L below threshold | Refuse all new trades until next session (config reset). Tell user their session hit the drawdown halt. |

---

## Security Notices

**Risk level: `advanced`** (per OKX Plugin Store risk-level taxonomy). This Skill places leveraged perpetual trades on a user's behalf; loss of capital is possible.

### Safeguards enforced by this Skill

- **Dry-run default.** `DRY_RUN = True` in `scripts/config.py`. `--confirm` is required for every live order.
- **Hard position cap.** `MAX_POSITION_PCT_EQUITY = 0.10` — a single trade may never exceed 10% of the user's Hyperliquid equity.
- **Stop-loss.** Every order attaches a bracket with `SL_PCT = 0.02` (2% default) in the same agent turn.
- **Leverage caps per mode.** Trending 5x, KOL 3x, funding-fade 10x. Never exceeds `MAX_LEVERAGE_ABSOLUTE = 20`. Always min()'d against the coin's exchange-level cap.
- **Liquidity filter.** `MIN_VOLUME_USD = 10_000_000` — illiquid coins are skipped.
- **Session drawdown halt.** New trades refused after cumulative `-15%` session P&L.
- **No key handling.** All signing flows through `hyperliquid-plugin`'s TEE-backed Agentic Wallet. This Skill never touches private keys, seed phrases, or raw EIP-712 signatures.
- **No credentials in source.** No API keys, tokens, or secrets committed anywhere in this repo.
- **Declared network surface.** Only `signals.useotto.xyz` and `api.hyperliquid.xyz` (via `hyperliquid-plugin`) — listed in `api_calls`. No other outbound calls.

### Things this Skill will NOT do

- **Never** place a live order without an explicit user "confirm" at Step 3.
- **Never** bypass the Hyperliquid Basic Skill. All Hyperliquid actions MUST flow through `hyperliquid-plugin` — this is a submission-eligibility requirement of the OKX Plugin Store Developer Challenge.
- **Never** fabricate a signal. If the Otto signal feed is unreachable, the Skill aborts.
- **Never** exceed configured leverage caps without a user-override flag, and never above the per-coin protocol max.
- **Never** size above `MAX_POSITION_PCT_EQUITY` of Hyperliquid account equity.
- **Never** reconstruct EIP-712 / L1 action signatures by hand.
- **Never** stack a second position on the same coin silently — ask the user.
- **Never** trade illiquid coins below `MIN_VOLUME_USD`.

### Risk disclaimer

**This Skill, its parameters, and all related documentation are provided solely for educational research and technical reference purposes. They do not constitute investment advice, trading guidance, or financial recommendations.**

1. **High risk.** Perpetual futures are leveraged. Liquidation is possible on rapid price moves. You may lose 100% of posted margin.
2. **Signals are not certainties.** Otto AI's alpha feed (trending, KOL sentiment, funding rates, filtered news) reflects aggregated real-time behavior. It has delays, noise, and blind spots, and cannot predict price direction.
3. **Parameters are reference-only.** Defaults in `scripts/config.py` are sized for a general user and are not tuned for your risk tolerance. Adjust before going live.
4. **Market conditions change.** Backtested or in-sample signal edges may degrade. Monitor your outcomes.
5. **Hyperliquid-specific risks.** Insurance fund haircuts, funding-rate regime shifts, keeper failures, L1 outages. Hyperliquid is geofenced — users must confirm eligibility in their jurisdiction.
6. **No profit guarantee.** Past performance ≠ future results.
7. **Regulatory risk.** Leveraged crypto derivatives are restricted or prohibited in many jurisdictions. User is solely responsible for compliance, taxes, and KYC where required.
8. **Assumption of responsibility.** Strategy is provided AS-IS. Authors, Otto AI, OKX, and affiliates are not liable for trading losses.

**Recommendation.** Start with `DRY_RUN = True` (default). Place at least 10 paper trades and review the logs before switching to live. Use the smallest allowed `SIZE_USD` for your first live trade.

### No claim of OKX endorsement

This Skill is authored by Otto AI, a community developer submitting to the OKX Plugin Store Developer Challenge. It is not an OKX-endorsed product. "Otto" and "Otto AI" are the author's branding; "Hyperliquid" and "OKX Onchain OS" are referenced as the execution venues, not as affiliated entities.

---

## Config reference

See `scripts/config.py`. Key defaults (tune before going live):

- `DRY_RUN = True`
- `DEFAULT_SIZE_USD = 25`
- `MAX_POSITION_PCT_EQUITY = 0.10`
- `MAX_LEVERAGE_TRENDING = 5`
- `MAX_LEVERAGE_KOL = 3`
- `MAX_LEVERAGE_FUNDING = 10`
- `MAX_LEVERAGE_ABSOLUTE = 20`
- `SL_PCT = 0.02` (2%)
- `TP_PCT = 0.04` (4%, 2:1 RR)
- `MIN_SCORE = 0.65`
- `MIN_CONFIDENCE_KOL = 0.70`
- `MIN_VOLUME_USD = 10_000_000`
- `SESSION_MAX_DRAWDOWN_PCT = 0.15`

---

## Onchain OS Integration

This Skill runs inside Onchain OS Agentic Wallet. All Hyperliquid interactions go through `hyperliquid-plugin`, which uses the TEE-backed signing context of the user's connected wallet. No private keys leave Onchain OS.

Otto Alpha Sniper does not provision its own wallet. It orchestrates.

---

## Links

- Otto AI: https://useotto.xyz
- Otto X (the x402 API layer this Skill shares a data moat with): https://xlayer.ottoai.services
- Docs: https://docs.useotto.xyz
- Source: https://github.com/useOttoAI/plugin-store (fork of okx/plugin-store with Otto Skills under `skills/otto-*`)
