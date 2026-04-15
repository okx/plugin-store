
# aave-v3-plugin -- Skill Summary

## Overview

This plugin provides complete access to Aave V3's decentralized lending protocol, allowing users to supply crypto assets to earn yield, borrow against collateral, monitor position health, and manage lending positions across Ethereum, Polygon, Arbitrum, and Base networks. All operations use simulation mode by default and require explicit user confirmation before execution, with built-in health factor monitoring to prevent liquidation risk.

## Usage

Connect your wallet with `onchainos wallet login`, then use commands like `aave-v3-plugin supply --asset USDC --amount 1000` to lend assets or `aave-v3-plugin borrow --asset WETH --amount 0.5` to borrow. Add `--confirm` flag after simulation to execute transactions.

## Commands

| Command | Purpose |
|---------|---------|
| `supply` | Deposit assets to earn interest |
| `withdraw` | Redeem supplied assets |
| `borrow` | Borrow against collateral |
| `repay` | Pay back borrowed debt |
| `health-factor` | Check position safety |
| `positions` | View current positions |
| `reserves` | List market rates and APYs |
| `set-collateral` | Enable/disable asset as collateral |
| `set-emode` | Configure efficiency mode |
| `claim-rewards` | Collect AAVE token rewards |

## Triggers

Activate this skill when users want to lend crypto assets for yield, borrow against their holdings, check their Aave positions, or monitor DeFi lending rates and health factors. Common trigger phrases include "supply to aave", "borrow from aave", "aave health factor", and "my aave positions".
