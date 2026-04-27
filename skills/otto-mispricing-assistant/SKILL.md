---
name: otto-mispricing-assistant
description: >
  Otto Mispricing Assistant v0.1 — Scans near-resolution Polymarket markets where the implied
  probability (mid-price of YES/NO outcome tokens) diverges from Otto AI's real-time news,
  KOL-sentiment, and funding-rate signals. Presents ranked candidates to the user. Each trade
  requires explicit user confirmation — no batch execution, no autonomous trading.
  Trigger when the user mentions Polymarket mispricing, prediction-market edge, late-stage
  Polymarket, find undervalued prediction markets, find mispriced prediction markets,
  Polymarket news edge, Polymarket odds vs Otto, or wants to find Polymarket markets where
  odds don't match the news.
version: "0.1.0"
author: "Otto AI"
updated: 2026-04-24
tags:
  - prediction-markets
  - polymarket
  - polygon
  - onchainos
  - trading-strategy
  - mispricing
  - news-signals
---

# Otto Mispricing Assistant — Skill Protocol

> Real on-chain trading of prediction-market outcome shares. Use paper mode (`DRY_RUN = True`) until you understand the strategy. Capital loss is possible — a "mispriced" market can still resolve against you.

---

## Overview

Otto Mispricing Assistant is a **scanner, not a bot.** It cross-references active Polymarket markets against Otto AI's live news-flash, KOL-sentiment, and funding-extreme feeds, computes a divergence score between implied probability and Otto's estimate, and presents a ranked list of candidate markets to the user. **Every single trade requires an explicit user confirmation.** There is no batch mode, no autonomous execution, and no "fire-and-forget" flow.

- **Primary use case.** User asks "find me a Polymarket market where the odds don't match the news" or similar.
- **Decision rule.** Rank by `|otto_estimate - implied_prob|` × `signal_confidence` × `liquidity_score`. Only surface markets above `MIN_EDGE_PCT`.
- **Execution.** All Polymarket actions flow through the Polymarket Basic Skill (`polymarket-plugin`). No raw CLOB interaction.
- **Fund sizing.** Capped per-trade by `MAX_TRADE_SIZE_USD` and per-session by `MAX_SESSION_BUDGET_USD`.
- **Never auto-trades.** Each candidate is presented; the user picks one and confirms.

---

## Pre-flight Checks

### 1. Install onchainos CLI (≥ 2.0.0-beta)

```bash
onchainos --version 2>/dev/null || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh
npx skills add okx/onchainos-skills --yes --global
npx skills add okx/plugin-store --skill plugin-store --yes --global
```

### 2. Install the Polymarket Basic Skill (required)

```bash
npx skills add okx/plugin-store --skill polymarket-plugin --yes --global
```

### 3. Set up Polymarket trading (proxy or EOA)

```bash
polymarket-plugin quickstart              # tells you what to do
polymarket-plugin check-access            # verify your region is not restricted
polymarket-plugin setup-proxy             # one-time, recommended (gasless mode)
polymarket-plugin deposit --amount 50     # fund the proxy wallet with USDC.e
```

### 4. Verify the Otto signal feed is reachable

```bash
curl -fsS "https://signals.useotto.xyz/v1/news-flash?since_minutes=60" | jq .updated_at
```

If this fails, abort and report "Otto signal feed unreachable" — the Skill does not fabricate signals.

---

## Commands

When the user fires a Mispricing Assistant intent, execute this protocol in order.

### Step 1 — Readiness check

```bash
polymarket-plugin quickstart
```

- **When to use**: always, first thing.
- **Output**: JSON status (`ready` / `needs_setup` / `needs_deposit` / `region_blocked`).
- **If not ready**: run `polymarket-plugin setup-proxy` or `polymarket-plugin deposit` as indicated. If `region_blocked`, abort with a clear message.

### Step 2 — Pull Otto's directional signals

```bash
curl -fsS "https://signals.useotto.xyz/v1/news-flash?since_minutes={WINDOW_MIN}&severity_min=3" | jq .
curl -fsS "https://signals.useotto.xyz/v1/kol-sentiment" | jq .
curl -fsS "https://signals.useotto.xyz/v1/funding-extremes?limit=5" | jq .
```

