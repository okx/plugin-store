
# exactly-protocol -- Skill Summary

## Overview
The Exactly Protocol skill enables interaction with a decentralized lending protocol that offers both fixed-rate, fixed-term lending via maturity-based pools and variable-rate floating pools. Unlike traditional lending protocols, Exactly requires explicit collateral enablement through `enter-market` commands before borrowing against deposited assets. The protocol is deployed on Optimism (primary, lower gas) and Ethereum Mainnet, supporting major DeFi assets with predictable fixed-rate returns and flexible floating-rate options.

## Usage
Install the plugin and use commands like `exactly-protocol get-markets` to view available lending pools, `exactly-protocol deposit` for lending, and `exactly-protocol borrow` for borrowing. Always run commands with `--dry-run` first to preview transactions before execution.

## Commands
| Command | Purpose |
|---------|---------|
| `get-markets` | List all available markets, rates, and maturities |
| `get-position` | View user's lending/borrowing positions |
| `deposit` | Lend assets at fixed or floating rates |
| `withdraw` | Withdraw deposited assets (with early penalty for fixed) |
| `borrow` | Borrow assets at fixed or floating rates |
| `repay` | Repay outstanding borrows |
| `enter-market` | Enable deposited asset as collateral |
| `--dry-run` | Preview any transaction before execution |
| `--chain` | Specify chain (10 for Optimism, 1 for Ethereum) |

## Triggers
Activate this skill when users want to lend or borrow at fixed rates with known maturity dates, or when they mention "exactly protocol," "fixed rate lending," "maturity deposits," or need predictable DeFi returns with explicit collateral management.
