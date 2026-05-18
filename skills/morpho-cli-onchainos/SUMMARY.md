# morpho-cli-onchainos

## Overview

Morpho Protocol is a decentralized lending infrastructure offering permissionless vaults and isolated lending markets across 10 EVM chains. This plugin wraps the Morpho CLI (`@morpho-org/cli`) to query protocol state and build `pending_sign` deposit/withdraw/borrow/repay transactions, routed through Onchain OS for TEE-based signing.

Core operations:
- Query vaults and markets across Ethereum, Base, Arbitrum, Optimism, Polygon, and more
- Get vault/market details including APY, liquidity, and utilization
- View positions and token balances
- Simulate transactions before executing
- Build deposit, withdraw, supply collateral, borrow, and repay transactions

Tags: `lending` `yield` `morpho` `vault` `ethereum` `base` `arbitrum`

## Prerequisites

- No IP restrictions
- Supported chains: Ethereum (1), Base (8453), Arbitrum (42161), Optimism (10), Polygon (137), Unichain, WorldChain, Katana, HyperEVM, Monad
- Supported tokens: USDC, WETH, WBTC, and any ERC-20 in a Morpho vault/market
- onchainos CLI installed and authenticated
- Node.js ≥ 18 and `tsx` for running the skill (`npm install` in skill directory)
- Morpho CLI installed: `npm install -g @morpho-org/cli`

## Quick Start

1. **Install dependencies**: `cd ~/.agents/skills/morpho-cli-onchainos && npm install`
2. **Query vaults**: Ask Claude "show me Morpho vaults on Base with USDC"
3. **Get market details**: Ask Claude "show me the WETH/USDC Morpho market on Ethereum"
4. **Deposit**: Claude will call `prepareDeposit`, return a `pending_sign` envelope, and route it through `onchainos wallet contract-call` for signing and broadcast
