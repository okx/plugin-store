# Plugin Store

Plugin Store is a **Skills and MCP marketplace** for AI coding assistants. It lets agents discover, install, update, and uninstall plugins — including on-chain trading strategies, DeFi protocol integrations, and developer tools — across Claude Code, Cursor, and OpenClaw.

## Install the Skill

```bash
npx skills add okx/plugin-store
```

Once installed, the agent gains full access to the plugin marketplace through natural language or the commands below.

## Commands

### Discovery

```bash
# List all available plugins
plugin-store list

# Search by keyword
plugin-store search <keyword>

# Show details for a plugin (description, chains, protocols, components)
plugin-store info <name>
```

### Install & Uninstall

```bash
# Install a plugin (interactive agent selection)
plugin-store install <name>

# Install to a specific agent
plugin-store install <name> --agent claude-code

# Install non-interactively (skip prompts, auto-detect agents)
plugin-store install <name> --yes

# Install skill component only (no binary)
plugin-store install <name> --skill-only

# Install MCP component only
plugin-store install <name> --mcp-only

# Uninstall from all agents
plugin-store uninstall <name>

# Uninstall from a specific agent
plugin-store uninstall <name> --agent claude-code
```

### Update

```bash
# Update a specific plugin
plugin-store update <name>

# Update all installed plugins
plugin-store update --all

# Update the plugin-store CLI itself
plugin-store self-update
```

### Manage

```bash
# Show all installed plugins and their status
plugin-store installed

# Force refresh the registry cache
plugin-store registry update
```

## Supported Agents

| Agent | Detection |
|-------|-----------|
| Claude Code | `~/.claude/` exists |
| Cursor | `~/.cursor/` exists |
| OpenClaw | `~/.openclaw/` exists |

## Plugin Source Trust Levels

| Source | Meaning |
|--------|---------|
| `official` | Developed and maintained by Plugin Store |
| `dapp-official` | Published by the DApp project itself |
| `community` | Community contribution — install prompt includes a warning |

## Official Strategies

After installing a strategy plugin, the corresponding binary and skill are available immediately. Each strategy requires [onchainos](https://web3.okx.com/zh-hans/onchainos/dev-docs/home/install-your-agentic-wallet) ≥ 2.0.0.

### ranking-sniper

Monitors the OKX DEX trending ranking board every 10s. Applies a 25-point safety filter, scores momentum (0–125 pts), and executes trades with a 6-layer exit system.

```bash
strategy-ranking-sniper start --budget 0.5 --per-trade 0.05
strategy-ranking-sniper status
strategy-ranking-sniper sell-all
```

### memepump-scanner

Scans pump.fun MIGRATED tokens every 10s. Applies a 22-point safety filter (dev rug zero-tolerance, bundler checks), detects 3-signal momentum, and trades with an 8-layer exit system.

```bash
strategy-memepump-scanner start
strategy-memepump-scanner analyze
strategy-memepump-scanner status
```

### signal-tracker

Polls OKX Signal API every 20s for SmartMoney / KOL / Whale buy signals. Applies a 17-point safety filter with cost-aware TP/SL, trailing stop, and session risk controls.

```bash
strategy-signal-tracker start
strategy-signal-tracker status
strategy-signal-tracker report
```

### hyperliquid

Perpetual futures and spot trading on Hyperliquid. 11 commands covering market data (no auth required) and order management (requires `EVM_PRIVATE_KEY`).

```bash
dapp-hyperliquid markets
dapp-hyperliquid price BTC
dapp-hyperliquid buy --symbol BTC --size 0.001 --price 70000 --leverage 10
dapp-hyperliquid positions
```

## Risk Warning

> **All trading strategies involve significant financial risk.** Always validate with `--dry-run` before going live. Never deploy more capital than you can afford to lose entirely.

## Contributing

To submit a community plugin, open a PR adding an entry to [`registry.json`](registry.json). See existing entries for the required schema.

## License

Apache-2.0
