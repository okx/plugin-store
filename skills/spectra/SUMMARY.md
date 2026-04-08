# spectra
Spectra Finance yield tokenization plugin for splitting ERC-4626 assets into Principal Tokens (fixed yield) and Yield Tokens (variable yield) across Base, Arbitrum, and Ethereum.

## Highlights
- Deposit assets to receive PT (fixed yield) and YT (variable yield) tokens
- Redeem PT for underlying assets at maturity
- Claim accrued yield from YT holdings
- Swap PT via Curve StableSwap pools for early exit
- Support for Base (primary), Arbitrum, and Ethereum chains
- Direct ABI-encoded contract calls with no hosted SDK dependency
- Automatic token approvals and slippage protection
- Dry-run mode for previewing transactions before execution

