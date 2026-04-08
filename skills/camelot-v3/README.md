# camelot-v3

Camelot V3 DEX plugin for onchainos. Camelot is Arbitrum's native concentrated liquidity DEX built on the Algebra V1 protocol.

## Features

- **quote** — Get price quotes for token swaps (no gas)
- **swap** — Execute token swaps on Camelot V3
- **positions** — List your LP positions
- **add-liquidity** — Add concentrated liquidity
- **remove-liquidity** — Remove liquidity from positions

## Chain

Arbitrum (chain ID: 42161)

## Usage

```bash
camelot-v3 quote --token-in WETH --token-out USDT --amount-in 1000000000000000 --chain 42161
camelot-v3 swap --token-in USDT --token-out WETH --amount-in 1000000 --chain 42161
camelot-v3 positions --chain 42161
camelot-v3 add-liquidity --token0 USDT --token1 WETH --amount0 1000000 --amount1 0 --chain 42161
camelot-v3 remove-liquidity --token-id 12345 --liquidity 1000000000 --chain 42161
```

## Building

```bash
cargo build --release
```
