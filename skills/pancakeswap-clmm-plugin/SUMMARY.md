# pancakeswap-clmm

**Overview**
Stake PancakeSwap V3 LP NFTs into MasterChefV3 to earn CAKE rewards on top of swap fees — with harvest, unfarm, and collect-fees commands across BSC, Ethereum, Base, and Arbitrum.

**Prerequisites**
- onchainos CLI installed and logged in with a BSC wallet (chain 56, default)
- A PancakeSwap V3 LP NFT (create one with `pancakeswap-v3` plugin)
- Some BNB in your wallet for gas

**How it Works**
1. Check your existing V3 LP positions: `pancakeswap-clmm-plugin positions` — auto-discovers staked and unstaked NFTs
2. Browse active farming pools: `pancakeswap-clmm-plugin farm-pools` — shows pools with CAKE emissions and allocation points
3. Preview staking (no transaction): `pancakeswap-clmm-plugin farm --token-id <TOKEN_ID>`
4. Stake the NFT: `pancakeswap-clmm-plugin farm --token-id <TOKEN_ID> --confirm`
5. Check pending CAKE rewards: `pancakeswap-clmm-plugin pending-rewards --token-id <TOKEN_ID>`
6. Harvest CAKE without unstaking: `pancakeswap-clmm-plugin harvest --token-id <TOKEN_ID> --confirm`
7. Stop farming — withdraw NFT and harvest remaining CAKE: `pancakeswap-clmm-plugin unfarm --token-id <TOKEN_ID> --confirm`
