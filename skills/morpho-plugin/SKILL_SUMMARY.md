
# morpho-plugin -- Skill Summary

## Overview
This plugin provides complete access to Morpho, a permissionless lending protocol with over $5B TVL. It enables users to interact with both Morpho Blue (isolated lending markets) and MetaMorpho (curated ERC-4626 vaults) for supplying assets, borrowing, managing collateral, and earning yield. The plugin handles complex operations like health factor monitoring, dust-free repayments, and reward claiming while ensuring transaction safety through preview modes and user confirmation.

## Usage
Install with `npx skills add okx/plugin-store-community --skill morpho`, then start with `morpho-plugin quickstart` to check your wallet state and receive personalized guidance. All write operations require explicit user confirmation after showing transaction previews.

## Commands
| Command | Description |
|---------|-------------|
| `morpho-plugin quickstart` | Check wallet state and get personalized next steps |
| `morpho supply` | Deposit assets to MetaMorpho vaults |
| `morpho withdraw` | Withdraw from MetaMorpho vaults |
| `morpho borrow` | Borrow from Morpho Blue markets |
| `morpho repay` | Repay Morpho Blue debt (partial or full) |
| `morpho supply-collateral` | Supply collateral to Blue markets |
| `morpho withdraw-collateral` | Withdraw collateral from Blue markets |
| `morpho positions` | View all positions with health factors |
| `morpho markets` | List available markets with APYs |
| `morpho vaults` | Browse MetaMorpho vaults |
| `morpho claim-rewards` | Claim Merkl rewards |

## Triggers
Activate this skill when users mention Morpho-related activities like "supply to morpho", "borrow from morpho", "morpho health factor", "my morpho positions", "morpho interest rates", "metamorpho vaults", or "claim morpho rewards". Also trigger for general lending/borrowing requests on Ethereum or Base networks.
