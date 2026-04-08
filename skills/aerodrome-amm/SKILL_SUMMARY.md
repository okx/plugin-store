
# aerodrome-amm -- Skill Summary

## Overview
This skill enables interaction with Aerodrome Finance's classic AMM (Automated Market Maker) on Base chain, supporting both volatile and stable pool types. It provides comprehensive DeFi functionality including token swapping, liquidity provision, position management, and reward claiming through Aerodrome's gauge system using standard ERC-20 LP tokens.

## Usage
Install the aerodrome-amm binary and ensure onchainos CLI is configured with your wallet. All write operations require user confirmation with the `--confirm` flag after previewing transaction details.

## Commands
| Command | Description |
|---------|-------------|
| `quote` | Get swap quote between tokens (read-only) |
| `swap` | Execute token swap with slippage protection |
| `pools` | Query pool addresses and reserve information |
| `positions` | View LP token balances and pool shares |
| `add-liquidity` | Add liquidity to pools and receive LP tokens |
| `remove-liquidity` | Remove liquidity by burning LP tokens |
| `claim-rewards` | Claim AERO token rewards from gauges |

## Triggers
Activate this skill when users want to trade tokens, provide liquidity, or manage positions specifically on Aerodrome's classic AMM pools on Base chain. Use when users mention Aerodrome, volatile/stable pools, LP tokens, or AERO rewards.
