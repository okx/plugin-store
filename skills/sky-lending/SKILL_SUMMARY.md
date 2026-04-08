
# sky-lending -- Skill Summary

## Overview
Sky Lending enables management of MakerDAO's collateralized debt positions (CDPs) on Ethereum mainnet. Users can deposit collateral like ETH into vaults and borrow DAI stablecoins against it. The skill provides comprehensive vault management including querying collateral types, viewing positions, and generating transaction calldata for all CDP operations. All write operations are safety-protected with mandatory dry-run mode to prevent accidental liquidations.

## Usage
Install and use through onchainos CLI. Start by checking available collateral types with `sky-lending ilks`, then view your vaults with `sky-lending vaults`. All vault operations (open, deposit, draw, repay, withdraw) generate dry-run calldata for safety verification.

## Commands
| Command | Description |
|---------|-------------|
| `sky-lending ilks` | List collateral types with rates and parameters |
| `sky-lending vaults` | List CDP vaults for an address |
| `sky-lending open-vault --ilk ETH-A` | Open new vault (dry-run) |
| `sky-lending deposit-collateral --amount-eth 1.0` | Deposit ETH collateral (dry-run) |
| `sky-lending draw-dai --amount-dai 500.0` | Draw/mint DAI against collateral (dry-run) |
| `sky-lending repay-dai --amount-dai 100.0` | Repay DAI debt (dry-run) |
| `sky-lending withdraw-collateral --amount-eth 0.5` | Withdraw ETH collateral (dry-run) |

## Triggers
Activate when users need to manage MakerDAO/Sky CDP positions, borrow DAI stablecoins against collateral, or check vault health and liquidation risks. Use for all collateralized lending operations on Ethereum mainnet.
