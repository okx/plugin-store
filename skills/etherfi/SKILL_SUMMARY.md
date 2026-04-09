
# etherfi -- Skill Summary

## Overview
The ether.fi plugin enables liquid restaking on Ethereum through a comprehensive workflow: deposit ETH to receive eETH liquid staking tokens, wrap eETH into weETH (ERC-4626 yield-bearing tokens) to earn auto-compounded staking and EigenLayer restaking rewards, and monitor positions with real-time APY data. All write operations use onchainos wallet integration with mandatory confirmation gates for secure transaction execution.

## Usage
Run commands without `--confirm` to preview transactions, then add `--confirm` to broadcast. Use `etherfi positions` to check balances and APY anytime.

## Commands
| Command | Description |
|---------|-------------|
| `etherfi positions [--owner ADDRESS]` | View eETH/weETH balances and protocol APY (read-only) |
| `etherfi stake --amount ETH [--confirm]` | Deposit ETH to receive eETH |
| `etherfi wrap --amount EETH [--confirm]` | Wrap eETH into yield-bearing weETH |
| `etherfi unwrap --amount WEETH [--confirm]` | Redeem weETH back to eETH |

## Triggers
Activate when users want to stake ETH on ether.fi, wrap/unwrap between eETH and weETH, check ether.fi positions or APY, or engage with liquid restaking protocols. Also trigger for phrases like "get weETH", "ether.fi liquid restaking", or similar DeFi staking requests.
