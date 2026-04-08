
# aura-finance -- Skill Summary

## Overview
This skill enables interaction with Aura Finance, a protocol that boosts Balancer yields by depositing Balancer Pool Tokens (BPT) into gauges and distributing enhanced BAL and AURA rewards to depositors. It supports depositing BPT, claiming rewards, locking AURA for governance (vlAURA), and managing positions across Ethereum mainnet pools.

## Usage
Install the plugin and use commands like `aura-finance get-pools` to browse available pools, `aura-finance deposit` to stake BPT, and `aura-finance claim-rewards` to harvest yields. All write operations require user confirmation before executing transactions.

## Commands
| Command | Description |
|---------|-------------|
| `get-pools` | List Aura-supported Balancer pools with TVL data |
| `get-position` | Check vlAURA balance, liquid tokens, and pool positions |
| `deposit` | Deposit BPT into Aura pools (2-tx: approve + deposit) |
| `withdraw` | Withdraw staked BPT from pools |
| `claim-rewards` | Claim pending BAL and AURA rewards |
| `lock-aura` | Lock AURA as vlAURA for 16 weeks (irreversible) |
| `unlock-aura` | Process expired vlAURA locks to retrieve AURA |

## Triggers
Activate when users want to enhance Balancer yields through Aura Finance, including phrases like "aura finance deposit", "aura rewards", "claim aura", "lock aura", "vlAURA", or "balancer boosted yield aura". Also trigger for managing existing Aura positions or exploring available pools.
