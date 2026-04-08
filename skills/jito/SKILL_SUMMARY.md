
# jito -- Skill Summary

## Overview
Jito is a MEV-enhanced liquid staking protocol on Solana that allows users to stake SOL and receive JitoSOL tokens in return. The protocol earns both traditional validator staking rewards and additional MEV (Maximum Extractable Value) rewards through Jito's block engine, providing enhanced yield compared to standard staking. Users can stake SOL instantly, track their positions, query current rates and APY, and unstake through a delayed process that creates stake accounts unlocking after the current epoch.

## Usage
Use `jito rates` to check current exchange rates and APY, `jito positions` to view your JitoSOL holdings, and `jito stake/unstake` with amounts to manage your staking positions. All write operations require user confirmation and support dry-run previews.

## Commands
| Command | Description | Example |
|---------|-------------|---------|
| `rates` | Query current SOL↔JitoSOL exchange rate and APY | `jito rates --chain 501` |
| `positions` | View JitoSOL balance and SOL equivalent value | `jito positions --chain 501` |
| `stake` | Deposit SOL to receive JitoSOL | `jito stake --amount 0.001 --chain 501 --dry-run` |
| `unstake` | Redeem JitoSOL for stake account (delayed unlock) | `jito unstake --amount 0.005 --chain 501 --dry-run` |

## Triggers
Activate this skill when users want to stake SOL on Jito, check JitoSOL rates or yields, view their liquid staking positions, or unstake JitoSOL back to SOL. Also trigger for general queries about Jito staking APY, MEV rewards, or liquid staking on Solana.