- **When to use**: after readiness.
- **Output**: three JSON payloads combined into an "Otto view" per asset/topic. See `../SIGNAL_FEED_CONTRACT.md` for schemas.
- **Fallback**: if any endpoint is `status: degraded`, use the remaining signals but mark confidence penalty in the rank.

### Step 3 — List candidate Polymarket markets

Pull active markets in topics the Otto signals actually cover. Default to the `crypto`, `macro`, and `elections` categories unless user narrows.

```bash
polymarket-plugin list-markets --limit 40 --category crypto
polymarket-plugin list-markets --limit 40 --category elections
polymarket-plugin list-markets --limit 40 --keyword "fed"
```

- **When to use**: after Step 2.
- **Output**: JSON list of markets with `market_id`, `question`, `end_date`, `yes_price`, `no_price`, `volume_usd`, `liquidity_usd`.
- **Filter**:
  - `end_date` within `RESOLUTION_WINDOW_DAYS` (default 14). Skip deep-out markets — noise dominates.
  - `liquidity_usd >= MIN_LIQUIDITY_USD` (default $5,000).
  - `volume_usd >= MIN_VOLUME_USD` (default $1,000 last 24h).

### Step 4 — Compute mispricing score per market

For each candidate market, compute:

```
otto_estimate   = Otto's probability estimate based on matched news-flash + kol + funding signals
implied_prob    = mid-price of the YES token (from polymarket-plugin get-market)
edge_pct        = otto_estimate - implied_prob           # signed, positive = YES undervalued
abs_edge        = abs(edge_pct)
signal_conf     = max(news_severity_conf, kol_conf, funding_conf) for matched signals
liquidity_score = min(liquidity_usd / 10000, 1.0)

mispricing_score = abs_edge × signal_conf × liquidity_score
```

- Keep only markets with `abs_edge >= MIN_EDGE_PCT` (default 0.08 = 8 percentage points).
- Sort descending by `mispricing_score`.
- Cap the list at `TOP_N_CANDIDATES` (default 5).
- If no candidate meets the threshold, return: "No Polymarket mispricings strong enough right now — try again later or a different category."

### Step 5 — Present the ranked candidates

Present a compact table to the user:

```
Top mispricing candidates (Otto edge ≥ 8%):

1. "Will Fed cut rates by July 2026?"
   Resolves: 2026-07-31  |  YES @ $0.42  |  Otto estimate: 0.58  |  Edge: +16pp (YES)
   Signal: Fed minutes dovish (severity 4, 3h ago), KOL bias 72% dovish
   Liquidity: $18,400  |  Volume (24h): $3,200

2. "Will BTC > $100K by May 2026?"
   Resolves: 2026-05-31  |  YES @ $0.31  |  Otto estimate: 0.44  |  Edge: +13pp (YES)
   Signal: funding crowded-short (fade long), KOL bullish, no negative news flashes
   Liquidity: $12,100  |  Volume (24h): $1,900

...

Which market would you like to trade? Reply with the number (1-5), or "none" to exit.
```

### Step 6 — Quote the selected market

Once the user picks `N`:

```bash
polymarket-plugin get-market --market-id <ID> --verbose
```

- **When to use**: confirm order book depth + refreshed mid-price before sizing.
- **Output**: JSON with full order book, best bid/ask, current mid-price, implied probability.

Recompute `edge_pct` with the freshly-fetched mid — prices move quickly near resolution. If `edge_pct` dropped below `MIN_EDGE_PCT`, warn the user and ask whether to proceed anyway.

### Step 7 — Confirm with the user (MANDATORY)

```
I'm about to buy {side} on "{question}".
• Market: {market_id}
• Side: {YES / NO}
• Amount: ${TRADE_SIZE_USD} USDC.e (Otto max: ${MAX_TRADE_SIZE_USD})
• Price: {price} ({implied_prob:.0%} implied)
• Otto estimate: {otto_estimate:.0%}  (edge {edge_pct:+.0%})
• Session budget remaining: ${SESSION_BUDGET_REMAINING}
• Dry run? {DRY_RUN}

Reply "confirm" to execute live, "paper" for dry-only, or "cancel".
```

**Do NOT proceed to Step 8 without the user's explicit "confirm".**

### Step 8 — Place the order

