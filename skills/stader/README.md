# Stader ETHx Liquid Staking Plugin

Stake ETH with [Stader](https://www.staderlabs.com/eth/) to receive ETHx, a liquid staking token on Ethereum.

## Commands

| Command | Description |
|---------|-------------|
| `stader rates` | View ETH→ETHx exchange rate and protocol stats |
| `stader positions` | View ETHx balance and pending withdrawals |
| `stader stake --amount <wei>` | Stake ETH to receive ETHx |
| `stader unstake --amount <wei>` | Request ETHx withdrawal (takes ~3-10 days) |
| `stader claim --request-id <id>` | Claim finalized ETH withdrawal |

## Chain

Ethereum Mainnet (chain ID 1) only.

## Key Contracts

- StaderStakePoolsManager: `0xcf5EA1b38380f6aF39068375516Daf40Ed70D299`
- UserWithdrawManager: `0x9F0491B32DBce587c50c4C43AB303b06478193A7`
- ETHx Token: `0xA35b1B31Ce002FBF2058D22F30f95D405200A15b`
