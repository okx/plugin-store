
# cian -- Skill Summary

## Overview
The CIAN plugin enables interaction with CIAN Yield Layer, a multi-chain ERC4626 yield aggregator that provides automated delta-neutral strategies for ETH and BTC derivative assets. Users can deposit LST tokens like stETH, rsETH, and pumpBTC into yield-generating vaults across Ethereum, Arbitrum, BSC, and Mantle networks, receiving yield-bearing receipt tokens in return. The plugin supports vault discovery, position tracking, deposits, and withdrawal requests with real-time APY and TVL data.

## Usage
Install the plugin and use natural language commands like "list CIAN vaults on Ethereum" or "deposit 1 stETH into CIAN vault". All on-chain transactions require explicit user confirmation before execution due to the immediate broadcast nature of the underlying binary operations.

## Commands
| Command | Description |
|---------|-------------|
| `cian list-vaults` | List available vaults with APY and TVL data |
| `cian get-positions` | Query user positions, shares, and earnings |
| `cian deposit` | Deposit tokens into vaults (requires approval + deposit) |
| `cian request-withdraw` | Request withdrawal of vault shares (queued processing) |

## Triggers
Activate this skill when users mention CIAN-related operations like "CIAN deposit", "CIAN vault", "CIAN yield", "list CIAN vaults", "my CIAN position", or specific token combinations like "CIAN stETH", "CIAN pumpBTC", or chain-specific requests like "CIAN Arbitrum vault".
