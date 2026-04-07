# spark-savings

Earn the **Sky Savings Rate (SSR)** on USDS/DAI via Spark Protocol's sUSDS/sDAI savings vaults.

## Overview

Spark Savings (by SparkFi / MakerDAO/Sky ecosystem) allows depositing USDS → sUSDS to passively earn the Sky Savings Rate (~3.75% APY). On Ethereum, sUSDS is an ERC-4626 vault. On Base, Arbitrum, and Optimism, deposits flow through the Spark PSM3 contract.

## Supported Chains

| Chain | Chain ID | Mechanism |
|-------|----------|-----------|
| Ethereum Mainnet | 1 | ERC-4626 |
| Base | 8453 | PSM3 |
| Arbitrum One | 42161 | PSM3 |
| Optimism | 10 | PSM3 |

## Commands

```bash
# Check current savings APY
spark-savings --chain 8453 apy

# Check your balance
spark-savings --chain 8453 balance

# Dry-run deposit 10 USDS
spark-savings --chain 8453 --dry-run deposit --amount 10.0

# Execute deposit
spark-savings --chain 8453 deposit --amount 10.0

# Dry-run withdraw all
spark-savings --chain 8453 --dry-run withdraw --all

# Show market stats
spark-savings --chain 8453 markets
```

## Contract Addresses

- **sUSDS (Base)**: `0x5875eEE11Cf8398102FdAd704C9E96607675467a`
- **USDS (Base)**: `0x820C137fa70C8691f0e44Dc420a5e53c168921Dc`
- **PSM3 (Base)**: `0x1601843c5E9bC251A3272907010AFa41Fa18347E`
- **sUSDS (Ethereum)**: `0xa3931d71877C0E7a3148CB7Eb4463524FEc27fbD`
- **sDAI (Ethereum)**: `0x83F20F44975D03b1b09e64809B757c47f942BEeA`

## Building

```bash
cargo build --release
```

## License

MIT
