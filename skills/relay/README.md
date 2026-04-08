# Relay Plugin

Cross-chain bridge and swap plugin for onchainos, powered by [Relay](https://relay.link).

## Features

- Bridge ETH and tokens across 74+ EVM chains
- Get live quotes with fee breakdown
- Monitor bridge transaction status
- List supported chains and currencies

## Supported Chains

74+ EVM chains including Ethereum, Base, Arbitrum, Optimism, Polygon, BNB Chain, Avalanche, Linea, zkSync, Scroll, Blast, Mode, Mantle, and more.

## Usage

```bash
# List supported chains
relay chains

# List tokens on Base
relay currencies --chain 8453

# Get a bridge quote (Base → Ethereum)
relay quote --from-chain 8453 --to-chain 1 --token ETH --amount 0.001

# Bridge ETH (dry run first)
relay bridge --from-chain 8453 --to-chain 1 --token ETH --amount 0.001 --dry-run
relay bridge --from-chain 8453 --to-chain 1 --token ETH --amount 0.001

# Check bridge status
relay status --request-id 0x...
```

## Building

```bash
cargo build --release
```

## API

Uses `https://api.relay.link` REST API — no authentication required.