```bash
polymarket-plugin buy --market-id <ID> --outcome <yes|no> --amount <USD> --price <PRICE> --strategy-id otto-mispricing-assistant --order-type GTC
```

- **Size**: `TRADE_SIZE_USD`, always ≤ `MAX_TRADE_SIZE_USD`, and total across the session ≤ `MAX_SESSION_BUDGET_USD`.
- **Dry-run**: if `DRY_RUN = True`, omit `--price` and pass `--dry-run` flag (see `polymarket-plugin buy --help`).
- **Strategy attribution**: always include `--strategy-id otto-mispricing-assistant`.
- **Order type**: GTC for patient limit orders at Otto's suggested price; user may override to FOK if they want immediate fill.

### Step 9 — Report back

```
✓ Otto Mispricing Assistant
  {side} ${amount} on "{question}" @ ${price}
  Market: {market_id}  Resolves: {end_date}
  Otto edge at entry: {edge_pct:+.0%}  Confidence: {signal_conf:.2f}
  Order id: {order_id}
  Session budget used: ${used} / ${MAX_SESSION_BUDGET_USD}
```

Always remind the user:

> "This is a single-market prediction-market trade. Prices can move against you even when news seems supportive. Polymarket markets resolve on a specific source — verify the resolution criteria before betting large."

### Configuration commands

Tunable parameters live in `scripts/config.py`:

```bash
grep -E "^[A-Z_]+ " scripts/config.py      # view defaults
```

### Optional autonomous poller (scanner only — never auto-buys)

```bash
python3 scripts/bot.py --interval 900 --notify-only    # print candidates every 15m, never trades
```

- **When to use**: user explicitly wants a background watcher that surfaces mispricings but never places orders.
- **Output**: JSONL log of scanned candidates; user pastes a market ID back into the reactive flow to trade.

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| `polymarket-plugin: command not found` | Basic Skill not installed | Run Pre-flight step 2. |
| `status: region_blocked` | Polymarket geofencing | Abort with clear message; do NOT suggest workarounds. |
| `status: needs_setup` | No proxy wallet yet | `polymarket-plugin setup-proxy`. |
| `status: needs_deposit` | Proxy wallet empty | `polymarket-plugin deposit --amount N`. |
| Signal feed 5xx / 503 | Otto backend transient failure | Retry ONCE after 3s. If still failing, abort. |
| Signal feed 429 | Rate-limited | Back off 30s, retry ONCE. |
| All candidate markets below `MIN_EDGE_PCT` | No edge available right now | Tell user "no Polymarket mispricings strong enough right now". |
| User picks a number outside 1..TOP_N | Typo or mis-selection | Ask again; do not default to any market. |
| `edge_pct` drops below threshold between Step 5 and Step 6 | Price moved | Warn user with updated numbers; ask whether to proceed. |
| `polymarket-plugin buy` returns insufficient liquidity | Order book thinner than list-markets showed | Offer to split the order or lower size; do NOT auto-split. |
| `MAX_SESSION_BUDGET_USD` reached | Session cap hit | Refuse further trades this session. |
| User explicitly aborts at Step 7 | Declined | Do not place. Do not retry. |

---

## Security Notices

**Risk level: `advanced`**. This Skill moves user funds on a public prediction market. Loss of capital is possible, including total loss of any given position.

### Safeguards enforced by this Skill

- **Explicit per-trade confirmation.** No batch mode, no autonomous orders. Every single trade requires the user to type "confirm" at Step 7.
- **Dry-run default.** `DRY_RUN = True` in `scripts/config.py`. Live orders require both `DRY_RUN = False` AND per-trade `--confirm`.
- **Per-trade cap.** `MAX_TRADE_SIZE_USD = 50` — no single trade above this regardless of user ask.
- **Per-session cap.** `MAX_SESSION_BUDGET_USD = 200` — cumulative trades refused once the budget is spent.
- **Resolution-window filter.** Markets with `end_date > RESOLUTION_WINDOW_DAYS` are filtered out — Otto's edge estimates degrade on long-dated markets.
- **Liquidity + volume floors.** Markets below `MIN_LIQUIDITY_USD` / `MIN_VOLUME_USD` are filtered — thin markets have poor exit paths.
- **Edge threshold.** Markets below `MIN_EDGE_PCT = 0.08` are filtered — smaller edges are dominated by noise + fees.
- **Price-refresh check.** Step 6 re-quotes between Step 5 ranking and Step 7 confirmation. User sees updated edge before confirming.
- **No key handling.** All signing flows through `polymarket-plugin`'s onchainos TEE-backed wallet. This Skill never touches private keys.
- **No credentials in source.** No API keys, tokens, or secrets committed.
- **Declared network surface.** Only `signals.useotto.xyz` + `gamma-api.polymarket.com` (via `polymarket-plugin`). Listed in `api_calls`.

