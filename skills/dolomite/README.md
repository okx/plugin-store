# Dolomite Plugin

Interact with [Dolomite](https://dolomite.io/) isolated lending markets from the onchainos CLI.

## Operations

- **markets** — List all Dolomite lending markets with TVL and rates
- **positions** — View your supply/borrow positions
- **deposit** — Supply tokens to earn yield
- **withdraw** — Withdraw supplied tokens
- **borrow** — Simulate borrowing (dry-run only)
- **repay** — Simulate debt repayment (dry-run only)

## Chains

| Chain     | ID    |
|-----------|-------|
| Arbitrum  | 42161 |
| Mantle    | 5000  |
| Berachain | 80094 |

## Quick Start

```bash
# List markets on Arbitrum
dolomite --chain 42161 markets

# View positions
dolomite --chain 42161 positions

# Dry-run deposit
dolomite --chain 42161 --dry-run deposit --asset USDC --amount 10

# Real deposit (requires confirmation)
dolomite --chain 42161 deposit --asset USDC --amount 10
```

## Architecture

Dolomite uses a central `DolomiteMargin` contract on Arbitrum (`0x6Bd780E7fDf01D77e4d475c821f1e7AE05409072`). All deposits, withdrawals, and borrows are executed via the `operate()` function with typed `ActionArgs`.
