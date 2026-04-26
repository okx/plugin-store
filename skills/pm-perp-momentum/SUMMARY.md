# pm-perp-momentum

## Overview

`pm-perp-momentum` is a strategy plugin that converts Polymarket probability momentum into Hyperliquid perpetual futures execution, with guarded live controls, deterministic replay support, and exportable proof artifacts.

Core operations:

- Resolve and select tradable Polymarket token IDs for signal tracking.
- Execute momentum-triggered Hyperliquid perp entries with configurable slicing.
- Protect positions with Hyperliquid mark-price stop-loss and risk breakers.
- Export run evidence and deterministic replay output for auditability.

Tags: `strategy` `polymarket` `hyperliquid` `momentum` `trading`

## Prerequisites

- Node.js `>= 20.10.0` and npm installed.
- `polymarket-plugin` and `hyperliquid-plugin` installed and available on PATH.
- Strategy runtime built with `npm run build`.
- A valid Polymarket token ID and target Hyperliquid perp market selected.
- For live execution, explicit safety confirmation and risk acknowledgment are required.

## Quick Start

1. **Build and validate the runtime**: Run `npm ci`, then run `npm run build` to compile the TypeScript command surface.
2. **Discover and resolve a market token**: Use `suggest-markets` to discover candidates, then run `resolve-market` to get token IDs and choose one for `--pm-market`.
3. **Run dry simulation first**: Execute `dry-run` with your thresholds and risk controls to validate behavior without live order placement.
4. **Run guarded live mode**: Execute `live` only after dry-run validation, with `--confirm-live` and the exact risk acknowledgment string.
5. **Generate and verify proof artifacts**: Use `export-proof` to produce JSON or CSV evidence, run `replay` for deterministic decision reconstruction, and run `verify-proof` to validate fingerprint/hash-chain integrity.
