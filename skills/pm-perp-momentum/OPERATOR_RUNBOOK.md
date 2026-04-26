# pm-perp-momentum Operator Runbook

This runbook is the operational manual for running `pm-perp-momentum` end-to-end with safe defaults, reproducible evidence, and judge-ready outputs.

---

## 1) Scope and Goals

This guide covers the full execution pipeline:

1. Build and health checks
2. Market discovery (`suggest-markets`)
3. Market token resolution (`resolve-market`)
4. Dry-run simulation (`dry-run`)
5. Guarded live execution (`live`)
6. Proof export (`export-proof`)
7. Deterministic replay (`replay`)
8. Proof integrity verification (`verify-proof`)

---

## 2) Prerequisites

- Node.js `>= 20.10.0`
- NPM installed
- `polymarket-plugin` and `hyperliquid-plugin` installed and available on PATH for live execution

Recommended working directory:

```bash
cd "/Users/midasbal/Desktop/okx-plugin/pm-perp-momentum"
```

---

## 3) Build and Validation

Run this once before any operation:

```bash
npm install
npm run lint
npm run typecheck
npm test
npm run build
npm run submission:check
```

Expected outcome:
- All checks pass
- CLI entrypoint available at `dist/src/index.js`

Before creating the final submission archive, run:

```bash
npm run submission:check:strict
```

For deterministic clean packaging, run:

```bash
npm run submission:bundle
```

This generates a strict-checked release tree at `release/pm-perp-momentum`.

---

## 4) Stage A — Discover High-Quality Active Markets

Use this to get candidate markets with token IDs ranked by activity:

```bash
node dist/src/index.js suggest-markets --limit 10 --min-liquidity 5000 --json
```

Optional category filter:

```bash
node dist/src/index.js suggest-markets --limit 10 --min-liquidity 5000 --category politics --json
```

Operator action:
- Pick one market from output
- Copy `tokenIds[0]` for `--pm-market`

---

## 5) Stage B — Resolve Slug or URL into Token IDs

If you start from a Polymarket URL or slug:

```bash
node dist/src/index.js resolve-market --input "https://polymarket.com/event/<event-slug>" --json
```

Or direct slug:

```bash
node dist/src/index.js resolve-market --input "<event-slug>" --json
```

Operator action:
- Select one token ID from the returned `tokenIds` array
- Use it as `--pm-market` in dry-run/live commands

---

## 6) Stage C — Dry-Run Simulation (No Real Orders)

Dry-run records deterministic replay ticks and simulated decisions without placing real chain-executing orders.

### Baseline dry-run

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
  --entry-slices 3 \
  --take-profit-targets 1.25,2.5,4 \
  --state-db ./fixtures/pm-dryrun.sqlite \
  --run-label "dryrun-volume-baseline"
```

### High transaction profile dry-run

```bash
node dist/src/index.js dry-run \
  --pm-market "<TOKEN_ID>" \
  --signal-side YES \
  --entry-threshold 65 \
  --exit-threshold 58 \
  --dwell-seconds 8 \
  --perp ETH \
  --side LONG \
  --notional-usd 1200 \
  --leverage 4 \
  --stop-loss-pct 7 \
  --leaderboard-mode tx-max \
  --entry-slices 8 \
  --take-profit-targets 0.4,0.8,1.2,1.6 \
  --state-db ./fixtures/pm-dryrun-tx.sqlite \
  --run-label "dryrun-tx-profile"
```

Stop with `Ctrl+C`.

---

## 7) Stage D — Guarded Live Execution

Live mode is hard-gated and requires both:

- `--confirm-live`
- exact `--risk-ack` phrase:
  - `I acknowledge leveraged trading risk and accept full responsibility.`

### Guarded live command (copy-paste template)

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
  --entry-slices 3 \
  --take-profit-targets 1.25,2.5,4 \
  --daily-loss-limit-usd 1500 \
  --consecutive-loss-limit 5 \
  --state-db ./fixtures/pm-live.sqlite \
  --run-label "live-volume-profile" \
  --confirm-live \
  --risk-ack "I acknowledge leveraged trading risk and accept full responsibility."
```

### Safety behavior in live mode

- Entry is blocked if daily kill-switch is hit
- Entry is blocked if consecutive-loss circuit breaker is hit
- Stop-loss is continuously monitored
- Multi-target take-profit executes reduce-only partial exits
- Runtime persists tamper-evident event chain and tick history

