
# etherfi -- Skill Summary

## Overview
The etherfi skill enables liquid restaking on Ethereum through the ether.fi protocol. Users can deposit ETH to receive eETH liquid staking tokens, wrap eETH into weETH (an ERC-4626 yield-bearing token) that auto-compounds staking and EigenLayer restaking rewards, and monitor their positions with real-time APY data. All write operations use a secure two-step confirmation process through onchainos wallet integration.

## Usage
Install the etherfi binary and ensure onchainos CLI is configured with your wallet. Run commands like `etherfi positions` to check balances, `etherfi stake --amount 0.1` to deposit ETH, and `etherfi wrap --amount 1.0` to convert eETH to yield-bearing weETH.

## Commands
| Command | Description |
|---------|-------------|
| `etherfi positions [--owner ADDRESS]` | View eETH/weETH balances and protocol APY |
| `etherfi stake --amount AMOUNT [--confirm]` | Deposit ETH to receive eETH |
| `etherfi wrap --amount AMOUNT [--confirm]` | Wrap eETH to weETH (ERC-4626) |
| `etherfi unwrap --amount AMOUNT [--confirm]` | Redeem weETH back to eETH |

All write operations support `--dry-run` for preview and require `--confirm` to broadcast.

## Triggers
AI agents should activate this skill when users want to stake ETH on ether.fi, wrap/unwrap between eETH and weETH tokens, check their liquid restaking positions, or earn EigenLayer restaking rewards alongside Ethereum staking yields.
