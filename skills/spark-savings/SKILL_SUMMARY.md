
# spark-savings -- Skill Summary

## Overview
Spark Savings enables users to earn the Sky Savings Rate (SSR) on USDS and DAI stablecoins through Spark Protocol's savings vaults. The plugin supports deposits and withdrawals across multiple chains, with sUSDS/sDAI vault tokens representing users' earning positions. On Ethereum, it uses direct ERC-4626 vault interactions, while on Layer 2 networks (Base, Arbitrum, Optimism) it operates through the Spark PSM3 contract for seamless USDS ↔ sUSDS swaps.

## Usage
Install the plugin and connect your wallet via `onchainos wallet login`. Use `spark-savings --chain <ID> apy` to check current rates, `balance` to view holdings, and `deposit`/`withdraw` commands with `--dry-run` for safe transaction simulation before execution.

## Commands
| Command | Description |
|---------|-------------|
| `spark-savings --chain <ID> apy` | Check current Sky Savings Rate APY |
| `spark-savings --chain <ID> balance` | View sUSDS/sDAI balances and USDS equivalent |
| `spark-savings --chain <ID> --dry-run deposit --amount <N>` | Simulate USDS deposit to earn savings |
| `spark-savings --chain <ID> --dry-run withdraw --amount <N>` | Simulate sUSDS withdrawal to USDS |
| `spark-savings --chain <ID> --dry-run withdraw --all` | Simulate withdrawal of all savings |
| `spark-savings --chain <ID> markets` | Display market statistics and TVL data |

## Triggers
Activate when users want to earn yield on stablecoins, check savings rates, deposit/withdraw from Spark savings vaults, or inquire about Sky Savings Rate, sUSDS/sDAI APY, or MakerDAO/Sky ecosystem savings products. Also responds to related phrases in multiple languages including Chinese terms for Spark savings operations.
