# maple-onchainos

## Overview

Maple Finance is an institutional fixed-income lending protocol offering ERC-4626 yield vaults (Syrup pools) on Ethereum and Base. This plugin wraps the Maple/Syrup SDK to build `pending_sign` deposit and withdrawal transactions, routed through Onchain OS for TEE-based signing.

Core operations:
- List supported pools with current APY and liquidity
- Get detailed pool information including terms and restrictions
- View existing positions in Maple pools
- Build deposit transactions into Maple/Syrup ERC-4626 pools
- Queue withdrawal requests from pools

Tags: `lending` `yield` `maple` `syrup` `erc-4626` `ethereum` `base`

## Prerequisites

- No IP restrictions (KYC may be required for some institutional pools)
- Supported chains: Ethereum (1), Base (8453)
- Supported tokens: USDC, USDT, and other Maple-accepted stablecoins
- onchainos CLI installed and authenticated
- Node.js ≥ 18 and `tsx` for running the skill (`npm install` in skill directory)

## Quick Start

1. **Install dependencies**: `cd ~/.agents/skills/maple-onchainos && npm install`
2. **List pools**: Ask Claude "show me Maple Finance pools on Ethereum"
3. **Check positions**: Ask Claude "show my Maple Finance positions"
4. **Deposit**: Claude will call `buildDeposit`, return a `pending_sign` envelope, and route it through `onchainos wallet contract-call` for signing and broadcast
