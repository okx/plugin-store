# uniswap-pay-with-any-token

Pay HTTP 402 payment challenges using any token via Tempo CLI and Uniswap Trading API, supporting MPP and x402 protocols

## Source

This skill is maintained by Uniswap Labs in the [uniswap-ai](https://github.com/uniswap/uniswap-ai) monorepo.

The canonical source is at [`packages/plugins/uniswap-trading/skills/pay-with-any-token/`](https://github.com/uniswap/uniswap-ai/tree/main/packages/plugins/uniswap-trading/skills/pay-with-any-token).

### What It Does

- Handles HTTP 402 Payment Required responses using the Machine Payment Protocol (MPP) and x402
- Swaps any held token to the required payment token via Uniswap Trading API
- Supports cross-chain bridging to Tempo for payment fulfillment

### Related Skills

- **uniswap-swap-integration**: Full swap integration guide for applications
- **uniswap-viem-integration**: Foundational EVM blockchain integration

## License

MIT
