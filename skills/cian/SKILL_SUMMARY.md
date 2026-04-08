
# cian -- Skill Summary

## Overview
The CIAN Yield Layer plugin enables interaction with a multi-chain ERC4626 yield aggregator that manages over $500M TVL through automated delta-neutral LST/LRT strategies. Users can deposit ETH-derivative or BTC-derivative assets into yield vaults and receive yield-bearing receipt tokens, with support across Ethereum, Arbitrum, BSC, and Mantle networks.

## Usage
Install the plugin and use natural language commands like "list CIAN vaults on Ethereum" or "deposit 1 WETH into CIAN stETH vault". The plugin automatically handles vault discovery, position tracking, deposits, and withdrawal requests with proper transaction confirmation flows.

## Commands
| Command | Description |
|---------|-------------|
| `cian list-vaults` | List all public CIAN vaults with APY and TVL data |
| `cian get-positions` | Query vault positions, shares, and earnings |
| `cian deposit` | Deposit tokens into vaults (requires approval) |
| `cian request-withdraw` | Request withdrawal of shares (queued process) |

## Triggers
Activate when users mention CIAN-specific operations like checking vault APYs, depositing into yield strategies, monitoring LST positions, or requesting withdrawals from delta-neutral vaults. Also triggers for multi-chain yield farming queries involving ETH or BTC derivatives.
