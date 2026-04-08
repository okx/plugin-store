
# lifi -- Skill Summary

## Overview
The LI.FI skill provides cross-chain bridge and DEX aggregation capabilities across 79+ EVM chains. It routes transactions through the best available bridges and exchanges to optimize for cost, speed, and reliability. The skill supports token discovery, quote generation, transaction execution, and status tracking for cross-chain transfers.

## Usage
Install the plugin and ensure onchainos wallet is connected. Use `get-quote` to preview cross-chain transfers, then execute with the `swap` command using the `--confirm` flag after reviewing transaction details.

## Commands
| Command | Description |
|---------|-------------|
| `get-chains` | List all supported EVM chains |
| `get-tokens` | Show available tokens on specific chains |
| `get-quote` | Get best-route quote for cross-chain transfers |
| `swap` | Execute cross-chain bridge/swap transactions |
| `get-status` | Check transfer status by transaction hash |
| `get-tools` | List available bridges and DEXes |

## Triggers
Activate this skill when users want to bridge tokens between different chains, swap tokens across networks, check cross-chain transfer status, or discover supported chains and tokens. The skill should also be triggered for queries about LI.FI protocol capabilities and multi-chain DeFi operations.
