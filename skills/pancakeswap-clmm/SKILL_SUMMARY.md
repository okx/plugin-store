# pancakeswap-clmm — Skill Summary

## Overview
This skill enables staking PancakeSwap V3 CLMM LP NFTs into MasterChefV3 to earn CAKE rewards, harvesting rewards, collecting swap fees, and viewing positions across BSC, Ethereum, Base, and Arbitrum.

## Commands

| Command | Description |
|---------|-------------|
| `farm` | Stake a V3 LP NFT into MasterChefV3 to earn CAKE rewards |
| `unfarm` | Withdraw a staked LP NFT from MasterChefV3 |
| `harvest` | Claim pending CAKE rewards from a farmed position |
| `collect-fees` | Collect accumulated swap fees from a V3 LP position |
| `pending-rewards` | View pending CAKE rewards for a position (read-only) |
| `farm-pools` | List active MasterChefV3 farming pools (read-only) |
| `positions` | View all V3 LP positions for a wallet (read-only) |

## Triggers
Activate when users want to farm CAKE, stake LP NFTs, harvest rewards, collect PancakeSwap fees, or view V3 positions. Also triggered by: "stake LP NFT", "farm CAKE", "harvest CAKE rewards", "collect fees", "unfarm position", "PancakeSwap V3 farming".
