
# morpho -- Skill Summary

## Overview
The Morpho skill enables interaction with the Morpho lending protocol ecosystem, supporting both MetaMorpho vault deposits for yield farming and Morpho Blue isolated market borrowing. Users can supply assets to earn yield, borrow against collateral, monitor health factors, and claim rewards across Ethereum and Base networks. The skill provides comprehensive position management with safety features including transaction previews and health factor warnings.

## Usage
Install with `npx skills add okx/plugin-store-community --skill morpho`, then use commands like `morpho positions` to view your portfolio or `morpho supply --vault 0x... --asset USDC --amount 1000` to deposit assets. All write operations require dry-run previews and user confirmation before execution.

## Commands
| Command | Description |
|---------|-------------|
| `morpho positions` | View positions and health factors across Blue markets and MetaMorpho vaults |
| `morpho markets [--asset TOKEN]` | List Morpho Blue markets with APYs and utilization rates |
| `morpho vaults [--asset TOKEN]` | List MetaMorpho vaults with APYs and curators |
| `morpho supply --vault ADDR --asset TOKEN --amount N` | Supply assets to MetaMorpho vault |
| `morpho withdraw --vault ADDR --asset TOKEN --amount N` | Withdraw from MetaMorpho vault |
| `morpho borrow --market-id HEX --amount N` | Borrow from Morpho Blue market |
| `morpho repay --market-id HEX --amount N` | Repay Morpho Blue debt |
| `morpho supply-collateral --market-id HEX --amount N` | Supply collateral to Blue market |
| `morpho withdraw-collateral --market-id HEX --amount N` | Withdraw collateral from Blue market |
| `morpho claim-rewards` | Claim Merkl distributor rewards |

## Triggers
Activate when users mention Morpho protocol operations like "supply to morpho vault", "borrow from morpho blue", "check my morpho positions", "morpho health factor", or "claim morpho rewards". Also trigger for general lending/borrowing intent on Ethereum or Base networks.