### Things this Skill will NOT do

- **Never** place a trade without explicit user "confirm" at Step 7.
- **Never** execute batch orders — one market per user intent.
- **Never** bypass the Polymarket Basic Skill. All CLOB interactions MUST flow through `polymarket-plugin`.
- **Never** fabricate signals. If Otto's feed is unreachable or `status: degraded`, the Skill abstains.
- **Never** exceed `MAX_TRADE_SIZE_USD` or `MAX_SESSION_BUDGET_USD` — not even with user override.
- **Never** suggest workarounds for region blocks.
- **Never** re-submit a failed order automatically.

### Risk disclaimer

**This Skill is provided solely for educational research and technical reference purposes. It does not constitute investment advice, trading guidance, betting advice, or financial recommendations.**

1. **Prediction markets can resolve against "obvious" signals.** Mispricings can persist or widen before they close. A positive expected-value trade can still lose.
2. **Resolution risk.** Polymarket markets resolve on specific oracle sources. Ambiguous outcomes may resolve differently than news coverage suggests.
3. **Liquidity risk.** Exit liquidity can disappear near resolution. You may have to hold a position to settlement whether you want to or not.
4. **Otto signals are aggregated, not predictive.** Otto's news + KOL + funding feeds reflect past and present behavior. They are NOT forecasts.
5. **Parameters are reference-only.** Defaults in `scripts/config.py` are not tuned for your risk tolerance.
6. **Regulatory risk.** Prediction markets are restricted or prohibited in many jurisdictions. Polymarket enforces geofencing. User is solely responsible for compliance, taxes, KYC.
7. **No profit guarantee.** Past mispricing detection accuracy ≠ future accuracy.
8. **Assumption of responsibility.** Strategy is AS-IS. Authors, Otto AI, OKX, Polymarket, and affiliates are not liable for losses.

### No claim of OKX or Polymarket endorsement

This Skill is authored by Otto AI, a community developer submitting to the OKX Plugin Store Developer Challenge. It is not endorsed by OKX or Polymarket. "OKX Onchain OS" and "Polymarket" are referenced as execution venues, not as affiliated entities.

---

## Config reference

See `scripts/config.py`. Key defaults:

- `DRY_RUN = True`
- `MAX_TRADE_SIZE_USD = 50`
- `MAX_SESSION_BUDGET_USD = 200`
- `DEFAULT_TRADE_SIZE_USD = 10`
- `MIN_EDGE_PCT = 0.08` (8 percentage points)
- `RESOLUTION_WINDOW_DAYS = 14`
- `MIN_LIQUIDITY_USD = 5_000`
- `MIN_VOLUME_USD = 1_000`
- `TOP_N_CANDIDATES = 5`
- `WINDOW_MIN_NEWS = 360` (6h lookback for news flashes)
- `PRICE_STALENESS_SEC = 60`

---

## Onchain OS Integration

This Skill runs inside Onchain OS Agentic Wallet. All Polymarket interactions go through `polymarket-plugin`, which uses the TEE-backed signing context of the user's connected wallet. No private keys leave Onchain OS.

Otto Mispricing Assistant does not provision its own wallet. It scans, ranks, and proposes — the user executes through the Basic Skill after explicit confirmation.

---

## Links

- Otto AI: https://useotto.xyz
- Signal feed contract: [../SIGNAL_FEED_CONTRACT.md](../SIGNAL_FEED_CONTRACT.md)
- Polymarket Basic Skill: https://github.com/okx/plugin-store/tree/main/skills/polymarket-plugin
- Docs: https://docs.useotto.xyz
- Source: https://github.com/useOttoAI/plugin-store
