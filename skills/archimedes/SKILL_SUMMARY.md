
# archimedes -- Skill Summary

## Overview
Archimedes Finance V2 is a protected yield protocol that wraps Convex and Aura LP strategies inside ERC4626 vaults with offchain auto-protection monitoring. Users can deposit WETH or crvFRAX to receive vault shares that appreciate over time, with the ability to withdraw or redeem at any point. The protocol uses a non-standard ERC4626 implementation that includes slippage protection via a minimumReceive parameter.

## Usage
Ensure onchainos wallet is installed and logged in, then use commands like `archimedes vaults` to list available vaults, `archimedes positions` to check balances, and `archimedes deposit/withdraw/redeem` for transactions. All write operations require user confirmation and support dry-run previews.

## Commands
| Command | Description |
|---------|-------------|
| `vaults` | List all Archimedes V2 vault addresses with TVL |
| `positions` | Show wallet's share balance across all vaults |
| `deposit` | Deposit underlying assets into a vault |
| `withdraw` | Withdraw specific amount of underlying assets |
| `redeem` | Redeem vault shares for underlying assets |

## Triggers
Activate this skill when users want to interact with Archimedes Finance yield vaults, including depositing WETH or crvFRAX for yield farming, checking vault positions, or withdrawing from protected yield strategies on Ethereum mainnet.
