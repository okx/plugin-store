
# spectra -- Skill Summary

## Overview
The Spectra Finance plugin enables yield tokenization operations by splitting ERC-4626 assets into Principal Tokens (PT) that provide fixed yield and Yield Tokens (YT) that capture variable yield. Users can deposit assets to receive both token types, redeem PT at maturity, claim accrued yield from YT, and swap PT via Curve pools for early exit. All operations use direct ABI-encoded contract calls with automatic approval handling and slippage protection.

## Usage
Install the plugin and use commands like `spectra get-pools` to view available markets, `spectra deposit` to tokenize yield, and `spectra claim-yield` to collect accrued rewards. Always use `--dry-run` first to preview transactions before execution.

## Commands
| Command | Purpose |
|---------|---------|
| `get-pools` | List available Spectra PT pools with APY and maturity data |
| `get-position` | View wallet PT/YT balances and pending yield |
| `deposit` | Deposit assets to receive PT + YT tokens |
| `redeem` | Redeem PT for underlying assets at maturity |
| `claim-yield` | Claim accrued yield from YT holdings |
| `swap` | Swap PT via Curve pools for early exit |

## Triggers
This skill should be activated when users mention Spectra Finance operations, yield tokenization, fixed yield strategies, PT/YT tokens, or phrases like "deposit Spectra", "claim yield", "redeem PT", or "sell PT early". It's specifically designed for Base chain operations with secondary support for Arbitrum and Ethereum.
