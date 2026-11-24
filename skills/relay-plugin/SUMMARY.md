# Relay — Plugin Summary

**Version**: 0.1.1

## Overview

Relay Protocol is an intent-based cross-chain bridge that delivers funds across chains in seconds. Users specify the source chain, destination chain, token, and amount — Relay handles the rest via an off-chain solver network.

Core operations:
- Bridge ETH and ERC-20 tokens (USDC, USDT, DAI) across 70+ chains
- Get quotes with real-time fee and output estimates
- Track transfer status by request ID

Tags: `defi` `bridge` `cross-chain` `multi-chain`

## Prerequisites

- No IP/region restrictions
- Supported chains: Ethereum (1), Arbitrum (42161), Base (8453), Optimism (10), Polygon (137), and 65+ more — run `relay chains` for the full list
- Supported tokens: ETH (native), USDC, USDT, DAI; any ERC-20 by address
- onchainos CLI installed and authenticated with an active EVM wallet on the source chain
- Sufficient balance on the source chain for the bridge amount plus gas

## Quick Start

1. **Check chains**: `relay chains` — view all 70+ supported chains
2. **Get a quote**: `relay quote --from-chain 1 --to-chain 42161 --token ETH --amount 0.01`
3. **Preview bridge**: `relay bridge --from-chain 1 --to-chain 42161 --token ETH --amount 0.01`
4. **Execute**: Add `--confirm` to the bridge command to send the transaction
5. **Track**: `relay status --request-id <id>` — check if funds arrived
