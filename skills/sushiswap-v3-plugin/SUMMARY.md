# SushiSwap V3 — Plugin Summary

**Version**: 0.1.0

## Overview

SushiSwap V3 is a concentrated liquidity market maker (CLMM) — a Uniswap V3 fork deployed across major EVM chains. Liquidity providers set price ranges and earn trading fees only when the market price is within their range.

Core operations:
- Swap tokens via the Sushi Swap API (auto-routes through optimal pool)
- List pools and their liquidity/price for any token pair
- Open concentrated liquidity positions (mint NFPM NFT)
- Remove liquidity, collect fees, and burn positions

Tags: `defi` `clmm` `swap` `liquidity` `multi-chain`

## Prerequisites

- No IP/geo restrictions
- onchainos CLI installed and authenticated with an active EVM wallet
- Supported chains: Ethereum (1), Arbitrum (42161), Base (8453), Polygon (137), Optimism (10)
- Supported tokens: any ERC-20 with a SushiSwap V3 pool; common symbols (WETH, USDC, USDT, etc.) resolve automatically
- ETH/native token for gas on the target chain

## Quick Start

1. **Check your wallet**: `onchainos wallet addresses --chain 42161`
2. **Get a quote**: `sushiswap-v3 --chain 42161 quote --token-in WETH --token-out USDC --amount-in 0.01`
3. **Preview a swap**: `sushiswap-v3 --chain 42161 swap --token-in WETH --token-out USDC --amount-in 0.01`
4. **Execute**: re-run the swap command with `--confirm` to broadcast
