# etherfi-plugin
Liquid restaking on Ethereum — deposit ETH to receive eETH, wrap/unwrap eETH/weETH (ERC-4626), unstake eETH back to ETH, and check positions with APY.

## Highlights
- Deposit ETH into ether.fi to receive liquid staking token eETH
- Wrap eETH into weETH (ERC-4626) to earn auto-compounding staking + EigenLayer restaking rewards
- Unwrap weETH back to eETH to realize accumulated yields
- Two-step ETH withdrawal process: request withdrawal (burns eETH, mints NFT) then claim after finalization
- Real-time balance tracking and APY monitoring from on-chain data
- Preview-first transaction flow with explicit confirmation required for all write operations
- Support for checking positions and yields across eETH/weETH holdings
- Integrated with onchainos CLI for secure transaction signing and wallet management

