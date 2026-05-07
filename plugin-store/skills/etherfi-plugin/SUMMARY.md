## Overview

Liquid restake ETH on Ethereum to receive eETH — earning staking rewards and EigenLayer restaking points simultaneously — with an optional wrap to auto-compounding weETH and an exit via withdrawal queue.

## Prerequisites
- onchainos agentic wallet connected
- Some ETH on Ethereum mainnet

## How it Works
1. **Check your wallet**: Get a personalised next step based on your ETH and eETH/weETH balances. `etherfi-plugin quickstart`
   - If `status: no_funds` — fund your wallet with ETH on Ethereum mainnet first
   - If `status: needs_gas` — send at least 0.005 ETH to your wallet for gas
   - If `status: ready` — proceed to stake below
2. **Check your positions and APY**: See current eETH/weETH balances and the live staking rate before committing. `etherfi-plugin positions`
3. **Stake ETH**: Deposit ETH into ether.fi and receive eETH — minimum 0.001 ETH enforced by the protocol. `etherfi-plugin stake --amount <amount> --confirm`
4. **Choose how to hold your stake**:
   - 4.1 **Hold as eETH** (simple): Your eETH balance grows daily via rebase — no further action needed.
   - 4.2 **Wrap to weETH** (auto-compounding): Convert eETH to weETH, whose exchange rate appreciates over time rather than rebasing — ERC-20 approval fires automatically. `etherfi-plugin wrap --amount <amount> --confirm`
5. **Exit**: Queue a withdrawal to start the exit process — burns eETH (unwrap weETH first if needed: `etherfi-plugin unwrap --amount <amount> --confirm`) and mints a WithdrawRequestNFT — ERC-20 approval fires automatically. Expect 1–7 days. `etherfi-plugin unstake --amount <amount> --confirm`
6. **Claim ETH**: Once finalized, redeem your WithdrawRequestNFT for ETH back to your wallet. `etherfi-plugin unstake --claim --token-id <ID> --confirm`
