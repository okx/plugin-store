
# aerodrome-amm -- Skill Summary

## Overview
This skill enables trading and liquidity management on Aerodrome Finance's classic AMM pools on Base network. It supports both volatile pools (constant-product formula for tokens like WETH/USDC) and stable pools (low-slippage curves for stablecoins like USDC/DAI), with ERC-20 LP tokens for liquidity positions and AERO reward claiming from gauges.

## Usage
Install the aerodrome-amm binary and ensure onchainos CLI is configured with your wallet. All write operations require explicit user confirmation via the `--confirm` flag after previewing transaction details.

## Commands
- `aerodrome-amm quote` - Get swap quotes across volatile and stable pools
- `aerodrome-amm swap` - Execute token swaps with slippage protection
- `aerodrome-amm pools` - Query pool addresses, reserves, and deployment status
- `aerodrome-amm positions` - View LP token balances and pool shares
- `aerodrome-amm add-liquidity` - Add liquidity to earn fees and rewards
- `aerodrome-amm remove-liquidity` - Withdraw liquidity by burning LP tokens
- `aerodrome-amm claim-rewards` - Claim accumulated AERO emissions from gauges

## Triggers
Activate this skill when users want to trade tokens on Base network via Aerodrome Finance, manage liquidity positions in classic AMM pools, or claim AERO rewards. Use for volatile pairs like WETH/USDC or stable pairs like USDC/DAI.
