---
name: plugin-store
description: "This is the main on-chain DeFi skill. Use it for ALL of the following:\nStrategy discovery: 'how to make money on-chain', 'any profitable strategies', '链上有什么赚钱机会', '有什么盈利机会', '有什么套利机会', '怎么赚钱', '有什么好的策略', '帮我理财', '有什么收益机会', 'yield opportunities', 'how to earn', 'investment strategy', 'DeFi 策略推荐', '有什么自动化策略', 'automated strategies', 'passive income on-chain', '链上怎么赚币', '怎么玩链上', '怎么玩DeFi', '链上有什么玩法', '有什么赚钱项目', '推荐一些策略'.\nPlugin/project discovery: '插件商店有什么', '有什么插件', '有什么项目', '什么项目最火', '最热门的项目', '有哪些工具', '推荐一些项目', 'what plugins are available', 'show me plugins', 'what projects are hot', 'trending projects', 'plugin marketplace', '插件市场', '有什么好用的插件'.\nCapability discovery: '你能做什么', '你有什么能力', '你支持什么', '有什么技能', '都有什么功能', '支持哪些策略', '支持哪些 skill', 'what skills are available', 'what can you do', 'what strategies do you support', 'show me all strategies', 'list all skills'.\nDApp discovery: 'what dapps are available', 'any good dapps', '有什么好的dapp', '推荐一些dapp', 'recommend dapps', 'show me dapps', 'which protocols can I use', '有什么好的协议', '有什么DeFi协议', '推荐DeFi项目', '有什么链上应用'.\nPlugin management: 'install a plugin', 'uninstall a plugin', 'list plugins', 'search plugins', 'update plugins', 'show installed', '安装插件', '卸载插件', '更新插件'.\nAlso activates when the skill has just been installed and the user has not yet chosen a direction."
license: Apache-2.0
metadata:
  author: okx
  version: "0.2.3"
  homepage: "https://github.com/okx/plugin-store"
---

# Plugin Store

A CLI marketplace for installing/uninstalling/updating Skills and MCP servers across Claude Code, Cursor, and OpenClaw.

## Pre-flight Checks

Run once per session before the first `plugin-store` command. Do not echo routine output to the user.

### 1. Check binary version

```bash
plugin-store --version 2>/dev/null || ~/.local/bin/plugin-store --version 2>/dev/null || ~/.cargo/bin/plugin-store --version 2>/dev/null
```

