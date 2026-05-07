# Plugin Store CLI Reference

Complete command reference with parameters, output fields, and usage examples.

---

## 1. `plugin-store list`

List all available plugins in the registry.

**Parameters:** None

**Output:**

| Field | Type | Description |
|-------|------|-------------|
| Name | string | Plugin name (unique identifier) |
| Version | string | Current version in registry |
| Source | enum | Trust level: `official`, `dapp-official`, `community` |
| Description | string | One-line description |

**Example:**

```bash
$ plugin-store list

Name                                Version    Source          Description
------------------------------------------------------------------------------------------
uniswap-ai                          1.7.0      dapp-official   AI-powered Uniswap developer tools...

2 plugins available.
```

---

## 2. `plugin-store search <keyword>`

Search plugins by keyword. Matches against name, tags, description, and category.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `keyword` | string | Yes | Search term |

**Output:** Same table format as `list`, filtered by keyword match.

**Example:**

```bash
$ plugin-store search trading

Name                                Version    Source          Description
------------------------------------------------------------------------------------------
uniswap-ai                          1.7.0      dapp-official   AI-powered Uniswap developer tools...

2 plugins found.
```

```bash
$ plugin-store search uniswap

Name                                Version    Source          Description
------------------------------------------------------------------------------------------
uniswap-ai                          1.7.0      dapp-official   AI-powered Uniswap developer tools...

1 plugins found.
```

---

## 3. `plugin-store info <name>`

Show detailed plugin metadata including components, chains, and protocols.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Exact plugin name |

**Output:**

| Field | Type | Description |
|-------|------|-------------|
| Name | string | Plugin name |
| Version | string | Current version |
| Description | string | Full description |
| Author | string | Author name and GitHub URL |
| Category | string | Plugin category |
| Source | enum | Trust level |
| Tags | string[] | Comma-separated tags |
| Components | flags | Which components are available: Skill ✔, MCP ✔ (type), Binary ✔ |
| Extra | object | Chains, protocols, risk level (only if plugin has extra metadata) |

**Example:**

```bash
$ plugin-store info uniswap-ai

Name: uniswap-ai
Version: 1.7.0
Description: AI-powered Uniswap developer tools: trading, hooks, drivers, and on-chain analysis across V2/V3/V4
Author: Uniswap (https://github.com/Uniswap/uniswap-ai)
Category: defi-protocol
Source: dapp-official
Tags: uniswap, trading, hooks, v2, v3, v4, multi-chain

Components:
  ✔ Skill

Extra:
  Chains: ethereum, base, arbitrum, optimism, polygon, bnb, avalanche, celo, blast, zora, worldchain, unichain
  Protocols: uniswap-v2, uniswap-v3, uniswap-v4, universal-router
  Risk Level: medium
```

**Error — plugin not found:**

```
Plugin 'foo' not found. Run `plugin-store search <keyword>` to find plugins.
```

---

## 4. `plugin-store install <name> [OPTIONS]`

Install a plugin to one or more agents. Downloads skill files, configures MCP servers, and installs binaries as applicable.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Plugin name to install |
| `--skill-only` | flag | No | Install skill component only (skip MCP and binary) |
| `--mcp-only` | flag | No | Install MCP component only (skip skill and binary) |
| `--agent <id>` | string | No | Target agent ID. Skip interactive selection. Valid: `claude-code`, `cursor`, `openclaw` |

**Behavior:**

1. Fetches plugin metadata from registry
2. If `type == "community"` and not already installed → shows warning, asks confirmation
3. If `--agent` not specified → detects installed agents, prompts multi-select
4. Installs components per agent:
   - **Skill**: downloads SKILL.md from GitHub → writes to `~/<agent>/skills/<plugin>/SKILL.md`
   - **MCP**: writes config entry to agent's settings file
   - **Binary**: downloads platform-specific binary from GitHub releases, verifies checksum
5. Records install state to `~/.plugin-store/installed.json`

**Example:**

```bash
# Interactive install (prompts for agent selection)
$ plugin-store install uniswap-ai

# Install to Claude Code only
$ plugin-store install uniswap-ai --agent claude-code

# Install skill only, skip MCP/binary
$ plugin-store install uniswap-ai --skill-only

# Install MCP only, skip skill/binary
$ plugin-store install uniswap-ai --mcp-only

# Combine: skill-only + specific agent
$ plugin-store install uniswap-ai --skill-only --agent claude-code
```

**Output:**

```
Installing uniswap-ai 1.7.0...
  ✔ Skill installed → ~/.claude/skills/uniswap-ai/ (Claude Code)
Done!
```

**Error — plugin not found:**

```
Plugin 'foo' not found. Run `plugin-store search <keyword>` to find plugins.
```

**Error — no agents selected:**

```
No agents selected.
```

**Error — unknown agent ID:**

```
Unknown agent 'foo'. Valid: claude-code, cursor, openclaw
```

---

## 5. `plugin-store uninstall <name> [OPTIONS]`

Uninstall a plugin. Removes skill files, MCP config entries, and binaries.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Plugin name to uninstall |
| `--agent <id>` | string | No | Only remove from this agent. Valid: `claude-code`, `cursor`, `openclaw` |

**Behavior:**

- Without `--agent`: removes from ALL agents the plugin was installed to, then removes state record
- With `--agent`: removes from that agent only. If no agents remain, removes state record entirely

**Example:**

```bash
# Uninstall from all agents
$ plugin-store uninstall uniswap-ai

# Uninstall from Claude Code only
$ plugin-store uninstall uniswap-ai --agent claude-code
```

**Output:**

