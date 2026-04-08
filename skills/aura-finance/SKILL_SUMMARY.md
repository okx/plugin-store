
# aura-finance -- Skill Summary

## Overview
This plugin integrates with Aura Finance, the Balancer equivalent of Convex Finance, allowing users to deposit Balancer Pool Tokens (BPT) into Aura pools to earn boosted BAL and AURA rewards. It supports the complete Aura workflow including staking BPT, claiming rewards, locking AURA as vlAURA for governance participation, and managing expired locks. All write operations require user confirmation and use a secure two-transaction flow for token approvals.

## Usage
First use `get-pools` to find available Aura pools, then `get-position` to check your current balances. For staking, you must already hold BPT tokens from Balancer before using the `deposit` command.

## Commands
| Command | Description |
|---------|-------------|
| `get-pools` | List Aura-supported Balancer pools with TVL data |
| `get-position` | Check vlAURA balance, liquid tokens, and pool positions |
| `deposit` | Deposit BPT into Aura pools (requires BPT balance) |
| `withdraw` | Withdraw staked BPT from Aura pools |
| `claim-rewards` | Claim pending BAL and AURA rewards |
| `lock-aura` | Lock AURA as vlAURA for 16 weeks (irreversible) |
| `unlock-aura` | Process expired vlAURA locks to retrieve AURA |

## Triggers
Activate this skill when users want to stake Balancer LP tokens for boosted rewards, manage Aura Finance positions, or work with vlAURA governance tokens. Trigger phrases include "aura finance deposit", "claim aura rewards", "lock aura", "vlAURA", and "balancer boosted yield".
