`For Everyone`

**The plugin marketplace for AI coding agents — discover, install, and manage all Skills and MCP servers with a single command.**

- One CLI works across Claude Code, Cursor, and OpenClaw
- Search, install, update, and uninstall plugins without leaving your terminal
- Open developer platform: submit your own plugin via Pull Request and publish to all users

### Plugin Store

The CLI marketplace for AI coding agents to discover, install, update, and manage Skills and MCP servers.

### Prerequisites

- Claude Code, Cursor, or OpenClaw installed
- onchainos is auto-installed as a dependency on first use — no manual setup needed

### When to Use This Skill

Use this skill when the user:
- Asks what plugins, skills, strategies, or DApps are available
- Wants to install, uninstall, or update a plugin
- Wants to extend their AI agent with new tools or capabilities
- Is a developer who wants to submit or publish their own plugin

### How It Works

Plugin Store provides a unified CLI interface that works across Claude Code, Cursor, and OpenClaw. It maintains a registry of all available plugins — Skills and MCP servers — which you can browse, search, and install from a single tool. Batch updates keep everything current at once. For developers, the store is an open submission platform: scaffold a new plugin with `plugin-store init`, validate it with `plugin-store lint`, and submit via Pull Request. Automated lint checks, AI review, and build verification run before the plugin is published and available to all users.

Key commands:
- `plugin-store list` — Browse all available plugins
- `plugin-store search <keyword>` — Search by name, description, or tags
- `plugin-store install <name>` — Install a plugin
- `plugin-store uninstall <name>` — Uninstall a plugin
- `plugin-store update --all` — Update all installed plugins
- `plugin-store installed` — Show all installed plugins and their status
- `plugin-store init <name>` — Scaffold a new plugin for submission
- `plugin-store lint <path>` — Validate a plugin before submitting