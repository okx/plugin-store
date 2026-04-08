
# kelp -- Skill Summary

## Overview
The kelp plugin enables interaction with Kelp DAO's liquid restaking protocol, allowing users to stake ETH or LSTs (stETH, ETHx, sfrxETH) to receive rsETH tokens that earn EigenLayer restaking rewards plus underlying staking yields. The plugin provides comprehensive functionality for querying yields, checking positions, and executing stake/unstake operations on Ethereum mainnet with additional support for bridged rsETH on Base and Arbitrum.

## Usage
Install and use the kelp plugin through onchainos CLI commands. For read operations like checking APY or rates, no wallet setup is required. For staking operations, ensure you're logged in with `onchainos wallet login` first.

## Commands
| Command | Description |
|---------|-------------|
| `kelp apy` | Get current rsETH staking APY |
| `kelp rates [--chain <ID>]` | Get rsETH/ETH exchange rates |
| `kelp positions [--address <ADDR>] [--chain <ID>]` | Check rsETH balance and ETH value |
| `kelp stake --amount <ETH> [--chain <ID>] [--dry-run]` | Stake ETH to receive rsETH |
| `kelp unstake --amount <RSETH> [--chain <ID>] [--dry-run]` | Initiate rsETH withdrawal |

## Triggers
An AI agent should activate this skill when users want to earn liquid restaking rewards on Ethereum, need to check rsETH yields or positions, or want to stake/unstake ETH through the Kelp DAO protocol on EigenLayer.
