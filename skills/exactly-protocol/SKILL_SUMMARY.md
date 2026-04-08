
# exactly-protocol -- Skill Summary

## Overview
This skill provides access to Exactly Protocol, a decentralized lending platform offering both fixed-rate deposits with guaranteed APY until maturity and floating-rate pools for flexible lending. Unlike traditional protocols, Exactly requires explicit collateral enablement before borrowing and uses weekly-aligned maturity timestamps for fixed-rate positions. The protocol operates on Optimism (primary) and Ethereum Mainnet with support for major assets like WETH, USDC, and wstETH.

## Usage
Install the plugin, connect your wallet via onchainos, then use commands like `exactly-protocol get-markets` to view available lending opportunities. Always run commands with `--dry-run` first and confirm transactions before execution.

## Commands
| Command | Purpose |
|---------|---------|
| `get-markets` | List all available markets and current rates |
| `get-position` | View your lending/borrowing positions |
| `deposit` | Lend assets at floating or fixed rates (with --maturity) |
| `borrow` | Borrow assets at floating or fixed rates |
| `repay` | Repay outstanding borrows |
| `withdraw` | Withdraw deposited assets |
| `enter-market` | Enable an asset as collateral for borrowing |

## Triggers
Activate this skill when users mention "exactly protocol", "fixed rate lending", "lend at fixed APY", "exactly borrow", "exactly deposit", "fixed maturity", or want predictable lending returns with locked rates until specific dates.
