# uniswap-cca-deployer

Deploy Continuous Clearing Auction (CCA) smart contracts using the Factory pattern with CREATE2 for consistent addresses

## Source

This skill is maintained by Uniswap Labs in the [uniswap-ai](https://github.com/uniswap/uniswap-ai) monorepo.

The canonical source is at [`packages/plugins/uniswap-cca/skills/deployer/`](https://github.com/uniswap/uniswap-ai/tree/main/packages/plugins/uniswap-cca/skills/deployer).

### What It Does

- Guides AI agents through deploying CCA contracts via the Factory
- Uses CREATE2 for deterministic addresses across chains
- Covers post-deployment verification and auction initialization

### Related Skills

- **uniswap-cca-configurator**: Configure auction parameters before deployment
- **uniswap-viem-integration**: Foundational EVM blockchain integration

## License

MIT
