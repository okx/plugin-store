---
name: bluefin
description: "Spot and perpetual trading on Bluefin DEX on Sui. Commands: markets, ticker, positions, quote, open-position, close-position. Triggers: bluefin trade, bluefin dex, bluefin perp, trade on bluefin, bluefin spot swap, bluefin perpetuals, open perp bluefin, check bluefin positions"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

## Bluefin DEX Plugin

Trade spot and perpetuals on [Bluefin](https://bluefin.io) — a high-performance decentralized exchange built on the Sui blockchain.

Bluefin offers sub-390ms latency perpetuals and spot trading powered by Sui's Mysticeti consensus.

## Commands

### `bluefin markets`
List all available trading markets with current prices and 24h stats.

```bash
bluefin markets
bluefin markets --json
```

### `bluefin ticker`
Get 24h price statistics for a specific market.

```bash
bluefin ticker --market BTC-PERP
bluefin ticker --market ETH-PERP
bluefin ticker --market SUI-PERP
```

### `bluefin positions`
Show open perpetual positions for a wallet address.

```bash
bluefin positions --wallet <sui-address>
bluefin positions --wallet 0x1234...
```

### `bluefin quote`
Get orderbook quote for a given market and size.

```bash
bluefin quote --market BTC-PERP --amount 0.01
bluefin quote --market ETH-PERP --amount 0.1 --side bid
```

### `bluefin open-position`
Preview or open a perpetual position. Shows a detailed preview by default; use `--confirm` to submit.

```bash
# Preview (safe, no transaction)
bluefin open-position --market ETH-PERP --side long --amount 0.01 --leverage 5

# Submit transaction (requires Sui wallet setup)
bluefin open-position --market ETH-PERP --side long --amount 0.01 --leverage 5 --confirm
```

### `bluefin close-position`
Preview or close an open perpetual position.

```bash
# Preview (safe, no transaction)
bluefin close-position --market ETH-PERP --amount 0.01

# Submit transaction (requires Sui wallet setup)
bluefin close-position --market ETH-PERP --amount 0.01 --confirm
```

## Notes

- **Sui blockchain**: Bluefin v2 runs exclusively on Sui. Write transactions require a Sui wallet and the Sui CLI/SDK.
- **Preview mode**: All write commands (`open-position`, `close-position`) show a transaction preview without `--confirm`. This lets you verify parameters before submitting.
- **Public data**: `markets`, `ticker`, `quote` require no authentication.
- **Position queries**: `positions` requires only a public wallet address (read-only, no signing needed).

## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use.

```bash
# 1. Install onchainos CLI
onchainos --version 2>/dev/null || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh

# 2. Install skills
npx skills@latest add okx/onchainos-skills --yes --global
```
