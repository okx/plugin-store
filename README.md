# OKX Plugin Store

Discover, install, and build AI agent plugins for DeFi, trading, and Web3.

**Supported platforms:** Claude Code, Cursor, OpenClaw

```bash
npx skills add okx/plugin-store --skill <plugin-name>
```

---

## Plugin Directory

| Plugin | Description | Source | Risk | Install Command |
|--------|-------------|--------|------|-----------------|
| plugin-store | CLI marketplace for discovering, installing, and managing plugins | 🟢 Official | 🟢 Starter | `npx skills add okx/plugin-store --skill plugin-store` |
| okx-buildx-hackathon-agent-track | AI Hackathon participation guide: registration, wallet setup, project building, and submission | 🟢 Official | 🟢 Starter | `npx skills add okx/plugin-store --skill okx-buildx-hackathon-agent-track` |
| uniswap-ai | AI-powered Uniswap developer tools: trading, hooks, drivers, and on-chain analysis across V2/V3/V4 | 🔵 Verified Partner | 🟡 Standard | `npx skills add okx/plugin-store --skill uniswap-ai` |
| uniswap-swap-planner | Plan token swaps and generate Uniswap deep links across all supported chains | 🔵 Verified Partner | 🟢 Starter | `npx skills add okx/plugin-store --skill uniswap-swap-planner` |
| uniswap-swap-integration | Integrate Uniswap swaps into frontends, backends, and smart contracts | 🔵 Verified Partner | 🟡 Standard | `npx skills add okx/plugin-store --skill uniswap-swap-integration` |
| uniswap-liquidity-planner | Plan and generate deep links for creating liquidity positions on Uniswap v2/v3/v4 | 🔵 Verified Partner | 🟢 Starter | `npx skills add okx/plugin-store --skill uniswap-liquidity-planner` |
| uniswap-pay-with-any-token | Pay HTTP 402 challenges using any token via Tempo CLI and Uniswap Trading API | 🔵 Verified Partner | 🟡 Standard | `npx skills add okx/plugin-store --skill uniswap-pay-with-any-token` |
| uniswap-cca-configurator | Configure Continuous Clearing Auction (CCA) smart contract parameters for token distribution | 🔵 Verified Partner | 🟡 Standard | `npx skills add okx/plugin-store --skill uniswap-cca-configurator` |
| uniswap-cca-deployer | Deploy CCA smart contracts using the Factory pattern with CREATE2 | 🔵 Verified Partner | 🟡 Standard | `npx skills add okx/plugin-store --skill uniswap-cca-deployer` |
| uniswap-v4-security-foundations | Security-first guide for building Uniswap v4 hooks: vulnerabilities, audits, and best practices | 🔵 Verified Partner | 🟢 Starter | `npx skills add okx/plugin-store --skill uniswap-v4-security-foundations` |
| uniswap-viem-integration | Integrate EVM blockchains using viem and wagmi for TypeScript/JavaScript apps | 🔵 Verified Partner | 🟢 Starter | `npx skills add okx/plugin-store --skill uniswap-viem-integration` |
| polymarket-agent-skills | Polymarket prediction market integration: trading, market data, streaming, bridge, and gasless transactions | 🔵 Verified Partner | 🟡 Standard | `npx skills add okx/plugin-store --skill polymarket-agent-skills` |
| meme-trench-scanner | Solana meme token automated trading bot with 11 launchpad coverage and 7-layer exit system | ⚪ Community | 🔴 Advanced | `npx skills add okx/plugin-store --skill meme-trench-scanner` |
| top-rank-tokens-sniper | OKX ranking leaderboard sniper with momentum scoring, safety checks, and 6-layer exit system | ⚪ Community | 🔴 Advanced | `npx skills add okx/plugin-store --skill top-rank-tokens-sniper` |
| smart-money-signal-copy-trade | Smart money signal tracker with cost-aware take-profit, safety checks, and copy trading | ⚪ Community | 🔴 Advanced | `npx skills add okx/plugin-store --skill smart-money-signal-copy-trade` |

---

## Browse by Category

| Category | Plugins |
|----------|---------|
| Trading | uniswap-ai, uniswap-swap-planner, uniswap-swap-integration |
| DeFi | uniswap-liquidity-planner, uniswap-pay-with-any-token, uniswap-cca-configurator, uniswap-cca-deployer |
| Prediction | polymarket-agent-skills |
| Dev Tools | uniswap-v4-security-foundations, uniswap-viem-integration, plugin-store |
| Automated Trading | meme-trench-scanner, top-rank-tokens-sniper, smart-money-signal-copy-trade |
| Other | okx-buildx-hackathon-agent-track |

## Browse by Risk Level

| Level | Meaning | Plugins |
|-------|---------|---------|
| 🟢 Starter | Safe to explore. Read-only queries, planning tools, and documentation. No transactions. | plugin-store, okx-buildx-hackathon-agent-track, uniswap-swap-planner, uniswap-liquidity-planner, uniswap-v4-security-foundations, uniswap-viem-integration |
| 🟡 Standard | Executes transactions with user confirmation. Always asks before signing or sending. | uniswap-ai, uniswap-swap-integration, uniswap-pay-with-any-token, uniswap-cca-configurator, uniswap-cca-deployer, polymarket-agent-skills |
| 🔴 Advanced | Automated trading strategies. Requires understanding of financial risks before use. | meme-trench-scanner, top-rank-tokens-sniper, smart-money-signal-copy-trade |

## Trust Indicators

| Badge | Source | Meaning |
|-------|--------|---------|
| 🟢 Official | plugin-store | Developed and maintained by OKX |
| 🔵 Verified Partner | uniswap-\*, polymarket-\* | Published by the protocol team itself |
| ⚪ Community | everything else | Community contribution; review before use |

---

## Documentation

| You are... | Go to... |
|------------|----------|
| Plugin user | [FOR-USERS.md](docs/FOR-USERS.md) |
| Plugin developer | [FOR-DEVELOPERS.md](docs/FOR-DEVELOPERS.md) |
| OKX/Partner team | [FOR-PARTNERS.md](docs/FOR-PARTNERS.md) |
| Reviewing standards | [REVIEW-GUIDELINES.md](docs/REVIEW-GUIDELINES.md) |

## Quick CLI Reference

```bash
# List all available plugins
plugin-store list

# Search by keyword
plugin-store search <keyword>

# Install a plugin
plugin-store install <name>

# Update all installed plugins
plugin-store update --all

# Uninstall a plugin
plugin-store uninstall <name>
```

## Contributing

To submit a plugin, see [FOR-DEVELOPERS.md](docs/FOR-DEVELOPERS.md). The workflow is Fork, develop, then open a Pull Request.

## Security

To report a security issue, please email [security@okx.com](mailto:security@okx.com). Do not open a public issue for security vulnerabilities.

## License

Apache-2.0
