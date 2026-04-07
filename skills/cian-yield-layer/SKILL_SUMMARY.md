
# cian-yield-layer -- Skill Summary

## Overview
CIAN Yield Layer operates ERC4626 vaults on Ethereum Mainnet that amplify staking yields through sophisticated recursive staking and restaking strategies. The protocol offers ylstETH vault for Ethereum-based assets and ylpumpBTC vault for Bitcoin-based assets, implementing automated leverage strategies on Lido and EigenLayer to maximize returns while maintaining risk management through protocol-controlled rebalancing.

## Usage
Install the plugin via `plugin-store install cian-yield-layer`, ensure onchainos wallet is logged in, then use commands like `cian-yield-layer deposit --vault ylsteth --token ETH --amount 1.0` to deposit or `cian-yield-layer request-redeem` to initiate the 5-day async withdrawal process.

## Commands
| Command | Description |
|---------|-------------|
| `vaults` | List all available vaults with APY, TVL, and accepted tokens |
| `balance` | Check vault share balances for logged-in or specified wallet |
| `positions` | View complete position details including pending redeems |
| `deposit` | Deposit ETH/stETH/wstETH/weETH/pumpBTC/WBTC into vaults |
| `request-redeem` | Initiate async 5-day withdrawal process (irreversible) |

## Triggers
Activate when users mention CIAN Yield Layer, ylstETH, ylpumpBTC, depositing into CIAN vaults, CIAN staking strategies, requesting CIAN withdrawals, or checking CIAN balances and positions. Do not use for general ETH staking unrelated to CIAN or other CIAN products outside Yield Layer.
