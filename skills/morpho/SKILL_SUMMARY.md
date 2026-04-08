
# morpho -- Skill Summary

## Overview
The Morpho skill enables users to interact with the Morpho lending protocol, which operates on two layers: Morpho Blue (isolated lending markets) and MetaMorpho (ERC-4626 vaults). Users can supply assets to earn yield, borrow against collateral, manage positions with health factor monitoring, and claim rewards. The skill supports both Ethereum mainnet and Base, providing access to over $5B in total value locked across curated markets managed by risk managers like Gauntlet and Steakhouse.

## Usage
Connect your wallet using `onchainos wallet login`, then use commands like `morpho supply`, `morpho borrow`, or `morpho positions`. All write operations require user confirmation and use dry-run previews for safety.

## Commands
| Command | Description |
|---------|-------------|
| `morpho supply` | Deposit assets to MetaMorpho vaults for yield |
| `morpho withdraw` | Withdraw assets from MetaMorpho vaults |
| `morpho borrow` | Borrow assets from Morpho Blue markets |
| `morpho repay` | Repay outstanding debt in Morpho Blue |
| `morpho supply-collateral` | Add collateral to Morpho Blue markets |
| `morpho positions` | View portfolio positions and health factors |
| `morpho markets` | List available lending markets with APYs |
| `morpho vaults` | Browse MetaMorpho vaults and their performance |
| `morpho claim-rewards` | Claim Merkl rewards from lending activity |

## Triggers
Activate this skill when users want to supply assets to earn yield on Morpho, borrow from Morpho Blue markets, check their lending positions and health factors, or manage collateral and debt. Also trigger for viewing Morpho market rates, MetaMorpho vault performance, or claiming protocol rewards.
