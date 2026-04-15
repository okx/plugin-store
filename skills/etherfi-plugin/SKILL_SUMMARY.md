
# etherfi-plugin -- Skill Summary

## Overview
The etherfi-plugin enables liquid restaking on Ethereum through the ether.fi protocol. Users can deposit ETH to receive eETH (liquid staking token), wrap eETH into weETH (an ERC-4626 yield-bearing token) to earn staking plus EigenLayer restaking rewards, manage positions, and withdraw back to ETH through a two-step finalization process. The plugin provides real-time APY tracking and balance monitoring while ensuring secure transactions through onchainos integration.

## Usage
Install the plugin and ensure onchainos CLI is configured with your wallet. Run `etherfi positions` to check current balances and APY, then use commands like `etherfi stake --amount 0.1 --confirm` to deposit ETH or `etherfi wrap --amount 1.0 --confirm` to convert eETH to yield-bearing weETH.

## Commands
| Command | Description |
|---------|-------------|
| `etherfi positions [--owner ADDRESS]` | View eETH/weETH balances, total value, and current APY |
| `etherfi stake --amount ETH [--confirm]` | Deposit ETH to receive eETH (minimum 0.001 ETH) |
| `etherfi wrap --amount EETH [--confirm]` | Wrap eETH into weETH (ERC-4626 yield-bearing token) |
| `etherfi unwrap --amount WEETH [--confirm]` | Unwrap weETH back to eETH |
| `etherfi unstake --amount EETH [--confirm]` | Request eETH withdrawal (step 1: burns eETH, mints NFT) |
| `etherfi unstake --claim --token-id ID [--confirm]` | Claim ETH after withdrawal finalization (step 2: burns NFT) |

## Triggers
AI agents should activate this skill when users want to participate in Ethereum liquid restaking, earn staking + EigenLayer rewards through weETH, or manage existing ether.fi positions. Use for ETH staking with liquidity, yield optimization through ERC-4626 tokens, or checking ether.fi portfolio balances and APY.
