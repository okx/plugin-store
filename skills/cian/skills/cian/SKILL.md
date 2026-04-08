---
name: cian
description: "CIAN Yield Layer plugin. Trigger phrases: CIAN deposit, CIAN vault, CIAN yield, deposit into CIAN stETH vault, CIAN pumpBTC, CIAN rsETH, CIAN slisBNB, list CIAN vaults, my CIAN position, CIAN APY, CIAN TVL, request CIAN withdrawal, redeem CIAN shares, CIAN Ethereum vault, CIAN Arbitrum vault, CIAN BSC vault"
version: "0.1.0"
author: "skylavis-sky"
tags:
  - yield
  - evm
  - multi-chain
  - delta-neutral
  - erc4626
  - lst
  - btc
  - ethereum
  - arbitrum
  - bsc
---

# cian

Interact with CIAN Yield Layer: list vaults, check positions, deposit tokens, and request withdrawals across Ethereum, Arbitrum, BSC, and Mantle.

## Overview

CIAN Yield Layer is a multi-chain ERC4626 yield aggregator (~$500M+ TVL) that wraps automated
delta-neutral LST/LRT strategies. Users deposit ETH-derivative or BTC-derivative assets and
receive yield-bearing receipt tokens (e.g., ylstETH, ylpumpBTC).

Supported chains: Ethereum (1), Arbitrum (42161), BSC (56), Mantle (5000)

**Always confirm with the user before executing any on-chain transaction.**
Show all parameters and wait for explicit approval before calling deposit or request-withdraw.

## Commands

### list-vaults
List all public CIAN vaults on a chain with APY (7-day average) and TVL.

Usage:
  cian list-vaults [--chain <chain_id>]

Options:
  --chain   Chain ID: 1 (Ethereum, default), 42161 (Arbitrum), 56 (BSC)
            Note: Mantle (5000) has no REST API endpoint for vault listing

Example trigger: "List CIAN vaults on Ethereum" / "CIAN APY on Arbitrum" / "What CIAN vaults are available on BSC?"

### get-positions
Query your position in a CIAN vault: shares, asset value, earnings, and points.

Usage:
  cian get-positions [--chain <chain_id>] [--vault <vault_addr>] [--wallet <wallet_addr>]

Options:
  --chain   Chain ID (default: 1)
  --vault   Vault proxy address (default: 0xB13aa2d0345b0439b064f26B82D8dCf3f508775d stETH YL)
  --wallet  Wallet address (default: resolved from onchainos active wallet)

Example trigger: "My CIAN stETH position" / "How much have I earned in CIAN?" / "Check my CIAN rsETH balance"

### deposit
Deposit tokens into a CIAN vault. Executes two transactions: ERC20 approve then optionalDeposit.

Usage:
  cian deposit [--chain <chain_id>] [--vault <vault_addr>] --token <token_addr> --amount <amount> [--decimals <decimals>] [--dry-run]

Options:
  --chain     Chain ID (default: 1)
  --vault     Vault proxy address (default: 0xB13aa2d0345b0439b064f26B82D8dCf3f508775d)
  --token     Underlying token address (e.g. WETH, stETH, pumpBTC contract address)
  --amount    Amount in human-readable form (e.g. 1.0)
  --decimals  Token decimals (default: 18)
  --dry-run   Simulate without broadcasting

Transaction flow:
  1. approve(vault, MAX_UINT256) on the token contract
  2. Wait 3 seconds (nonce safety)
  3. optionalDeposit(_token, _assets, _receiver, _referral=0x0) on the vault

Example trigger: "Deposit 1 WETH into CIAN stETH vault" / "Put 0.1 pumpBTC into CIAN vault"

### request-withdraw
Request withdrawal of yl-token shares from a CIAN vault (non-instant, queued).

Usage:
  cian request-withdraw [--chain <chain_id>] [--vault <vault_addr>] --shares <amount> [--token <token_addr>] [--decimals <decimals>] [--dry-run]

Options:
  --chain     Chain ID (default: 1)
  --vault     Vault proxy address (default: 0xB13aa2d0345b0439b064f26B82D8dCf3f508775d)
  --shares    Number of yl-token shares to redeem (human-readable, e.g. 0.5)
  --token     Token address to receive (ETH-class vaults only; leave empty for pumpBTC vaults)
  --decimals  Share token decimals (default: 18)
  --dry-run   Simulate without broadcasting

