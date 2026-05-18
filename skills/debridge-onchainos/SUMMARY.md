# debridge-onchainos

## Overview

deBridge is a cross-chain interoperability protocol that enables fast, secure bridging via the DLN (Decentralized Limit Order Network). This plugin wraps the deBridge DLN REST API to build `pending_sign` bridge and swap transactions, routed through Onchain OS for TEE-based signing.

Core operations:
- List supported source/destination chains
- Get bridge/swap quotes with fee and slippage breakdown
- Check order fulfillment status by order ID
- Build cross-chain bridge transactions (returns `pending_sign` envelope)
- Build same-chain swap transactions (returns `pending_sign` envelope)

Tags: `bridge` `crosschain` `debridge` `dln` `ethereum` `arbitrum` `solana`

## Prerequisites

- No IP restrictions
- Supported chains: Ethereum (1), Arbitrum (42161), Base (8453), Optimism (10), Polygon (137), BSC (56), Avalanche (43114), Solana
- Supported tokens: ETH, USDC, USDT, WBTC, and other DLN-supported assets
- onchainos CLI installed and authenticated
- Node.js ≥ 18 and `tsx` for running the skill (`npm install` in skill directory)

## Quick Start

1. **Install dependencies**: `cd ~/.agents/skills/debridge-onchainos && npm install`
2. **List supported chains**: Ask Claude "what chains does deBridge support?"
3. **Get a quote**: Ask Claude "bridge 100 USDC from Ethereum to Base via deBridge"
4. **Execute a bridge**: Claude will call `buildBridge`, return a `pending_sign` envelope, and route it through `onchainos wallet contract-call` for signing and broadcast
