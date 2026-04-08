# Bluefin DEX Plugin

A Rust CLI plugin for [Bluefin](https://bluefin.io) — a high-performance spot and perpetuals DEX built on the Sui blockchain.

## Overview

Bluefin v2 operates exclusively on Sui and leverages Sui's Mysticeti consensus for sub-390ms transaction latency. This plugin provides:

- **Market data**: live prices, 24h stats, orderbook depth
- **Position tracking**: open perpetual positions for any wallet
- **Trade preview**: safe preview of open/close perpetual orders before any signing

> **Note on Sui**: Since `onchainos` CLI does not support Sui, all write operations display a detailed transaction preview and SDK code snippet. Broadcasting requires the Bluefin TypeScript or Python SDK.

## Installation

```bash
# Build from source
cd skills/bluefin
cargo build --release
cp target/release/bluefin ~/.local/bin/
```

## Commands

### `markets` - List trading markets
```bash
bluefin markets
bluefin markets --symbol BTC-PERP
bluefin markets --json
```

### `positions` - Show open positions
```bash
bluefin positions --wallet <sui-address>
```

### `quote` - Get orderbook quote
```bash
bluefin quote --market ETH-PERP --amount 0.1
bluefin quote --market BTC-PERP --amount 0.01 --side bid --depth 10
```

### `open-position` - Open a perpetual position
```bash
# Preview (safe, no transaction)
bluefin open-position --market ETH-PERP --side long --amount 0.01 --leverage 5

# With confirmation (shows SDK code to execute)
bluefin open-position --market ETH-PERP --side long --amount 0.01 --leverage 5 --confirm
```

### `close-position` - Close a perpetual position
```bash
# Preview
bluefin close-position --market ETH-PERP

# Close specific amount
bluefin close-position --market ETH-PERP --amount 0.005 --wallet <address>

# With confirmation
bluefin close-position --market ETH-PERP --confirm
```

## API

This plugin uses the Bluefin REST API at `https://dapi.api.sui-prod.bluefin.io`:

| Endpoint | Description |
|----------|-------------|
| `GET /ticker` | 24h price stats for all markets |
| `GET /ticker?symbol=BTC-PERP` | Stats for specific market |
| `GET /marketData` | Market overview |
| `GET /orderbook?symbol=` | Order book depth |
| `GET /userPosition?userAddress=` | Open positions |
| `GET /account?address=` | Account balance |
| `GET /fundingRate?symbol=` | Current funding rate |

## Chain Info

- **Chain**: Sui mainnet
- **Protocol**: Bluefin Pro (v2)
- **RPC**: https://fullnode.mainnet.sui.io

## License

MIT
