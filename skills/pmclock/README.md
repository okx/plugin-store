# pmclock — Plugin Store Skill

5-minute crypto Up/Down arbitrage scanner for Polymarket, powered by a
**deterministic TypeScript binary**. Compares CEX-implied GBM
probability to live Polymarket YES/NO ask, deploys risk-bounded maker
buys via `polymarket-plugin` only when net edge ≥ 1.5%.

## Install

```bash
npx skills add okx/plugin-store --skill pmclock
```

## At a glance

```
User: "Scan BTC 5min arb, $50 budget, balanced"
  │
  ▼
pmclock Skill (natural-language parsing)
  │
  ├─► api.binance.com /ticker/price  →  live BTC spot
  ├─► api.binance.com /klines (1m, 60) →  realized daily vol from past hour
  ├─► polymarket-plugin list-5m        →  next 5 BTC Up/Down markets
  ├─► polymarket-plugin get-market     →  per-market YES/NO ask + book depth
  └─► pmclock binary                   →  deterministic GBM probability +
                                          Kelly-fractional sizing per market
  │
  ▼
Dry-run plan:
  • Markets considered : 5
  • Decisions          : 1  (4 rejected: 3 below-edge, 1 low-liquidity)
  • Decision detail    : NO $7.50 @ 0.610  edge 38.90%
                         (CEX 99.9% above 78,000 vs PM 61.0%)
  • Total deployed     : $7.50 of $50 budget
  • planHash           : a3f82b...   (same inputs → same plan, always)
  │
  ▼
User: "place it"
  │
  ▼
polymarket-plugin buy --market-id <id> --outcome no --amount 7.5 \
   --price 0.610 --post-only --order-type GTC \
   --expires <closesAt-30s> --strategy-id pmclock --confirm
  │
  ▼
Maker-side rebate fill → wait until 5-min resolution → polymarket-plugin redeem
```

## What pmclock actually does for a user

| Pain point | Without pmclock | With pmclock |
|---|---|---|
| 5-min Up/Down markets often misprice vs. CEX during the first 1-2 minutes after open | Manual eyeballing impossible at this speed; HFT bots own top-of-book | LLM-Skill targets the long tail (thin $50-200 books) where small operators can capture edge HFT ignores |
| LLM math drifts (different model = different decisions) | Same prompt, different sizes, no audit trail | Compiled TypeScript binary, byte-identical decisions, stable `planHash` SHA-256 |
| Manual decision-making is too slow for 5-min windows | Trader sees market, computes prob, sizes — already past close | Binary returns full `PmclockPlan` in < 50ms; agent's ~30s loop captures most of the open-window edge |
| Statistical errors compound | "Felt right" sizing → over-bet on weak edges, lose long-run | Kelly-fractional sizing (1/4 Kelly at balanced), saturates at 5% edge → stable Sharpe |
| Can't tell why a bot skipped a market | Black-box "no opp" message | 7 structured rejection reasons (`expired`, `too_far_out`, `low_liquidity`, `edge_below_threshold`, `yes_no_ask_invalid`, `insufficient_budget`, `unknown_direction`) + per-market detail |
| Single bad CEX price could nuke account | Bot YOLOs full position | Hard caps in source: $200/scan, $20/market, 1.5% min edge — refused at plan stage |

## Hard differentiators vs other Polymarket strategies

1. **Deterministic binary, not LLM math** — every probability,
   every Kelly fraction, every rejection decision is in compiled
   TypeScript. Same inputs → byte-identical outputs. Other AI
   strategy Skills leave the math to the LLM and drift each call.

2. **Targets long-tail 5-min books, not top-of-book HFT** — pmclock
   is intentionally slower (30-60s polling) than Rust-on-VPS
   bots. Its niche: the first 1-2 minutes after a 5-min market
   opens, where book depth is thin and HFT bots don't bother with
   $50-200 books. Small AI-Skill operators can sustain 1-2% edges
   that don't scale to multi-million-dollar HFT operations.

