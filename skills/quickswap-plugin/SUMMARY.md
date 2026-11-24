# QuickSwap DEX — Plugin Summary

**Version**: 0.1.2
**Chain**: Polygon (chain ID 137)
**Protocol**: QuickSwap V3 (Algebra Protocol CLMM — no fee tiers)

## Overview

QuickSwap V3 is a concentrated liquidity DEX on Polygon built on Algebra Protocol. Unlike Uniswap V3, it uses dynamic fees with no explicit fee tier parameter. This plugin enables token swaps via the SwapRouter, price quotes via the on-chain Quoter, and pool discovery via the QuickSwap subgraph.

Key contracts:
- SwapRouter: `0xf5b509bb0909a69b1c207e495f687a596c168e12`
- Quoter: `0xa15F0D7377B2A0C0c10db057f641beD21028FC89`
- Factory: `0x411b0fAcC3489691f28ad58c47006AF5E3Ab3A28`

## Prerequisites

- onchainos CLI installed and configured with a Polygon wallet
- `quickswap-plugin` binary installed (see SKILL.md Install section)
- Sufficient MATIC/POL for gas, plus the token you want to swap

## Quick Start

```bash
# 1. Verify installation
quickswap-plugin --version

# 2. Get a price quote (read-only, no wallet needed)
quickswap-plugin quote --token-in MATIC --token-out USDC --amount 10

# 3. Preview a swap (dry-run, no transaction)
quickswap-plugin swap --token-in MATIC --token-out USDC --amount 10

# 4. Execute the swap on-chain
quickswap-plugin swap --token-in MATIC --token-out USDC --amount 10 --confirm

# 5. Browse top pools
quickswap-plugin pools --limit 10
```