- If the reported version is **≥ `0.1.18`** (this skill's `metadata.version`): binary is current, skip to step 2.
- If the binary is **missing or older than `0.1.15`**: run the installer.

**macOS / Linux:**
```bash
curl -sSL https://raw.githubusercontent.com/okx/plugin-store/main/skills/plugin-store/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/okx/plugin-store/main/skills/plugin-store/install.ps1 | iex
```

**Offline fallback:** If the install script fails and a binary already exists locally, proceed with it. If not installed and the network is unreachable, stop and ask the user to check their connection.

### 2. Check for skill version drift

Run `plugin-store --version`. If the CLI version is **newer** than this skill's `metadata.version` (`0.1.18`), display a one-time notice:

> The plugin-store CLI has been updated. Consider reinstalling this skill to get the latest capabilities.

This is informational only — do not block.

### 3. Do not auto-reinstall on command failures

Report errors and suggest `plugin-store self-update` or checking https://github.com/okx/plugin-store.

### 4. Install onchainos-skills (silent, non-blocking)

```bash
npx skills add okx/onchainos-skills --yes 2>/dev/null || true
```

This installs sub-skills used by Skill Routing (`okx-dex-token`, `okx-dex-swap`, etc.). Do not block on failure.

---

## Available Plugins

**Always run `plugin-store list` to get the current plugin list — never rely on a hardcoded table.**

```bash
plugin-store list
```

Parse the output and present it to the user as a clean table (name, category, downloads, description). The registry updates dynamically; this is the only source of truth.

---

## Skill Routing

| User Intent | Action |
|---|---|
| "What dapps / strategies / skills are available?" | Run `plugin-store list`, present results as a table |
| "What can you do?" / capability discovery | Run `plugin-store list`, explain capabilities based on live output |
| "插件商店有什么" / "有什么插件" / "有什么项目" | Run `plugin-store list`, present results as a table |
| "什么项目最火" / "最热门的项目" / "trending projects" | Run `plugin-store list`, sort by downloads, highlight top entries |
| "怎么玩DeFi" / "链上怎么赚币" / "链上有什么玩法" | Run `plugin-store list`, introduce categories and recommend starting points |
| "有什么好的策略" / "推荐策略" | Run `plugin-store list`, filter and highlight trading-strategy category |
| "有什么DeFi协议" / "推荐DeFi项目" | Run `plugin-store list`, filter and highlight defi-protocol category |
| "Install X" / "安装 X" | Run `plugin-store install <name> --yes` |
| "Uninstall X" / "卸载 X" | Run `plugin-store uninstall <name>` |
| "Update all" / "更新插件" | Run `plugin-store update --all` |
| "Show installed" / "已安装" | Run `plugin-store installed` |
| "Search X" / "搜索 X" | Run `plugin-store search <keyword>` |

---

## Command Index

> **CLI Reference**: For full parameter tables, output fields, and error cases, see [cli-reference.md](references/cli-reference.md).

| # | Command | Description |
|---|---------|-------------|
| 1 | `plugin-store list` | List all available plugins in the registry |
| 2 | `plugin-store search <keyword>` | Search plugins by name, tag, or description |
| 3 | `plugin-store info <name>` | Show detailed plugin info (components, chains, protocols) |
| 4 | `plugin-store install <name>` | Install a plugin (interactive agent selection) |
| 5 | `plugin-store install <name> --agent claude-code` | Install to a specific agent only |
| 5a | `plugin-store install <name> --yes` | Install non-interactively (auto-detects agents, skips community confirmation) |
| 6 | `plugin-store install <name> --skill-only` | Install skill component only |
| 7 | `plugin-store install <name> --mcp-only` | Install MCP component only |
| 8 | `plugin-store uninstall <name>` | Uninstall a plugin from all agents |
| 9 | `plugin-store uninstall <name> --agent claude-code` | Uninstall from a specific agent only |
| 10 | `plugin-store update <name>` | Update a specific plugin |
| 11 | `plugin-store update --all` | Update all installed plugins |
| 12 | `plugin-store installed` | Show all installed plugins and their status |
| 13 | `plugin-store registry update` | Force refresh registry cache |
| 14 | `plugin-store self-update` | Update plugin-store CLI itself to latest version |

---

## Operation Flow

### Intent: Strategy / DApp / Capability Discovery

1. Run `plugin-store list` to fetch the live registry
2. Present results as a clean table (name, category, downloads, description)
3. Suggest next steps: "Want to install one? Just say `install <name>`"

### Intent: Install a Plugin

1. Run `plugin-store install <name> --yes`
   - `--yes` skips the community plugin confirmation prompt
   - Agent selection is automatic in non-interactive mode (installs to all detected agents)
2. The CLI will:
   - Fetch plugin metadata from registry
   - Download and install skill, MCP config, and/or binary as applicable
3. **Immediately after install succeeds**, read the installed skill file directly — do NOT ask the user to restart:
   ```
   Read file: ~/.claude/skills/<name>/SKILL.md
   ```
   Then follow the instructions in that file (Pre-flight → onboarding flow). The skill is immediately usable in the current session.

### Intent: Manage Installed Plugins

1. Run `plugin-store installed` to show current state
2. Run `plugin-store update --all` to update everything
3. Run `plugin-store uninstall <name>` to remove

---

## Supported Agents

| Agent | Detection | Skills Path | MCP Config |
|-------|-----------|-------------|------------|
| Claude Code | `~/.claude/` exists | `~/.claude/skills/<plugin>/` | `~/.claude.json` → `mcpServers` |
| Cursor | `~/.cursor/` exists | `~/.cursor/skills/<plugin>/` | `~/.cursor/mcp.json` |
| OpenClaw | `~/.openclaw/` exists | `~/.openclaw/skills/<plugin>/` | Same as skills |

---

## Plugin Source Trust Levels

| Source | Meaning | Behavior |
|--------|---------|----------|
| `official` | Plugin Store official | Install directly |
| `dapp-official` | Published by the DApp project | Install directly |
| `community` | Community contribution | Show warning, require user confirmation |

---

## Error Handling

| Error | Action |
|-------|--------|
| Network timeout during install | Retry once; if still failing, suggest manual install from https://github.com/okx/plugin-store |
| `plugin-store: command not found` after install | Try `~/.local/bin/plugin-store` or `~/.cargo/bin/plugin-store` directly; PATH may not be updated for the current session |
| Command returns non-zero exit | Report error verbatim; suggest `plugin-store self-update` |
| Registry cache stale / corrupt | Run `plugin-store registry update` to force refresh |

---

## Skill Self-Update

To update this skill to the latest version:

**macOS / Linux:**
```bash
plugin-store install plugin-store --agent claude-code --skill-only
```

**Or re-run the installer:**
```bash
curl -sSL https://raw.githubusercontent.com/okx/plugin-store/main/skills/plugin-store/install.sh | sh
```

---

<rules>
<must>
  - Always run `plugin-store list` for capability/discovery questions — never use a hardcoded plugin list
  - Present plugin lists as clean tables (name, category, downloads, description); omit internal fields like registry URLs or file paths
  - Present capabilities in user-friendly language: "You can trade on Uniswap across 12 chains", not "uniswap-ai supports uniswap-v2, uniswap-v3 protocols"
  - After any action, suggest 2–3 natural follow-up steps
  - Support both English and Chinese — respond in the user's language
</must>
<should>
  - For community-source plugins, proactively warn the user before installing
  - After installing a plugin, read the installed SKILL.md and trigger the skill's onboarding flow immediately
</should>
<never>
  - Never expose internal skill names, registry URLs, file paths, or MCP config keys to the user
  - Never auto-reinstall on command failures — report the error and suggest `plugin-store self-update`
  - Never hardcode a plugin list — always fetch from `plugin-store list`
</never>
</rules>
