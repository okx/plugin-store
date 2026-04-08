
# morpho-base -- Skill Summary

## Overview
This skill enables interaction with Morpho V1 lending protocol on the Base network, providing access to both Morpho Blue isolated lending markets and MetaMorpho ERC-4626 vaults. Users can supply assets to earn yield, borrow against collateral, manage positions, and claim rewards through natural language commands. All operations include safety checks with health factor monitoring and mandatory dry-run simulations before execution.

## Usage
Install the plugin and connect your wallet with `onchainos wallet login`, then use natural language like "supply USDC to morpho base vault" or "borrow from morpho base market". All write operations require explicit confirmation after dry-run preview.

## Commands
| Command | Description |
|---------|-------------|
| `supply` | Supply assets to MetaMorpho vaults (ERC-4626 deposit) |
| `withdraw` | Withdraw assets from MetaMorpho vaults |
| `borrow` | Borrow from Morpho Blue markets |
| `repay` | Repay debt to Morpho Blue markets |
| `supply-collateral` | Supply collateral to Morpho Blue markets |
| `positions` | View wallet's active positions and health factors |
| `markets` | List Morpho Blue markets with rates and TVL |
| `vaults` | List MetaMorpho vaults with APY and TVL |
| `claim-rewards` | Claim Merkl rewards |

## Triggers
Activate when users mention lending, borrowing, or yield activities on Morpho Base, including phrases like "supply to morpho base", "borrow from morpho", "morpho base positions", or Chinese equivalents like "在Morpho Base存款". Also trigger for DeFi portfolio management and reward claiming on Base network.
