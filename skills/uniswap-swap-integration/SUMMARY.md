# uniswap-swap-integration
Integrate Uniswap token swaps into frontends, backends, and smart contracts via the Trading API, Universal Router SDK, or direct contract calls.

## Highlights
- Three integration paths: Trading API (recommended), Universal Router SDK, and direct smart contract calls
- Trading API 3-step flow: check_approval → quote → swap
- Routing types: CLASSIC, DUTCH_V2, DUTCH_V3, PRIORITY, LIMIT_ORDER, BRIDGE, and QUICKROUTE
- Universal Router SDK for full control over transaction construction
- Permit2 token approval support
- Input validation rules for addresses, chain IDs, amounts, and API keys
- Mandatory user confirmation before any gas-spending transaction
- Multi-chain support across all Uniswap-supported networks
