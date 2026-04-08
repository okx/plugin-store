# sky-lending

Sky Lending (MakerDAO CDP) plugin for onchainos. Manage collateralized debt positions on Ethereum mainnet — deposit collateral, borrow DAI, repay debt, withdraw collateral.

## Features

- List collateral types (ilks) with rates and parameters
- View CDP vaults for any address
- Generate calldata for: open vault, deposit collateral, draw DAI, repay DAI, withdraw collateral
- All write operations are dry-run only (CDP liquidation risk safety measure)

## Supported Chains

- Ethereum mainnet (chain ID 1)

## Commands

```
sky-lending ilks                     # List collateral types
sky-lending vaults                   # List your CDP vaults
sky-lending open-vault --ilk ETH-A   # Open new vault (dry-run)
sky-lending deposit-collateral --amount-eth 1.0   # Deposit ETH (dry-run)
sky-lending draw-dai --amount-dai 500.0           # Draw DAI (dry-run)
sky-lending repay-dai --amount-dai 100.0          # Repay DAI (dry-run)
sky-lending withdraw-collateral --amount-eth 0.5  # Withdraw ETH (dry-run)
```

## Build

```bash
cargo build --release
```

## License

MIT
