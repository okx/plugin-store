
# sanctum-validator-lst -- Skill Summary

## Overview
This skill enables staking SOL into validator liquid staking tokens (LSTs) and swapping between different LSTs using the Sanctum Router on Solana. It supports 8 major validator LSTs including jitoSOL, bSOL, jupSOL, and others, providing functionality to stake SOL directly into validator pools, swap between LST tokens, view real-time market data, and manage LST positions.

## Usage
Use commands like `sanctum-validator-lst stake --lst jitoSOL --amount 0.002` to stake SOL or `sanctum-validator-lst swap-lst --from jitoSOL --to bSOL --amount 0.005` to swap between LSTs. All staking and swapping operations require user confirmation for security.

## Commands
| Command | Description |
|---------|-------------|
| `list-lsts` | List all tracked validator LSTs with APY, TVL, and SOL value |
| `get-quote` | Quote a swap between two LSTs with optional slippage |
| `swap-lst` | Swap between two validator LSTs via Sanctum Router |
| `stake` | Stake SOL into a validator LST pool (primarily jitoSOL) |
| `get-position` | Show your validator LST holdings and SOL equivalent value |

## Triggers
Activate when users want to stake SOL into validator liquid staking tokens, swap between different LSTs, or manage their validator staking positions on Solana. Use trigger phrases like "sanctum stake", "stake sol jitosol", or "swap lst sanctum".
