
# pancakeswap-clmm -- Skill Summary

## Overview
This skill manages PancakeSwap V3 CLMM (Concentrated Liquidity Market Maker) farming operations, allowing users to stake their V3 LP NFTs into MasterChefV3 contracts to earn CAKE token rewards. It provides comprehensive farming functionality including staking, unstaking, reward harvesting, and fee collection across BSC, Ethereum, Base, and Arbitrum networks.

## Usage
Use this skill after creating V3 LP positions with the pancakeswap plugin. Stake LP NFTs to start earning CAKE rewards, harvest rewards periodically, and collect swap fees as needed.

## Commands
| Command | Description |
|---------|-------------|
| `farm --token-id <ID>` | Stake LP NFT into MasterChefV3 to earn CAKE |
| `unfarm --token-id <ID>` | Withdraw LP NFT and harvest rewards |
| `harvest --token-id <ID>` | Claim CAKE rewards without unstaking |
| `collect-fees --token-id <ID>` | Collect swap fees from unstaked positions |
| `pending-rewards --token-id <ID>` | View pending CAKE rewards |
| `positions` | View all LP positions (with optional staked IDs) |
| `farm-pools` | List active MasterChefV3 farming pools |

## Triggers
Activate this skill when users want to farm CAKE rewards from V3 LP positions, harvest farming rewards, collect swap fees, or manage staked NFT positions on PancakeSwap. Use trigger phrases like "stake LP NFT", "farm CAKE", "harvest rewards", or "collect fees".
