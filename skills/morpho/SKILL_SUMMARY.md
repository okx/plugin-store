
# morpho -- Skill Summary

## Overview
The morpho skill enables interaction with Morpho, a permissionless lending protocol with $5B+ TVL operating on Ethereum and Base. It supports both Morpho Blue isolated markets for borrowing/lending with collateral, and MetaMorpho ERC-4626 vaults that aggregate liquidity across markets with professional risk management by curators like Gauntlet and Steakhouse.

## Usage
Install with `npx skills add okx/plugin-store-community --skill morpho`. Connect your wallet via `onchainos wallet login`, then use commands like `morpho positions` to view your portfolio or `morpho markets --asset USDC` to browse lending opportunities.

## Commands
| Command | Description |
|---------|-------------|
| `morpho positions` | View your Morpho Blue positions and MetaMorpho vault balances with health factors |
| `morpho markets [--asset TOKEN]` | List Morpho Blue markets with supply/borrow APYs and utilization rates |
| `morpho vaults [--asset TOKEN]` | Browse MetaMorpho vaults with APYs and curator information |
| `morpho supply --vault ADDR --asset TOKEN --amount N [--confirm]` | Deposit assets to a MetaMorpho vault to earn yield |
| `morpho withdraw --vault ADDR --asset TOKEN --amount N [--confirm]` | Withdraw from MetaMorpho vault (use `--all` for full withdrawal) |
| `morpho borrow --market-id HEX --amount N [--confirm]` | Borrow from a Morpho Blue market |
| `morpho repay --market-id HEX --amount N [--confirm]` | Repay Morpho Blue debt (use `--all` for dust-free full repayment) |
| `morpho supply-collateral --market-id HEX --amount N [--confirm]` | Add collateral to a Morpho Blue market |
| `morpho withdraw-collateral --market-id HEX --amount N [--confirm]` | Remove collateral from Morpho Blue market |
| `morpho claim-rewards [--confirm]` | Claim Merkl rewards |

Global flags: `--chain 1|8453` (Ethereum/Base), `--dry-run`, `--from ADDRESS`

## Triggers
Activate this skill when users mention supplying/depositing to Morpho vaults, borrowing from Morpho Blue markets, checking Morpho positions or health factors, viewing Morpho interest rates, repaying Morpho loans, managing collateral, claiming Morpho rewards, or browsing MetaMorpho vaults. Also triggered by phrases like "earn yield on morpho", "morpho lending", or "metamorpho".
