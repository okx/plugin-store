# pm-perp-momentum

Polymarket-to-Hyperliquid momentum strategy for Onchain OS, built for high-confidence KPI attribution, verifiable economic proof, and advanced risk-controlled execution.

Repository: [https://github.com/midasbal/pm-perp-momentum](https://github.com/midasbal/pm-perp-momentum)

## Why this Skill exists

`pm-perp-momentum` is designed for strategy-category competition environments where leaderboard performance and compliance quality both matter. The engine translates Polymarket probability momentum into Hyperliquid perpetual execution with:

- strict plugin-mediated execution paths,
- deterministic replay and tamper-evident proof data,
- advanced risk controls suitable for autonomous strategy operation.

## Environment Warning (Read first)

**CRITICAL: KPI attribution and leaderboard points are ONLY earned when this Skill is executed within the official Onchain OS / Agentic Wallet environment.**

If the strategy runs outside the official environment, orders may execute but challenge KPI attribution may not be credited.

## Compliance Architecture

### Built on top of Basic Skills

This Skill is a Strategy-layer orchestrator that depends on:

- `polymarket-plugin`
- `hyperliquid-plugin`

Declared in `plugin.yaml` via `dependent_plugin`, with explicit versions.

### Execution chain compliance

- `dry-run` and `live` strategy paths call Basic Skill plugins through the centralized subprocess wrapper.
- Trade write operations are tagged with `--strategy-id pm-perp-momentum` automatically.
- No direct chain signing logic is implemented in this repository.

### Utility command scope

For operator performance and discovery UX:

- `resolve-market` and `suggest-markets` use direct API reads.

For challenge-critical strategy execution:

- all trading and signal execution paths remain plugin-mediated.

## Core Features

- Momentum signal detection with hysteresis + dwell-time filters.
- Entry slicing engine for volume and transaction profile tuning.
- Multi-target take-profit ladder with partial reduce-only exits.
- Hyperliquid mark-price stop-loss monitoring loop.
- Daily loss kill-switch and consecutive-loss circuit breaker.
- Run fingerprinting and hash-chained event log.
- Exportable proof artifacts (`json` and `csv`).
- Deterministic replay with mark-aware stop-loss behavior.
- Proof integrity verification (`verify-proof`).

## Economic Proof Model

The strategy generates auditable evidence in SQLite during execution:

- `runs`: run metadata + deterministic config fingerprint,
- `positions`: entries, exits, partial closes, realized PnL context,
- `events`: hash-chained event log,
- `replay_ticks`: signal and mark tick streams for deterministic reconstruction.

Proof workflow:

1. Run strategy (`dry-run` or `live`).
2. Export artifacts with `export-proof`.
3. Validate integrity with `verify-proof`.
4. Reconstruct decision sequence with `replay`.

## Advanced Risk Management

- **Live safety gate**: `--confirm-live` + exact risk acknowledgment phrase required.
- **Mark-price stop-loss**: independent of Polymarket signal availability.
- **Daily loss limit**: blocks new entries after configured threshold.
- **Consecutive-loss breaker**: blocks new entries after loss streak threshold.
- **Opt-in state recovery**: `--resume-open-position` must be explicitly enabled.

## Command Surface

### Build and validate

```bash
npm ci
npm run lint
npm run typecheck
npm test
npm run build
```

### Discover markets

```bash
node dist/src/index.js suggest-markets --limit 10 --min-liquidity 5000 --json
```

### Resolve token IDs

```bash
node dist/src/index.js resolve-market --input "<slug-or-url>" --json
```

### Dry-run simulation

```bash
node dist/src/index.js dry-run \
  --pm-market "<TOKEN_ID>" \
  --signal-side YES \
  --entry-threshold 70 \
  --exit-threshold 60 \
  --dwell-seconds 20 \
  --perp ETH \
  --side LONG \
  --notional-usd 2000 \
  --leverage 5 \
  --stop-loss-pct 8 \
  --leaderboard-mode volume-max \
  --state-db ./fixtures/pm-dryrun.sqlite
```

### Guarded live execution

```bash
node dist/src/index.js live \
  --pm-market "<TOKEN_ID>" \
  --signal-side YES \
  --entry-threshold 70 \
  --exit-threshold 60 \
  --dwell-seconds 20 \
  --perp ETH \
  --side LONG \
  --notional-usd 2000 \
  --leverage 5 \
  --stop-loss-pct 8 \
  --leaderboard-mode volume-max \
  --daily-loss-limit-usd 1500 \
  --consecutive-loss-limit 5 \
  --state-db ./fixtures/pm-live.sqlite \
  --resume-open-position \
  --confirm-live \
  --risk-ack "I acknowledge leveraged trading risk and accept full responsibility."
```

### Export proof artifacts

```bash
node dist/src/index.js export-proof \
  --state-db ./fixtures/pm-live.sqlite \
  --format json \
  --output ./fixtures/proof-live.json
```

### Replay decisions

```bash
node dist/src/index.js replay --state-db ./fixtures/pm-live.sqlite --json
```

### Verify proof integrity

```bash
node dist/src/index.js verify-proof --state-db ./fixtures/pm-live.sqlite --json
```

## Submission-Grade Release Workflow

Use the release pipeline to produce a clean deliverable tree:

```bash
npm run submission:bundle
```

This command:

- builds a clean source bundle at `release/pm-perp-momentum`,
- runs strict submission checks in that bundle,
- ensures the release folder is the canonical final deliverable source.

## Troubleshooting

- `Live mode requires --confirm-live`  
  Add `--confirm-live`.

- `Live mode requires --risk-ack`  
  Use the exact required phrase.

- `Unsupported argument: --...`  
  Use `--help` and pass only supported flags.

- `Failed to fetch Polymarket market data via polymarket-plugin`  
  Verify plugin installation and supported read command surface.

- `Failed to fetch Hyperliquid mark price`  
  Verify plugin availability and retry.

- `Proof verification failed`  
  Re-export and verify untampered database artifacts.

## License

MIT
