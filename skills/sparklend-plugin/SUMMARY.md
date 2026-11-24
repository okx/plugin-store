## Overview

SparkLend is an Aave V3 fork governed by Sky Protocol (formerly MakerDAO), offering overcollateralized lending and borrowing on Ethereum Mainnet with competitive rates for DAI, USDS, wstETH, WETH, and other blue-chip assets.

Core operations:
- Supply collateral assets to earn interest (spTokens)
- Borrow against your collateral at variable rates
- Monitor health factor to avoid liquidation
- Repay debt fully or partially

Tags: `defi` `ethereum` `lending` `aave-v3` `sparklend`

## Prerequisites

- No IP restrictions
- Supported chain: Ethereum Mainnet (chain ID: 1)
- Supported tokens: DAI, USDC, USDT, USDS, sUSDS, sDAI, wstETH, WETH, rETH, weETH, cbBTC, WBTC, LBTC, tBTC, ezETH, rsETH, PYUSD, GNO
- onchainos CLI installed and authenticated (`onchainos wallet login`)
- Ethereum Mainnet wallet with ETH for gas

## Quick Start

1. **Check your wallet**: Run `onchainos wallet addresses --chain 1` to confirm your wallet is connected.
2. **See market rates**: Run `sparklend reserves` to browse supply and borrow APYs across all assets.
3. **Supply assets**: Run `sparklend supply --asset DAI --amount 1000` to preview, then add `--confirm` to execute.
4. **Borrow**: After supplying, run `sparklend borrow --asset USDC --amount 500` to borrow against your collateral.
5. **Monitor health**: Run `sparklend health-factor` to check liquidation risk.
6. **Repay**: Run `sparklend repay --asset USDC --all` to repay full outstanding debt.
7. **Withdraw**: Run `sparklend withdraw --asset DAI --all` to withdraw your collateral.
