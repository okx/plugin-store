# Plugin Store

Plugin Store is a **Skills and MCP marketplace** for AI coding assistants. It lets agents discover, install, update, and uninstall plugins — including on-chain trading strategies, DeFi protocol integrations, and developer tools — across Claude Code, Cursor, and OpenClaw.

## Install

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/okx/plugin-store/main/install-local.sh | bash

# Or update an existing installation
plugin-store self-update
```

## Commands

### Discovery

```bash
# List all available plugins
plugin-store list

# Search by keyword
plugin-store search <keyword>

# Show details for a plugin (description, components, install command)
plugin-store info <name>
```

### Install & Uninstall

```bash
# Install a plugin (interactive agent selection)
plugin-store install <name>

# Install to a specific agent
plugin-store install <name> --agent claude-code

# Install non-interactively (skip prompts)
plugin-store install <name> --yes

# Install skill component only (no binary/npm/pip)
plugin-store install <name> --skill-only

# Uninstall from all agents
plugin-store uninstall <name>
```

### Update

```bash
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

## Plugin Distribution Model

Plugins are distributed differently based on their language:

| Language | Distribution | User Install Size |
|----------|-------------|-------------------|
| **Rust** | GitHub Release binary | ~1-20 MB |
| **Go** | GitHub Release binary | ~2-15 MB |
| **TypeScript** | `npm install -g` from source | ~KB |
| **Node.js** | `npm install -g` from source | ~KB |
| **Python** | `pip install` from source | ~KB |
| **Pure Skill** | SKILL.md download | ~KB |

Binary plugins (Rust/Go) are compiled by our CI and published as GitHub Releases.
TS/Node/Python plugins are installed directly from the developer's source repo via npm/pip.

## Supported Agents

| Agent | Detection |
|-------|-----------|
| Claude Code | `~/.claude/` exists |
| Cursor | `~/.cursor/` exists |
| OpenClaw | `~/.openclaw/` exists |

## Plugin Sources

| Source | Meaning |
|--------|---------|
| `official` | Developed and maintained by Plugin Store |
| `dapp-official` | Published by the DApp project itself |
| `community-developer` | Community contribution — install prompt includes a warning |

## Contributing

To submit a community plugin, visit [okx/plugin-store-community](https://github.com/okx/plugin-store-community) and follow the [Plugin Development Guide](https://github.com/okx/plugin-store-community/blob/main/PLUGIN_DEVELOPMENT_GUIDE.md).

## Risk Warning

> **All trading strategies involve significant financial risk.** Always validate with `--dry-run` before going live. Never deploy more capital than you can afford to lose entirely.

## License

Apache-2.0
