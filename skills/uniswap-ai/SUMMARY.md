## Overview

`uniswap-ai` is the official Uniswap bundle of AI-agent skills mirrored from `github.com/Uniswap/uniswap-ai`, packaging **5 sub-plugins and 10 skills** that cover the full Uniswap developer surface — swap integration, v4 hook generation and security, Continuous Clearing Auction (CCA) deployment and configuration, swap/liquidity planning with deep-link generation, machine-payment (HTTP 402 / x402) settlement, and EVM connectivity via viem. Each skill activates from natural-language triggers in any Claude-compatible agent, and together they let a developer go from "describe the integration" to working frontends, scripts, contracts, hooks, or auctions across all Uniswap-supported chains (Ethereum, Base, Arbitrum, Optimism, Polygon, …) on V2 / V3 / V4.

## Prerequisites
- Claude Code (latest) or any agent that supports the Skills protocol
- Node.js 22.x and npm 11.7.0+ (only required for local development and contribution)
- An EVM wallet (e.g. MetaMask, Rabby, or a Smart Account / ERC-4337 wallet) for runtime swaps and liquidity actions
- For hook development: a [Foundry](https://book.getfoundry.sh/) project; the `v4-hook-generator` skill also requires the OpenZeppelin MCP server
- For `pay-with-any-token`: the [Tempo CLI](https://tempo.dev) installed and configured for x402 / MPP payments
- For `swap-integration` / `v4-sdk-integration`: a Uniswap [Trading API](https://trade-api.gateway.uniswap.org/v1/) key (or use the SDK direct path)

## Quick Start

### Install
```bash
npx skills add Uniswap/uniswap-ai -y -g
```
This pulls all 5 sub-plugins (10 skills) directly from `github.com/Uniswap/uniswap-ai`. Inside Claude Code you can alternatively use the marketplace: `/plugin marketplace add uniswap/uniswap-ai` followed by `/plugin install uniswap-trading|uniswap-hooks|uniswap-cca|uniswap-driver|uniswap-viem`.

### Trading — `uniswap-trading` (3 skills)
1. **`swap-integration`** — say *"integrate Uniswap swaps into my app"*, *"build a swap frontend"*, *"create a swap script"*, or *"smart contract swap via Universal Router"* → scaffolds React/TS frontends, Node.js scripts, or Solidity integrations using the Trading API, Universal Router SDK, or direct contract calls
2. **`v4-sdk-integration`** — say *"v4 sdk"*, *"PoolManager"*, *"V4Planner"*, *"StateView"*, *"PositionManager"* → builds swap and liquidity UX directly on the Uniswap v4 SDK (Quoter, Planner, Position Manager) without going through the Trading API
3. **`pay-with-any-token`** — say *"HTTP 402"*, *"x402"*, *"MPP"*, *"pay with Tempo"* → solves a 402 Payment Required challenge by routing the bill through a Uniswap swap via the Tempo CLI

### Hooks — `uniswap-hooks` (2 skills)
4. **`v4-hook-generator`** — say *"generate a v4 hook for dynamic fees"*, *"limit order hook"*, *"oracle hook"*, *"async swap"*, *"hook-owned liquidity"*, *"MEV protection"* → emits a complete Solidity hook contract via OpenZeppelin MCP, with permissions wired correctly
5. **`v4-security-foundations`** — say *"v4 hook security"*, *"beforeSwap / afterSwap"*, *"hook audit"* → walks through best practices, common vulnerability classes, and an audit checklist before deployment

### Continuous Clearing Auction — `uniswap-cca` (2 skills)
6. **`configurator`** — say *"configure auction"*, *"setup token auction"*, *"continuous auction"* → drives an interactive bulk form that builds the CCA parameter set
7. **`deployer`** — say *"deploy auction"*, *"deploy cca"*, *"factory deployment"* → deploys the configured CCA via the Factory pattern (CREATE2 for deterministic addresses)

### Planning — `uniswap-driver` (2 skills)
8. **`swap-planner`** — say *"swap ETH for USDC"*, *"trade tokens on Uniswap"*, *"find memecoins to buy"*, *"discover tokens"* → researches the token via keyword + web search and generates a Uniswap-interface deep link to execute the swap
9. **`liquidity-planner`** — say *"provide liquidity"*, *"create LP position"*, *"set price range"*, *"v3 / v4 concentrated liquidity"* → generates a Uniswap-interface deep link to open the LP position with the chosen range and fee tier

### EVM connectivity — `uniswap-viem` (1 skill)
10. **`viem-integration`** — say *"read blockchain data"*, *"send transaction"*, *"connect to Ethereum with viem"*, *"wagmi setup"* → scaffolds TypeScript / wagmi clients, contract reads, transaction sends, and wallet connections on any EVM chain

### Verify
After install, the skills auto-trigger on the natural-language phrases above. You can also invoke a skill directly by slash command (e.g. `/v4-security-foundations`) when running inside Claude Code.
