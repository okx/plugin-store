
# morpho-plugin -- Skill Summary

## Overview
The morpho-plugin enables AI agents to interact with Morpho, a permissionless lending protocol with over $5B TVL operating on Ethereum and Base. It provides comprehensive DeFi lending capabilities through both Morpho Blue isolated markets and MetaMorpho curated vaults, supporting supply, borrowing, collateral management, position monitoring, and reward claiming with built-in safety features and health factor tracking.

## Usage
Install via `npx skills add okx/plugin-store-community --skill morpho` and ensure onchainos wallet is connected. All write operations use a two-step safety flow: first run without `--confirm` to preview transactions, then re-run with `--confirm` after user approval.

## Commands
| Command | Description |
|---------|-------------|
| `morpho positions` | View all Morpho Blue and MetaMorpho positions with health factors |
| `morpho markets [--asset SYMBOL]` | List Morpho Blue markets with APYs and utilization |
| `morpho vaults [--asset SYMBOL]` | Browse MetaMorpho vaults with curators and yields |
| `morpho supply --vault ADDR --asset SYMBOL --amount N` | Deposit to MetaMorpho vault |
| `morpho withdraw --vault ADDR --asset SYMBOL --amount N` | Withdraw from MetaMorpho vault |
| `morpho borrow --market-id HEX --amount N` | Borrow from Morpho Blue market |
| `morpho repay --market-id HEX --amount N` | Repay Morpho Blue debt |
| `morpho supply-collateral --market-id HEX --amount N` | Supply collateral to Blue market |
| `morpho withdraw-collateral --market-id HEX --amount N` | Withdraw collateral from Blue market |
| `morpho claim-rewards` | Claim Merkl rewards |

All commands support `--chain` (1 for Ethereum, 8453 for Base), `--dry-run`, and write operations require `--confirm`.

## Triggers
Activate this skill when users mention Morpho lending activities: "supply to morpho", "borrow from morpho", "morpho health factor", "metamorpho vaults", "morpho interest rates", "claim morpho rewards", or "my morpho positions". Also trigger for DeFi lending needs on Ethereum or Base when users want to earn yield or need borrowing with collateral.
