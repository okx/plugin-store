# camelot-v3-onchainos

## Overview

Camelot DEX is the native Arbitrum AMM built on Algebra V3, offering concentrated liquidity swaps with dynamic fees. This plugin wraps the Camelot Algebra V3 contracts to build `pending_sign` swap transactions on Arbitrum, routed through Onchain OS for TEE-based signing.

Core operations:
- List supported chains (Arbitrum One)
- Get token information by address
- Get swap quotes with price impact and fee breakdown
- Build swap transactions (returns `pending_sign` envelope)

Tags: `swap` `amm` `camelot` `algebra` `arbitrum`

## Prerequisites

- No IP restrictions
- Supported chain: Arbitrum One (42161)
- Supported tokens: any ERC-20 token with a Camelot V3 liquidity pool
- onchainos CLI installed and authenticated
- Node.js ≥ 18 and `tsx` for running the skill (`npm install` in skill directory)

## Quick Start

1. **Install dependencies**: `cd ~/.agents/skills/camelot-v3-onchainos && npm install`
2. **Get token info**: Ask Claude "get token info for GRAIL on Camelot"
3. **Get a quote**: Ask Claude "swap 10 USDC to ETH on Camelot"
4. **Execute a swap**: Claude will call `buildSwap`, return a `pending_sign` envelope, and route it through `onchainos wallet contract-call` for signing and broadcast
