**Overview**

Liquid restake ETH on Ethereum to receive eETH — earning staking rewards and EigenLayer restaking points simultaneously — with an optional wrap to auto-compounding weETH and an exit via withdrawal queue.

**Prerequisites**
- onchainos agentic wallet connected
- Ethereum mainnet wallet (chain 1) with at least 0.001 ETH

**How it Works**
1. **Check your positions and APY**: See current eETH/weETH balances and the live staking rate before committing. `etherfi-plugin positions`
2. **Stake ETH**: Deposit ETH into ether.fi and receive eETH — approval fires automatically; minimum 0.001 ETH enforced by the protocol. `etherfi-plugin stake --amount 0.1 --confirm`
3. **Choose how to hold your stake**:
   - 3.1 **Hold as eETH** (simple): Your eETH balance grows daily via rebase — no further action needed.
   - 3.2 **Wrap to weETH** (auto-compounding): Convert eETH to weETH, whose exchange rate appreciates over time rather than rebasing. `etherfi-plugin wrap --amount 0.1 --confirm`
4. **Exit**: Queue a withdrawal to start the exit process — burns eETH (unwrap weETH first if needed) and mints a WithdrawRequestNFT. Expect 1–7 days. `etherfi-plugin unstake --amount 0.1 --confirm`
5. **Claim ETH**: Once finalized, redeem your WithdrawRequestNFT for ETH back to your wallet. `etherfi-plugin unstake --claim --token-id <ID> --confirm`
