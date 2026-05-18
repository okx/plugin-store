# across-protocol-onchainos

## Overview

Across Protocol is a cross-chain bridge and swap protocol that uses an intent-based architecture with UMA's optimistic oracle for settlement. This plugin wraps the Across Swap API to build `pending_sign` transactions for crosschain bridging and same-chain swaps, routed through Onchain OS for TEE-based signing.

Core operations:
- List supported source/destination chains and token pairs
- Get bridge/swap quotes with fee breakdown
- Build bridge or swap transactions (returns `pending_sign` envelope)
- Track deposit status by deposit ID

Tags: `bridge` `crosschain` `ethereum` `arbitrum` `base` `optimism` `polygon` `bsc`

## Prerequisites

- No IP restrictions
- Supported chains: Ethereum (1), Arbitrum (42161), Base (8453), Optimism (10), Polygon (137), BSC (56)
- Supported tokens: ETH, WETH, USDC, USDT, WBTC, and other Across-supported assets
- onchainos CLI installed and authenticated
- Node.js ≥ 18 and `tsx` for running the skill (`npm install` in skill directory)

## Quick Start

1. **Install dependencies**: `cd ~/.agents/skills/across-protocol-onchainos && npm install`
2. **List supported chains**: Ask Claude "what chains does Across Protocol support?"
3. **Get a quote**: Ask Claude "bridge 0.01 ETH from Ethereum to Arbitrum via Across"
4. **Execute a bridge**: Claude will call `buildSwap`, return a `pending_sign` envelope, and route it through `onchainos wallet contract-call` for signing and broadcast
