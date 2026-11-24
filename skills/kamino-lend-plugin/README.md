# kamino-lend

Kamino Lend plugin for OKX Plugin Store — supply, borrow, and manage positions on [Kamino Lend](https://kamino.finance), the leading lending protocol on Solana.

## Features

- **Markets**: View all Kamino lending markets with current supply/borrow APYs and TVL
- **Positions**: Query your current lending obligations and health factor
- **Supply**: Deposit assets to earn yield
- **Withdraw**: Withdraw supplied assets
- **Borrow**: Borrow assets against collateral (dry-run supported)
- **Repay**: Repay outstanding loans (dry-run supported)

## Chain Support

- Solana mainnet (chain 501)

## Usage

```bash
# List markets and APYs
kamino-lend markets

# Check your positions
kamino-lend positions

# Supply 0.01 USDC
kamino-lend supply --token USDC --amount 0.01

# Withdraw 0.01 USDC
kamino-lend withdraw --token USDC --amount 0.01

# Preview borrow (dry-run)
kamino-lend borrow --token SOL --amount 0.001 --dry-run

# Preview repay (dry-run)
kamino-lend repay --token SOL --amount 0.001 --dry-run
```

## Key Addresses

- Main Market: `7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF`
- Kamino Lend Program: `KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD`

## Important Notes

- Amounts are always in UI units (0.01 USDC = 0.01, not 10000)
- Solana transactions expire in ~60 seconds; transactions are submitted immediately
- Borrowing requires prior collateral supply (obligation must exist)
