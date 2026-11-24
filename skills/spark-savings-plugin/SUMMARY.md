## Overview

Spark Savings is the yield-bearing arm of Sky (formerly MakerDAO). Deposit USDS or DAI and receive sUSDS - an ERC-4626 vault token that auto-accrues the Sky Savings Rate (SSR). No collateral, no liquidation, just compounding stablecoin yield. This skill supports deposit, withdraw, balance, APY queries on Ethereum (native ERC-4626 vault) and Base/Arbitrum (Spark PSM).

## Prerequisites
- onchainos CLI installed and logged in
- USDS, DAI, or sUSDS on at least one of: Ethereum, Base, Arbitrum
- Native gas token on the chain you intend to transact on (ETH for all 3 chains)

## Quick Start
1. Check your current state and get a guided next step: `spark-savings-plugin quickstart`
2. If you see `status: rpc_degraded` - wait a minute and retry: `spark-savings-plugin quickstart`
3. If you see `status: no_funds` - show address per chain to top up: `spark-savings-plugin balance`
4. If you see `status: has_dai_to_upgrade` - upgrade legacy DAI to USDS 1:1 via the official DaiUsds migrator, then deposit: `spark-savings-plugin upgrade-dai --amount 10 --confirm`
5. If you see `status: ready_to_deposit` - deposit USDS to start earning SSR (use the recommended amount from quickstart): `spark-savings-plugin deposit --chain ETH --amount 10 --confirm`
6. If you see `status: has_susds_earning` - view accrued yield, current APY, and underlying USDS value of your sUSDS holdings: `spark-savings-plugin balance --chain ETH`
7. To redeem some or all of your sUSDS back to USDS at any time: `spark-savings-plugin withdraw --chain ETH --amount 5 --confirm`
8. To check live SSR / APY across all chains: `spark-savings-plugin apy`
