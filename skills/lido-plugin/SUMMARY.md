**Overview**

Stake ETH on Ethereum mainnet and receive stETH — a liquid staking token that earns daily validator rewards via automatic rebase, redeemable for ETH after a 1–5 day withdrawal queue.

**Prerequisites**
- onchainos agentic wallet connected
- Some ETH on Ethereum mainnet

**How it Works**
1. **Check the APY**: See the live Lido stETH staking rate before committing — shows current APY, TVL, and 30-day trend sourced from DeFiLlama. `lido-plugin get-apy`
2. **Stake ETH**: Deposit ETH into Lido and receive stETH in your wallet — a preview shows the expected stETH amount first; add `--confirm` to execute. `lido-plugin stake --amount-eth <amount>`
3. **Watch your balance grow**: Your stETH balance increases automatically each day via rebase — no claiming or compounding needed. `lido-plugin balance`
4. **Request a withdrawal**: Begin the exit process by burning stETH and minting a WithdrawalNFT that represents your ETH claim — note the NFT ID, you'll need it to claim. Expect a 1–5 day wait. `lido-plugin request-withdrawal --amount <amount> --confirm`
5. **Track your withdrawal**: Check the status of pending withdrawals — progresses through PENDING → READY TO CLAIM → CLAIMED with an estimated wait time. `lido-plugin get-withdrawals`
6. **Claim your ETH**: Once status shows READY TO CLAIM, redeem your WithdrawalNFT for ETH back to your wallet. `lido-plugin claim-withdrawal --ids <ID> --confirm`
