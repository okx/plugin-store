# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Plugin Store — a CLI marketplace for installing/uninstalling/updating Skills and MCP servers across Claude Code, Cursor, and OpenClaw.

## Build & Test

```bash
cargo build                          # Debug build
cargo build --release                # Release build
cargo test                           # Run all tests
cargo test <test_name>               # Run a single test
cargo test --test integration_test   # Run only integration tests
```

## Architecture

### Data Flow

Registry (GitHub JSON) → Cache (~/.plugin-store/cache/) → Plugin selection → Agent detection → Component installation (skill/mcp/binary) → State tracking (~/.plugin-store/installed.json)

### Plugin Model

Each plugin (`registry::models::Plugin`) has three optional components:
- **Skill** — a SKILL.md file downloaded from a GitHub repo, written into the agent's skill directory
- **MCP** — an MCP server config (npm/pip/binary) injected into the agent's settings JSON
- **Binary** — a platform-specific binary downloaded from GitHub releases with optional checksum verification

### Agent Adapter Pattern

`agent::AgentAdapter` trait defines how plugins integrate with each supported agent. Each adapter (`claude_code`, `cursor`, `openclaw`) implements detection, skill directory resolution, and MCP config read/write for its agent's config format. `detect_agents()` probes all adapters and returns which are available.

### State Tracking

`state::InstalledState` persists to `~/.plugin-store/installed.json`. Each installed plugin records which agents it was installed to, with per-agent paths (skill_path, mcp_key, binary_path) for clean uninstall.

### CLI Dispatch

`main.rs` uses clap derive to parse commands → dispatches to `commands::<name>::execute()`. Each command handler orchestrates registry fetching, user interaction (via `dialoguer`), and installer calls. The `install` command supports `--skill-only`, `--mcp-only`, and `--agent` flags to skip interactive selection.

## Key Paths

- Registry cache: `~/.plugin-store/cache/registry.json` (12-hour TTL)
- Installed state: `~/.plugin-store/installed.json`
- Registry source: `ganliux/plugin-store` on GitHub

### Skill Auto-Discovery

When `SkillComponent.path` is absent, `SkillInstaller::discover_all()` fetches the GitHub repo tree and auto-discovers:
- **Skills** — any `SKILL.md` file (root or in a subdirectory); each subdirectory becomes a named skill, with all sibling files installed alongside it.
- **MCP servers** — any `*.mcp.json` file following the Vercel plugin convention (`{ "mcpServers": { "<name>": { "command", "args", "env" } } }`).

### Self-Update

`plugin-store self-update` downloads the latest release binary from `okx/plugin-store` on GitHub, resolves the platform target via `utils::platform::current_target()`, and does an atomic rename-based replacement of the running executable with rollback on failure.

## Key Commands

```bash
plugin-store list                    # List all plugins
plugin-store search <keyword>        # Search plugins
plugin-store info <name>             # Show plugin details
npx skills add okx/plugin-store --skill <name>  # Install via npx (recommended)
plugin-store install <name> --agent claude-code --skill-only  # Install via CLI (alternative)
plugin-store uninstall <name>        # Uninstall
plugin-store update --all            # Update all installed plugins
plugin-store installed               # Show installed plugins
plugin-store registry update         # Refresh registry cache
plugin-store self-update             # Update the plugin-store CLI itself
```
