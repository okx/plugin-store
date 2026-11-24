## Overview

Swap tokens and provide liquidity on Velodrome V2's AMM on Optimism — supporting volatile and stable pool types — earning trading fees and VELO emissions from pool gauges.

## Prerequisites
- onchainos agentic wallet connected
- Some tokens on Optimism to swap or provide as liquidity

## Quick Start
1. **Swap tokens**:
   - 1.1 **Get a quote**: Check the expected output before committing — auto-checks both volatile and stable pools (no gas). `velodrome-v2-plugin quote --token-in WETH --token-out USDC --amount-in <amount>`
   - 1.2 **Execute the swap**: Send tokens and receive output in one transaction. `velodrome-v2-plugin swap --token-in WETH --token-out USDC --amount-in <amount> --slippage 0.5 --confirm`
2. **Provide liquidity**:
   - 2.1 **Check the pool**: Verify the pair exists and see pool type (volatile or stable). `velodrome-v2-plugin pools --token-a WETH --token-b USDC`
   - 2.2 **Add liquidity**: Deposit both tokens of the pair to receive LP tokens — use `--stable` flag for stable pools (USDC/DAI). `velodrome-v2-plugin add-liquidity --token-a WETH --token-b USDC --amount-a-desired <amount> --confirm`
   - 2.3 **View LP balance**: Check your current LP position in the pool. `velodrome-v2-plugin positions --token-a WETH --token-b USDC`
   - 2.4 **Remove liquidity**: Withdraw your LP tokens and receive both underlying tokens back. `velodrome-v2-plugin remove-liquidity --token-a WETH --token-b USDC --confirm`
   - 2.5 **Claim VELO rewards**: Collect VELO emissions — requires an existing LP position in an incentivized pool. `velodrome-v2-plugin claim-rewards --token-a WETH --token-b USDC --confirm`