Vault type detection (automatic):
  - BTC-class (pumpBTC): uses requestRedeem(uint256) -- selector 0xaa2f892d
  - ETH-class (all others: stETH, rsETH, ezETH, BTCLST, FBTC, uniBTC): uses requestRedeem(uint256,address) -- selector 0x107703ab

IMPORTANT: Withdrawals are NOT instant. Assets enter a rebalancer queue and may take
hours to a few days to process.

Example trigger: "Withdraw my CIAN stETH shares" / "Redeem CIAN pumpBTC position"

## Do NOT use for

Do NOT use for: non-CIAN vaults, direct rsETH/stETH staking (use ether.fi or Lido skill), CIAN vaults not listed in list-vaults output

## Key Facts

- All vaults are ERC4626 TransparentUpgradeableProxy contracts; call the proxy address directly
- Deposits use optionalDeposit() (not plain ERC4626 deposit) to support multi-token input and referrals
- requestRedeem has two signatures: ETH-class (2 params) vs BTC-class/pumpBTC (1 param)
- Referral address defaults to 0x0000000000000000000000000000000000000000
- Mantle (5000) has no REST API; use on-chain interactions only
- All transactions use --force (handled automatically by the binary)
- approve + deposit use 3-second delay between steps for nonce safety

## Supported Chains

| Chain    | Chain ID | list-vaults | get-positions | deposit | request-withdraw |
|----------|----------|-------------|---------------|---------|-----------------|
| Ethereum | 1        | Yes         | Yes           | Yes     | Yes             |
| Arbitrum | 42161    | Yes         | Yes           | Yes     | Yes             |
| BSC      | 56       | Yes         | Yes           | Yes     | Yes             |
| Mantle   | 5000     | No          | No (no API)   | Yes     | Yes             |

## Vault Addresses

### Ethereum (1)
- stETH Yield Layer:  0xB13aa2d0345b0439b064f26B82D8dCf3f508775d  (WETH/stETH)
- rsETH Yield Layer:  0xd87a19fF681AE98BF10d2220D1AE3Fbd374ADE4e  (WETH/rsETH)
- BTCLST Yield Layer: 0x6c77bdE03952BbcB923815d90A73a7eD7EC895D1  (BTC LST)
- uniBTC Yield Layer: 0xcc7E6dE27DdF225E24E8652F62101Dab4656E20A  (uniBTC)
- ezETH Yield Layer:  0x3D086B688D7c0362BE4f9600d626f622792c4a20  (ezETH)
- pumpBTC Yield Layer: 0xd4Cc9b31e9eF33E392FF2f81AD52BE8523e0993b  (pumpBTC) [BTC-class]
- FBTC Yield Layer:   0x8D76e7847dFbEA6e9F4C235CADF51586bA3560A2  (FBTC)

### Arbitrum (42161)
- rsETH Yield Layer:  0x15cbFF12d53e7BdE3f1618844CaaEf99b2836d2A  (rsETH)

### BSC (56)
- slisBNB Yield Layer: 0x406e1e0e3cb4201B4AEe409Ad2f6Cd56d3242De7  (slisBNB)
- BTCB Yield Layer:    0x74D2Bef5Afe200DaCC76FE2D3C4022435b54CdbB  (BTCB)
- USD1 Yield Layer:    0xD896bf804c01c4C0Fa5C42bF6A4b15C465009481  (USD1)

### Mantle (5000)
- bybit USDT0 Vault: 0x74D2Bef5Afe200DaCC76FE2D3C4022435b54CdbB  (USDT0)
- bybit USDC Vault:  0x6B2BA8F249cC1376f2A02A9FaF8BEcA5D7718DCf  (USDC)

## Function Selectors

| Function | Selector |
|----------|----------|
| ERC-20 approve(address,uint256) | 0x095ea7b3 |
| optionalDeposit(address,uint256,address,address) | 0x32507a5f |
| deposit(uint256,address) | 0x6e553f65 |
| requestRedeem(uint256,address) -- ETH-class | 0x107703ab |
| requestRedeem(uint256) -- BTC-class pumpBTC | 0xaa2f892d |
| asset() | 0x38d52e0f |
| balanceOf(address) | 0x70a08231 |
| exchangePrice() | 0x9e65741e |
| maxDeposit(address) | 0x402d267d |
