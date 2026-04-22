# rust-eth-price

## 1. Overview

A Rust CLI tool that queries real-time ETH price data from the OKX public API.

Core operations:

- Query current ETH price in USD
- Display 24-hour price change percentage
- Fetch trading volume data

Tags: `ethereum` `price` `rust` `analytics`

## 2. Prerequisites

- No IP restrictions
- Supported chain: Ethereum
- Supported token: ETH
- onchainos CLI installed and authenticated

## 3. Quick Start

1. **Check ETH price**: Run `rust-eth-price` to get the current ETH/USDT price with 24h change.
2. **Alternative**: Use `onchainos market price --token ETH --chain ethereum`.
3. **Output**: JSON with `price`, `change_24h`, `volume_24h`, and `timestamp` fields.