---

## 8) Stage E — Export Economic Proof Pack

Export latest run as JSON:

```bash
node dist/src/index.js export-proof \
  --state-db ./fixtures/pm-live.sqlite \
  --format json \
  --output ./fixtures/proof-live.json
```

Export specific run as CSV bundle:

```bash
node dist/src/index.js export-proof \
  --state-db ./fixtures/pm-live.sqlite \
  --run-id "<RUN_ID>" \
  --format csv \
  --output ./fixtures/proof-live-csv
```

CSV bundle includes:
- `run.json`
- `positions.csv`
- `events.csv`
- `ticks.csv`

Notes:
- `--format json` expects `--output` to be a file path.
- `--format csv` expects `--output` to be a directory path.

---

## 9) Stage F — Deterministic Replay

Replay latest run:

```bash
node dist/src/index.js replay \
  --state-db ./fixtures/pm-live.sqlite \
  --json
```

Replay a specific run at faster speed:

```bash
node dist/src/index.js replay \
  --state-db ./fixtures/pm-live.sqlite \
  --run-id "<RUN_ID>" \
  --replay-speed 5 \
  --json
```

Replay validates reproducibility of signal and action sequence from stored ticks.

---

## 10) Stage G — Verify Proof Integrity

```bash
node dist/src/index.js verify-proof \
  --state-db ./fixtures/pm-live.sqlite \
  --json
```

Expected outcome:
- `fingerprintValid: true`
- `hashChainValid: true`
- `issues: []`

---

## 11) Leaderboard Preset Guidance

Use `--leaderboard-mode` to load tuned defaults:

- `volume-max`: larger risk budget and larger target spacing
- `tx-max`: more slices and tighter target cadence
- `address-max`: safer profile for broader user adoption

Manual flags override preset defaults:
- `--entry-slices`
- `--take-profit-targets`
- `--daily-loss-limit-usd`
- `--consecutive-loss-limit`

---

## 12) Rapid Command Cheatsheet

### Build

```bash
npm run build
```

### Discover

```bash
node dist/src/index.js suggest-markets --limit 10 --min-liquidity 5000 --json
```

### Resolve

```bash
node dist/src/index.js resolve-market --input "<slug-or-url>" --json
```

### Dry-run

```bash
node dist/src/index.js dry-run --pm-market "<TOKEN_ID>" --signal-side YES --entry-threshold 70 --exit-threshold 60 --dwell-seconds 20 --perp ETH --side LONG --notional-usd 2000 --leverage 5 --stop-loss-pct 8 --leaderboard-mode volume-max --state-db ./fixtures/pm-dryrun.sqlite
```

### Live

```bash
node dist/src/index.js live --pm-market "<TOKEN_ID>" --signal-side YES --entry-threshold 70 --exit-threshold 60 --dwell-seconds 20 --perp ETH --side LONG --notional-usd 2000 --leverage 5 --stop-loss-pct 8 --leaderboard-mode volume-max --state-db ./fixtures/pm-live.sqlite --confirm-live --risk-ack "I acknowledge leveraged trading risk and accept full responsibility."
```

### Export Proof

```bash
node dist/src/index.js export-proof --state-db ./fixtures/pm-live.sqlite --format json --output ./fixtures/proof-live.json
```

### Replay

```bash
node dist/src/index.js replay --state-db ./fixtures/pm-live.sqlite --json
```

### Verify Proof

```bash
node dist/src/index.js verify-proof --state-db ./fixtures/pm-live.sqlite --json
```

---

## 13) Operational Notes

- Keep one runtime process per state DB file unless intentionally testing concurrency.
- Always run dry-run first when changing thresholds, slices, or target ladders.
- Use explicit `--run-label` values to make proof exports easier to audit.
- For judge demos, prepare:
  1. one `dry-run` DB,
  2. one `live` DB,
  3. exported JSON proof,
  4. replay JSON output.

---

## 14) Troubleshooting

- **Live mode rejected immediately**
  - Ensure both `--confirm-live` and exact `--risk-ack` phrase are present.

- **No replay ticks available**
  - Do not pass `--no-record-ticks` during runtime runs.

- **No active markets returned**
  - Lower `--min-liquidity` and increase `--limit`.

- **Entry not opening in live mode**
  - Check if kill-switch or consecutive-loss breaker is active in event logs.

- **Proof export run not found**
  - Verify `--run-id` exists, or omit it to export the latest run.

