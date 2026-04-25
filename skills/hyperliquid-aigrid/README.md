# hyperliquid-aigrid — Plugin Store Skill

Natural-language perpetual grids on Hyperliquid, powered by a **deterministic
TypeScript engine** with **funding-aware asymmetric sizing**, **concentrated-
liquidity weighting**, and a **built-in backtest engine**.

## Install

```bash
npx skills add okx/plugin-store --skill hyperliquid-aigrid
```

## At a glance

```
User: "BTC 90k-95k range, $500 at 2x"
  │
  ▼
hyperliquid-aigrid Skill  (natural-language parsing)
  │
  ├─► hyperliquid-plugin   →  fetch mark / funding / 1h candles
  └─► hyperliquid-aigrid binary       →  deterministic grid math
  │
  ▼
Dry-run Plan:
  • 23 tick-aligned rungs (concentrated: ~$20/rung at mark, ~$3/rung at edges)
  • funding −12% annualized  →  +6.6% notional tilt toward buy side
  • stop-loss $87,500        →  max loss 6.9% / $34.50
  • backtest past 7d         →  +$7.57 realized / 15 fills / 0 drawdown
  • planHash a3f82b...       (same input → same plan, always)
  │
  ▼
User: "go live"
  │
  ▼
Limit orders via hyperliquid-plugin  --strategy-id hyperliquid-aigrid
Stop-loss trigger  via hyperliquid-plugin  tpsl  --strategy-id hyperliquid-aigrid
  │
  ▼
Agentic Wallet TEE signing  →  on-chain fills
```

## What hyperliquid-aigrid actually does for a user

| Pain point | Without hyperliquid-aigrid | With hyperliquid-aigrid |
|---|---|---|
| Setting up 20+ tick-aligned rung prices by hand | 30 min of arithmetic, high chance of slipping | One sentence → plan in 2 seconds, math guaranteed correct |
| Capital wasted on edge rungs that rarely fill | Uniform sizing pays ~25% of capital to fill-probability-<1% zones | Concentrated-liquidity weighting, same $300 gets denser near mark |
| Paying funding passively | Symmetric grid eats all funding | At ≥10% annualized funding, tilt up to ±20% toward the profitable side |
| No way to know how this parameter set would have performed | Blind decision | `hyperliquid-aigrid backtest` gives concrete PnL / fills / drawdown / Sharpe on real past candles |
| LLM picks different rung prices across runs | Same prompt, different grid → plan drifts | Compiled binary, stable `planHash` fingerprint, byte-identical across models |
| Accidental $5,000 / 50× blow-up request | Platform accepts it, you liquidate | Binary hard-caps $5k / 10× / 50 rungs / stop-loss ≤30% of notional — refused at plan stage |

## Hard differentiators vs other Hyperliquid grid bots

1. **Deterministic binary, not LLM math** — grid-level prices, stop-loss, and
   sizing run in a compiled TypeScript engine. Same inputs → byte-identical
   outputs. Other AI grid Skills leave it to the LLM and drift each call.

2. **Funding rate as alpha, not drag** — hyperliquid-aigrid reads the live hourly funding
   rate and tilts per-rung notional up to ±20% toward the side that *collects*
   funding. Hyperliquid-specific feature; zero other grid Skill does this.

3. **Concentrated-liquidity rung sizing** — per-rung notional weighted by
   Gaussian fill-probability in log-price space. Center-heavy, edge-light —
   higher expected fill density per dollar deployed than uniform spacing.

4. **Built-in backtest** — `hyperliquid-aigrid backtest` runs the plan bar-by-bar over
   historical candles and returns realized PnL, max DD, fill counts, Sharpe.
   Runs in the same deterministic binary; same input candles → same numbers.

## Safety (enforced in binary, not just documented)

| Cap / check | Value |
|---|---|
| Max notional per grid | $5,000 |
| Max leverage | 10× |
| Max / min grid rungs | 50 / 4 |
| Max loss at range break | 30% of notional (warning enforced) |
| Conservative + >5× leverage | auto-downshift to 5× with warning |
| Mark-vs-candle drift >3% | warning (volatility estimate may be stale) |
| Mark far outside range | REFUSE (range premise is wrong) |
| `rangeLow >= rangeHigh` | hard failure, empty plan + `INPUT:` warning |
| Non-positive `rangeLow` / `tickSize` | hard failure |
| Sum(sizeUsd) invariant | always equals `totalNotionalUsd` |

**Behavioral:** dry-run default on every session; explicit user confirmation
before any order batch; first-session training wheels cap $200 / 2× even if
user asks for more.

**No private keys** handled by hyperliquid-aigrid — all signing via Agentic Wallet TEE
through `hyperliquid-plugin`. The compiled `hyperliquid-aigrid` binary itself makes
**zero** network calls (no `Math.random`, no `Date.now`, no I/O). The Skill
declares only one external endpoint in `plugin.yaml` — `api.hyperliquid.xyz`,
the public read-only info API used to fetch the live funding rate and
historical candles for `hyperliquid-aigrid backtest`. Every write operation still flows
through `hyperliquid-plugin`.

## When to use

**Good fit:**
- Range-bound view on a liquid Hyperliquid perp (BTC / ETH / SOL / etc.)
- Want passive execution via limit orders working at pre-computed levels
- Want predictable, reproducible parameters across agent sessions and models

**Bad fit:**
- Directional thesis ("BTC breaks out to 100k") — grid accumulates a losing
  position against trends
- Market in clear strong trend — Skill refuses with explanation
- Want leverage > 10× or notional > $5k — hard caps, not preferences

## Requires

- `onchainos` CLI + unlocked Agentic Wallet
- **`hyperliquid-plugin` ≥ 0.3.9**:
  `npx skills add okx/plugin-store --skill hyperliquid-plugin`
- ≥ 20 USDC on your Hyperliquid perp account (for minimum collateral)

## Source

TypeScript → compiled to JS, distributed via `bun install -g` by the Plugin
Store CI from [dddd86971-cloud/hyperliquid-aigrid](https://github.com/dddd86971-cloud/hyperliquid-aigrid)
at a pinned commit (see `plugin.yaml` `build.source_commit`).

Self-test suite: `npm test` — 30 invariants covering determinism, cap
enforcement, funding bias noise-floor / saturation / sign, concentrated-
liquidity sizing, backtest determinism, quickstart, optimize ranking, and
input validation across all subcommands.

## Strategy-id Attribution

Every write operation carries `--strategy-id hyperliquid-aigrid` (or `ha-{planHash[:6]}`
for per-plan granularity). `hyperliquid-plugin` calls
`onchainos wallet report-plugin-info` after each successful order so OKX's
Plugin Store Season 1 leaderboard can attribute trades to this Skill.

## License

MIT — see `LICENSE`.

See `SKILL.md` for the full command reference, error handling, and
Security Notices. See `SUMMARY.md` for the 30-second user pitch.
