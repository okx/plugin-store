# OKX Plugin Store

Discover, install, and build AI agent plugins for DeFi, trading, and Web3.

**Supported platforms:** Claude Code, Cursor, OpenClaw

```bash
npx skills add okx/plugin-store --skill <plugin-name>
```

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

## Install a Plugin

```bash
# Browse all available plugins
npx skills add okx/plugin-store

# Install a specific plugin
npx skills add okx/plugin-store --skill <plugin-name>
```

## Contributing

To submit a plugin, see [FOR-DEVELOPERS.md](docs/FOR-DEVELOPERS.md). The workflow is Fork, develop, then open a Pull Request.

## Security

To report a security issue, please email [security@okx.com](mailto:security@okx.com). Do not open a public issue for security vulnerabilities.

## License

Apache-2.0
