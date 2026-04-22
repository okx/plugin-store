# rust-sol-price

## 1. Overview

A Rust CLI tool that queries real-time SOL price data from the OKX public API.

Core operations:

- Query current SOL price in USD
- Display 24-hour price change percentage
- Fetch trading volume data

Tags: `solana` `price` `rust` `analytics`

## 2. Prerequisites

- No IP restrictions
- Supported chain: Solana
- Supported token: SOL
- onchainos CLI installed and authenticated

## 3. Quick Start

1. **Check SOL price**: Run `rust-sol-price` to get the current SOL/USDT price with 24h change.
2. **Alternative**: Use `onchainos market price --token SOL --chain solana`.
3. **Output**: JSON with `price`, `change_24h`, `volume_24h`, and `timestamp` fields.
