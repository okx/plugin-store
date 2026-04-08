# sky-lending
Manages MakerDAO/Sky collateralized debt positions on Ethereum — deposit collateral, borrow DAI stablecoins, and manage vault operations.

## Highlights
- List collateral types (ilks) with current rates and debt parameters
- View CDP vaults and positions for any Ethereum address
- Generate calldata for opening new vaults with specified collateral types
- Deposit ETH collateral into existing vaults (dry-run safety mode)
- Draw/mint DAI stablecoins against deposited collateral (dry-run)
- Repay DAI debt to reduce liquidation risk (dry-run)
- Withdraw collateral from vaults when safely over-collateralized (dry-run)
- Built-in liquidation risk protection with mandatory dry-run operations