```
Uninstalling uniswap-ai...
  ✔ Skill removed from Claude Code
  ✔ State updated
Done!
```

**Error — not installed:**

```
Plugin 'foo' is not installed.
```

---

## 6. `plugin-store update <name>`

Update a specific installed plugin to the latest registry version.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | No | Plugin name. Omit when using `--all` |
| `--all` | flag | No | Update all installed plugins |

**Behavior:**

1. Force-refreshes registry cache
2. Compares installed version vs registry version for target plugin(s)
3. If update available: shows `oldVersion -> newVersion`, re-installs to same agents
4. If `--all` with multiple updates: asks confirmation before proceeding
5. If already up to date: prints `(up to date)`

**Example:**

```bash
# Update a specific plugin
$ plugin-store update uniswap-ai

# Update all installed plugins
$ plugin-store update --all
```

**Output — update available:**

```
Updates available:
  uniswap-ai: 1.5.0 -> 1.7.0
Installing uniswap-ai 1.7.0...
  ✔ Skill installed → ~/.claude/skills/uniswap-ai/ (Claude Code)
Done!
```

**Output — up to date:**

```
  uniswap-ai 1.7.0 (up to date)
All plugins are up to date.
```

**Output — nothing installed:**

```
No plugins installed.
```

**Output — missing arguments:**

```
Specify a plugin name or use --all.
```

---

## 7. `plugin-store installed`

Show all installed plugins with their agents and components.

**Parameters:** None

**Output:**

| Field | Type | Description |
|-------|------|-------------|
| Name | string | Plugin name |
| Version | string | Installed version |
| Agents | string[] | Comma-separated agent IDs (e.g. `claude-code, cursor`) |
| Components | string[] | Comma-separated installed components (e.g. `skill, mcp, binary`) |

**Example:**

```bash
$ plugin-store installed

Name                                Version    Agents               Components
-------------------------------------------------------------------------------------
uniswap-ai                          1.7.0      claude-code          skill

2 plugins installed.
```

**Output — nothing installed:**

```
No plugins installed.
```

---

## 8. `plugin-store registry update`

Force refresh the local registry cache from remote GitHub.

**Parameters:** None

**Behavior:**

- Fetches latest `registry.json` from GitHub raw URL
- Overwrites local cache at `~/.plugin-store/cache/registry.json`
- Resets the 12-hour cache TTL

**Example:**

```bash
$ plugin-store registry update

Refreshing registry...
  OK Registry updated. 2 plugins available.
```

---

## 9. `plugin-store self-update`

Update the plugin-store CLI binary itself to the latest GitHub release.

**Parameters:** None

**Behavior:**

1. Fetches latest release from GitHub API
2. Compares latest tag version with current binary version (`CARGO_PKG_VERSION`)
3. If already up to date: prints confirmation and exits
4. If update available: detects current platform target, finds matching binary asset in release
5. Downloads binary, verifies SHA256 checksum against `checksums.txt` (if present in release)
6. Atomically replaces current binary: `current → .bak`, `new → current`; removes `.bak` on success; rolls back on failure

**Example:**

```bash
$ plugin-store self-update

Checking for updates...
  Current version: 0.1.0
  Latest version:  0.2.0

  Update available: 0.1.0 → 0.2.0
  Downloading plugin-store-aarch64-apple-darwin...
  Checksum verified ✓

Updated!  0.1.0 → 0.2.0
```

**Output — already up to date:**

```
Checking for updates...
  Current version: 0.2.0
  Latest version:  0.2.0

Already up to date!
```

**Output — no releases on GitHub:**

```
  No releases found on GitHub. You're on the latest build.
```

**Error — platform not supported:**

```
No binary found for platform 'riscv64-unknown-linux-gnu'. Available assets: plugin-store-x86_64-apple-darwin, ...
```

**Error — checksum mismatch:**

```
Checksum verification failed.
  Expected: abc123...
  Got:      def456...
```

---

## Agent IDs

Valid values for `--agent` parameter:

| ID | Agent | Detection |
|----|-------|-----------|
| `claude-code` | Claude Code | `~/.claude/` exists |
| `cursor` | Cursor | `~/.cursor/` exists |
| `openclaw` | OpenClaw | `~/.openclaw/` exists |

---

## File Paths

| Path | Description |
|------|-------------|
| `~/.plugin-store/cache/registry.json` | Cached registry (12h TTL) |
| `~/.plugin-store/installed.json` | Installed plugin state |
| `~/.plugin-store/bin/` | Default binary install directory |
| `~/.claude/skills/<plugin>/SKILL.md` | Skill file for Claude Code |
| `~/.claude/settings.json` → `mcpServers` | MCP config for Claude Code |
| `~/.cursor/skills/<plugin>/SKILL.md` | Skill file for Cursor |
| `~/.cursor/mcp.json` → `mcpServers` | MCP config for Cursor |
| `~/.openclaw/skills/<plugin>/SKILL.md` | Skill file for OpenClaw |

---

## Common Workflows

### Install a plugin end-to-end

```bash
plugin-store list                              # Browse available plugins
plugin-store info uniswap-ai                   # Check details
plugin-store install uniswap-ai --agent claude-code   # Install
plugin-store installed                         # Verify
```

### Update everything

```bash
plugin-store registry update                   # Refresh registry first
plugin-store update --all                      # Update all installed
```

### Remove a plugin from one agent

```bash
plugin-store uninstall uniswap-ai --agent cursor   # Remove from Cursor only
plugin-store installed                              # Verify remaining
```

### Search and install

```bash
plugin-store search prediction                 # Find prediction market plugins
```