3. **Structured rejection reasons** — every skipped market has a
   reason code. Users can audit "why didn't pmclock buy this one?"
   instantly. Most bots are black-box "no opportunity" — pmclock
   shows the math.

4. **Auditable Kelly + ramp** — sizing schedule is fully spelled
   out in `src/arb.ts:sizeFromEdge`. No magic constants, no ML
   model — pure formula, fully reproducible, fully open-source.

## Safety (enforced in binary, not just documented)

| Cap / check | Value |
|---|---|
| Max total notional per scan | **$200** |
| Max per individual market | **$20** |
| Min combined book liquidity (YES + NO) | **$50** |
| Min edge to fire a decision | **1.5%** |
| Absolute min if user lowers threshold | **0.5%** |
| Skip markets too close to resolution | **< 30s** to close |
| Skip markets too far out | **> 600s** (10 min) to close |
| Conservative Kelly fraction | **0.15** |
| Balanced Kelly fraction | **0.25** |
| Aggressive Kelly fraction | **0.33** |
| Edge saturation point (full per-market sizing) | **5%** |

**Behavioral:** dry-run by default; explicit user confirmation
before any order placement; `--post-only` on every limit (no
accidental taker fills); `--expires` 30s before resolution (no
forgotten orders).

**No private keys** handled by pmclock — all signing via Agentic
Wallet TEE through `polymarket-plugin`. The binary itself makes
**zero** network calls. The Skill declares two read-only public
endpoints in `plugin.yaml`: `api.binance.com` (CEX spot + klines)
and `gamma-api.polymarket.com` (market data). Every write
operation flows through `polymarket-plugin`.

## When to use

**Good fit:**
- USDC.e on Polygon, $20+ to start
- Comfortable with automated decision-making on a CEX-vs-PM model
- In a non-restricted region (NOT US / France / Singapore)
- Wants reproducible, auditable arbitrage decisions
- OK with statistical edge over many scans rather than guaranteed wins

**Bad fit:**
- Restricted region — Polymarket TOS blocks (pre-flight refuses)
- Wants to bet on event direction (sports / elections) — pmclock is
  exclusively crypto Up/Down
- Wants to compete in HFT — use Rust + VPS, not an LLM Skill
- Expects guaranteed returns — this is statistical arbitrage,
  individual scans can lose; edge is in long-run repetition

## Requires

- `onchainos` CLI + unlocked Agentic Wallet
- **`polymarket-plugin` ≥ 0.4.10**:
  `npx skills add okx/plugin-store --skill polymarket-plugin`
- ≥ $20 USDC.e on Polygon (and a small POL balance for one-time
  proxy setup if using POLY_PROXY mode)
- Public internet access to `api.binance.com` and
  `gamma-api.polymarket.com`

## Source

TypeScript → compiled to JS, distributed via `bun install -g` by the
Plugin Store CI from
[dddd86971-cloud/pmclock](https://github.com/dddd86971-cloud/pmclock)
at a pinned commit (see `plugin.yaml` `build.source_commit`).

Self-test suite: `npm test` — 18 invariants covering standard normal
CDF correctness (vs known values), GBM probability bounds (ATM /
ITM / OTM behaviors), Kelly-fractional sizing monotonicity, scan
determinism + planHash stability, all 7 rejection-reason paths,
direction handling (`above` vs `below`), greedy budget enforcement,
cap clamping, structural input validation.

## Strategy-id Attribution

Every write operation carries `--strategy-id pmclock`. `polymarket-plugin`
calls `onchainos wallet report-plugin-info` after each successful order
so OKX's Plugin Store Season 1 leaderboard can attribute trades to this
Skill.

## License

MIT — see `LICENSE`.

See `SKILL.md` for the full command reference, error handling, and
Security Notices. See `SUMMARY.md` for the 30-second user pitch.
