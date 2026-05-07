# Plugin Development & Submission Guide

> This guide walks you through developing a Plugin for the Plugin Store ecosystem
> and packaging it for review. By the end, you will have a working Plugin that
> users can install via `npx skills add okx/plugin-store --skill <name>`.

---

## Table of Contents

1. [What is a Plugin?](#1-what-is-a-plugin)
2. [Before You Start](#2-before-you-start)
3. [Quick Start (7 Steps)](#3-quick-start-7-steps)
4. [Plugin Structure](#4-plugin-structure)
5. [Writing SKILL.md](#5-writing-skillmd)
6. [Writing SUMMARY.md](#writing-summarymd)
7. [Submitting Plugins with Source Code (Binary)](#6-submitting-plugins-with-source-code-binary)
8. [Three Submission Modes](#7-three-submission-modes)
9. [Onchain OS Integration](#8-onchainos-integration)
10. [Review Process](#9-review-process)
11. [Risk Levels](#10-risk-levels)
12. [Rules & Restrictions](#11-rules--restrictions)
13. [FAQ](#12-faq)

---

## 1. What is a Plugin?

A Plugin has one required core: **SKILL.md** -- a markdown document that teaches
AI agents how to perform tasks. Optionally, it can also include a **Binary**
(compiled from your source code by our CI).

**SKILL.md is always the entry point.** Even if your Plugin includes a binary,
the Skill tells the AI agent what tools are available and when to use them.

### Two Types of Plugins

```
Type A: Skill-Only (most common, any developer)
────────────────────────────────────────────────
  SKILL.md → instructs AI → calls CLI tools / APIs
                           + queries external data (free)

Type B: Skill + Binary (any developer, source code compiled by our CI)
────────────────────────────────────────────────
  SKILL.md → instructs AI → calls CLI tools / APIs
                           + calls your binary tools
                           + queries external data (free)

  Your source code (in your GitHub repo)
    → our CI compiles it
    → users install our compiled artifact
```

Plugins are **not limited to Web3**. You can build analytics dashboards, developer
utilities, trading strategies, DeFi integrations, security scanners, NFT tools,
or anything else that benefits from AI-agent orchestration.

| I want to... | Type |
|--------------|------|
| Use AI agents to accomplish specific tasks via natural language, e.g. invoke CLI tools to create trading strategies | Skill-Only |
| Build a CLI tool alongside a Skill | Skill + Binary (submit source code, we compile) |

### Supported Languages for Binary Plugins

| Language | Build Tool | Distribution |
|----------|-----------|-------------|
| Rust | `cargo build --release` | Native binary |
| Go | `go build` | Native binary |
| TypeScript | Bun | Bun global install |
| Node.js | Bun | Bun global install |
| Python | `pip install` | pip package |

### What Makes a Good Plugin

- **Useful** -- solves a real problem or automates a tedious workflow
- **Safe** -- does not handle private keys directly, declares all external API calls, includes risk disclaimers where appropriate
- **Well-documented** -- clear SKILL.md with concrete examples, error handling, and pre-flight checks so the AI agent can operate from a blank environment

---

## 2. Before You Start

### Prerequisites

- **Git** and a **GitHub account**
- **onchainos CLI** installed (recommended for testing your commands):
  ```
  npx skills add okx/onchainos-skills
  ```
  After installation, if `onchainos` is not found, add it to your PATH:
  ```
  export PATH="$HOME/.local/bin:$PATH"
  ```
- Basic understanding of the domain your Plugin covers

> **Note:** The plugin-store CLI is optional for local linting. Users install
> your finished Plugin via `npx skills add okx/plugin-store --skill <name>` --
> no CLI installation required on their end.

### Key Rules

1. **On-chain write operations can use onchainos CLI or any other approach.** Onchain OS provides
   wallet signing, transaction broadcasting, swap execution, contract calls, and
   token approvals. You are free to query external data sources (third-party APIs,
   market data providers, analytics services, etc.).
2. Onchain OS is **optional**. Plugins can freely use any on-chain technology.
   Non-blockchain Plugins do not need it at all.

---

## 3. Quick Start (7 Steps)

This walkthrough creates a minimal Skill-only Plugin and submits it.

### Step 2: Create Your Plugin Directory

```
mkdir -p skills/my-plugin
```

### Step 3: Create plugin.yaml

```
cat > skills/my-plugin/plugin.yaml << 'EOF'
schema_version: 1
name: my-plugin
version: "1.0.0"
description: "What my plugin does in one sentence"
author:
  name: "Your Name"
  github: "your-github-username"
license: MIT
category: utility
tags:
  - keyword1
  - keyword2

components:
  skill:
    dir: "."

api_calls: []
EOF
```

### Step 4: Create .claude-plugin/plugin.json

This file allows Claude to recognize your Plugin as a Skill. Without it, the AI agent cannot discover or install your Plugin.

```
mkdir -p skills/my-plugin/.claude-plugin
cat > skills/my-plugin/.claude-plugin/plugin.json << 'EOF'
{
  "name": "my-plugin",
  "description": "What my plugin does in one sentence",
  "version": "1.0.0",
  "author": {
    "name": "Your Name"
  },
  "license": "MIT",
  "keywords": ["keyword1", "keyword2"]
}
EOF
```

> **Important**: The `name`, `description`, and `version` fields must match your `plugin.yaml` exactly.

### Step 5: Create SKILL.md

Create `skills/my-plugin/SKILL.md` with the following content:

```markdown
---
name: my-plugin
description: "What my plugin does in one sentence"
version: "1.0.0"
author: "Your Name"
tags:
  - keyword1
  - keyword2
---

# My Plugin

## Overview

This skill enables the AI agent to [describe what it does in 2-3 sentences].

## Pre-flight Checks

Before using this skill, ensure:

1. [List any prerequisites, e.g. API keys, CLI tools]

## Commands

### Command Name

`example-command --flag value`

**When to use**: Describe when the AI agent should invoke this command.
**Output**: Describe what the command returns.
**Example**: `example-command --flag "real-value"`

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| "Something failed" | Why it happens | What the AI agent should do |

## Security Notices

- This plugin is read-only and does not perform transactions.
```

### Step 6: Validate Locally

```
cd /path/to/plugin-store
plugin-store lint skills/my-plugin
```

If everything passes:

```
Linting skills/my-plugin...

  Plugin 'my-plugin' passed all checks!
```

---

## 4. Plugin Structure

### Directory Layout

```
skills/my-plugin/
├── .claude-plugin/
│   └── plugin.json      # Required -- Claude Skill registration metadata
├── plugin.yaml          # Required -- plugin metadata and manifest
├── SKILL.md             # Required -- skill definition for the AI agent
├── SUMMARY.md           # Required -- English summary (Overview, Prerequisites, Quick Start)
├── src/                 # Optional -- source code for binary plugins
│   └── main.rs          #   Rust example (or main.go, main.ts, main.py)
├── Cargo.toml           # Optional -- build config (or go.mod, package.json, pyproject.toml)
├── scripts/             # Optional -- Python scripts, shell scripts
│   ├── bot.py
│   └── config.py
├── assets/              # Optional -- HTML dashboards, images
│   └── dashboard.html
├── references/          # Optional -- extra documentation for the AI agent
│   └── api-reference.md
├── README.md            # Optional -- developer-facing documentation
└── LICENSE              # Recommended -- SPDX-compatible license file
```

`.claude-plugin/plugin.json`, `plugin.yaml`, `SKILL.md`, and `SUMMARY.md` are all **required**. Everything else is optional.

### .claude-plugin/plugin.json

This file follows the [Claude Skill architecture](https://docs.anthropic.com/en/docs/claude-code) and is required for Plugin registration. It must be consistent with your `plugin.yaml`.

```json
{
  "name": "my-plugin",
  "description": "What my plugin does in one sentence",
  "version": "1.0.0",
  "author": {
    "name": "Your Name",
    "email": "you@example.com"
  },
  "homepage": "https://github.com/your-username/your-repo",
  "repository": "https://github.com/your-username/your-repo",
  "license": "MIT",
  "keywords": ["keyword1", "keyword2"]
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Must match `plugin.yaml` name |
| `description` | Yes | Must match `plugin.yaml` description |
| `version` | Yes | Must match `plugin.yaml` version (semver) |
| `author` | Yes | Name and optional email |
| `license` | Yes | SPDX identifier (MIT, Apache-2.0, etc.) |
| `keywords` | No | Searchable tags |
| `homepage` | No | Project homepage URL |
| `repository` | No | Source code URL |

### plugin.yaml Reference

Examples organized by submission mode. Choose the one that matches your use case.

#### Mode A: Skill-Only (no source code)

The simplest form — just a SKILL.md that orchestrates existing CLI tools and commands.

```yaml
schema_version: 1
name: sol-price-checker
version: "1.0.0"
description: "Query real-time token prices on Solana with market data and trend analysis"
author:
  name: "Your Name"
  github: "your-github-username"
license: MIT
category: analytics
tags:
  - price
  - solana

components:
  skill:
    dir: "."

api_calls: []
```

#### Mode A: Skill + Source Code (compiled binary, local source)

Source code lives directly in the plugin directory. CI compiles from here — no external repo needed.

```yaml
schema_version: 1
name: my-rust-tool
version: "1.0.0"
description: "A Rust CLI tool with source code included in the submission"
author:
  name: "Your Name"
  github: "your-username"
license: MIT
category: utility
tags:
  - rust
  - onchainos

components:
  skill:
    dir: "."

build:
  lang: rust            # rust | go | typescript | node | python
  binary_name: my-tool  # compiled output name

api_calls: []
```

Directory structure:
```
skills/my-rust-tool/
├── .claude-plugin/plugin.json
├── plugin.yaml
├── SKILL.md
├── Cargo.toml
├── Cargo.lock
├── src/
│   └── main.rs
└── LICENSE
```

#### Mode B: External Repository (source code in your own repo)

Your source code stays in your own GitHub repo, pinned to a specific commit. CI clones your repo at the pinned commit to compile the binary.

**Important: Your PR must include the following files in `skills/<name>/`:**

| File | Required | Notes |
|------|:--------:|-------|
| `plugin.yaml` | Yes | Must include `build.source_repo`, `build.source_commit`, `build.source_dir` |
| `SKILL.md` | **Yes** | Must contain full command documentation. CI cannot fetch SKILL.md from your remote repo — it must be in the PR. CI will inject pre-flight blocks automatically. |
| `SUMMARY.md` | **Yes** | English summary with Overview, Prerequisites, and Quick Start sections. See [Writing SUMMARY.md](#writing-summarymd). |
| `LICENSE` | Yes | SPDX-compatible license file |
| `.claude-plugin/plugin.json` | Yes | Standard Claude skill metadata |

> **Why must SKILL.md be in the PR?** CI needs to inject pre-flight installation blocks (onchainos CLI, binary download, telemetry) into SKILL.md and push them back to your PR branch. This is not possible if SKILL.md lives in your remote repo. Additionally, SKILL.md must be reviewable in the PR diff.

```
skills/defi-yield-optimizer/
├── .claude-plugin/
│   └── plugin.json
├── plugin.yaml
├── SKILL.md              ← Full command docs (required in PR)
├── SUMMARY.md            ← English summary (required in PR)
└── LICENSE
```

```yaml
schema_version: 1
name: defi-yield-optimizer
version: "1.0.0"
description: "Optimize DeFi yield across protocols with custom analytics"
author:
  name: "DeFi Builder"
  github: "defi-builder"
license: MIT
category: defi-protocol
tags:
  - defi
  - yield

components:
  skill:
    dir: "."              # SKILL.md is in the plugin directory, not remote

build:
  lang: rust
  source_repo: "defi-builder/yield-optimizer"    # Your GitHub repo
  source_commit: "a1b2c3d4e5f6789012345678901234567890abcd"  # Pinned commit
  source_dir: "defi-yield"                       # Subdirectory in your repo (for monorepos)
  binary_name: defi-yield

api_calls:
  - "api.defillama.com"
```

The `build.source_dir` field tells CI which subdirectory in your repo contains the `Cargo.toml` (or `go.mod`). This is important for **monorepos** — CI will only compile from that subdirectory, not the entire repository.

#### Field-by-Field Reference

| Field | Required | Description | Rules |
|-------|----------|-------------|-------|
| `schema_version` | Yes | Schema version | Always `1` |
| `name` | Yes | Plugin name | Lowercase `[a-z0-9-]`, 2-40 chars, no consecutive hyphens |
| `version` | Yes | Plugin version | Semantic versioning `x.y.z` (quoted string) |
| `description` | Yes | One-line summary | Under 200 characters |
| `author.name` | Yes | Author display name | Your name or organization |
| `author.github` | Yes | GitHub username | Must match PR author |
| `author.email` | No | Contact email | Used for security notifications |
| `license` | Yes | License identifier | SPDX format: `MIT`, `Apache-2.0`, `GPL-3.0`, etc. |
| `category` | Yes | Plugin category | One of: `trading-strategy`, `strategy`, `defi-protocol`, `analytics`, `utility`, `security`, `wallet`, `nft` |
| `tags` | No | Search keywords | Array of strings |
| `type` | No | Author type | `"official"`, `"dapp-official"`, `"community-developer"` |
| `github_link` | No | Project GitHub URL | URL, displayed in the marketplace |
| `components.skill.dir` | Mode A | Skill directory path | Relative path within the plugin directory (use `"."` for the plugin root) |
| `components.skill.repo` | Mode B | External repository | Format: `owner/repo` |
| `components.skill.commit` | Mode B | Pinned commit | Full 40-character hex SHA |
| `build.lang` | Binary only | Source language | `rust` / `go` / `typescript` / `node` / `python` |
| `build.source_repo` | Binary only | Source code repo | Format: `owner/repo` |
| `build.source_commit` | Binary only | Pinned commit SHA | Full 40-character hex; get via `git rev-parse HEAD` |
| `build.source_dir` | No | Source subdirectory | Path within repo, default `.` |
| `build.binary_name` | Binary only | Output binary name | Must match what the compiler produces |
| `build.main` | TS/Node/Python | Entry point file | e.g., `src/index.js` or `src/main.py` |
| `api_calls` | No | External API domains | Array of domain strings the Plugin calls |

#### Naming Rules

- **Allowed**: `solana-price-checker`, `defi-yield-optimizer`, `nft-tracker`
- **Forbidden**: `OKX-Plugin` (uppercase), `my_plugin` (underscores), `a` (too short)
- **Reserved prefix**: only OKX org members may use the `okx-` prefix

---

## 5. Writing SKILL.md

SKILL.md is the **single entry point** of your Plugin. It teaches the AI agent
what your Plugin does and how to use it. For Skill-only Plugins, it orchestrates
commands and tools. For Binary Plugins, it also orchestrates your custom tools.

```
Skill-Only Plugin:
  SKILL.md → commands / tools

Binary Plugin:
  SKILL.md → commands / tools
           + your binary tools (calculate_yield, find_route, ...)
           + your binary commands (my-tool start, my-tool status, ...)
```

### SKILL.md Template (Skill-Only)

```markdown
---
name: <your-plugin-name>
description: "Brief description of what this skill does"
version: "1.0.0"
author: "Your Name"
tags:
  - keyword1
  - keyword2
---

# My Awesome Plugin

## Overview

[2-3 sentences: what does this skill enable the AI agent to do?]

## Pre-flight Checks

Before using this skill, ensure:

1. The `onchainos` CLI is installed and configured
2. [Any other prerequisites]

## Commands

### [Command Name]

\`\`\`bash
onchainos <command> <subcommand> --flag value
\`\`\`

**When to use**: [Describe when the AI should use this command]
**Output**: [Describe what the command returns]
**Example**:

\`\`\`bash
onchainos token search --query SOL --chain solana
\`\`\`

### [Another Command]

...

## Examples

### Example 1: [Scenario Name]

[Walk through a complete workflow step by step]

1. First, run ...
2. Then, check ...
3. Finally, execute ...

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| "Token not found" | Invalid token symbol | Ask user to verify the token name |
| "Rate limited" | Too many requests | Wait 10 seconds and retry |

## Security Notices

- [Risk level and what operations the plugin performs]
- [Any disclaimers for trading or financial operations]

## Skill Routing

- For token swaps -> use `okx-dex-swap` skill
- For wallet balances -> use `okx-wallet-portfolio` skill
- For security scanning -> use `okx-security` skill
```

### SKILL.md Template (Binary Plugin)

If your Plugin includes a binary, the SKILL.md must tell the AI agent about
**both** onchainos commands and your custom binary tools:

```markdown
---
name: defi-yield-optimizer
description: "Optimize DeFi yield with custom analytics and onchainos execution"
version: "1.0.0"
author: "DeFi Builder"
tags:
  - defi
  - yield
---

# DeFi Yield Optimizer

## Overview

This plugin combines custom yield analytics (via binary tools) with
onchainos execution capabilities to find and enter the best DeFi positions.

## Pre-flight Checks

1. The `onchainos` CLI is installed and configured
2. The defi-yield binary is installed via plugin-store
3. A valid DEFI_API_KEY environment variable is set

## Binary Tools (provided by this plugin)

### calculate_yield
Calculate the projected APY for a specific DeFi pool.
**Parameters**: pool_address (string), chain (string)
**Returns**: APY percentage, TVL, risk score

### find_best_route
Find the optimal swap route to enter a DeFi position.
**Parameters**: from_token (string), to_token (string), amount (number)
**Returns**: Route steps, estimated output, price impact

## Commands (using onchainos + binary tools together)

### Find Best Yield

1. Call binary tool `calculate_yield` for the target pool
2. Run `onchainos token info --address <pool_token> --chain <chain>`
3. Present yield rate + token info to user

### Enter Position

1. Call binary tool `find_best_route` for the swap
2. Run `onchainos swap quote --from <token> --to <pool_token> --amount <amount>`
3. **Ask user to confirm** the swap amount and expected yield
4. Run `onchainos swap swap ...` to execute
5. Report result to user

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| Binary connection failed | Server not running | Run `npx skills add okx/plugin-store --skill defi-yield-optimizer` |
| "Pool not found" | Invalid pool address | Verify the pool contract address |
| "Insufficient balance" | Not enough tokens | Check balance with `onchainos portfolio all-balances` |

## Skill Routing

- For token swaps only -> use `okx-dex-swap` skill
- For security checks -> use `okx-security` skill
```

### SKILL.md as the Orchestrator

Your SKILL.md tells the AI agent how to use your Plugin's commands and tools:

```markdown
## Commands

### Check Yield (uses your binary tool)
Call binary tool `calculate_yield` with pool address and chain.

### Execute Deposit (uses CLI tools + your binary)
1. Call binary tool `find_best_route` for the chosen pool
2. Query swap quote for the target token
3. **Ask user to confirm** amount and expected yield
4. Execute the swap
5. Call binary tool `monitor_position` to start tracking
```

### SKILL.md Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Must match `name` in plugin.yaml |
| `description` | Yes | Brief description (should match plugin.yaml) |
| `version` | Yes | Must match `version` in plugin.yaml |
| `author` | Yes | Author name |
| `tags` | No | Keywords for discoverability |

### SKILL.md Required Sections

- **Overview** -- what the skill does
- **Pre-flight Checks** -- prerequisites, dependency installs (must be runnable from a blank environment)
- **Commands** -- each command with when-to-use, output description, and concrete example
- **Error Handling** -- table of errors, causes, and resolutions
- **Security Notices** -- risk level, disclaimers

### SKILL.md Best Practices

1. **Be specific** -- `onchainos token search --query SOL --chain solana` is better than "search for tokens"
2. **Always include error handling** -- the AI agent needs to know what to do when things fail
3. **Use skill routing** -- tell the AI when to defer to other skills
4. **Include pre-flight checks** -- dependency installation commands so the AI agent can set up from scratch
5. **Do not duplicate existing tool capabilities** -- orchestrate, do not replace

### Good vs Bad Examples

**Bad: vague instructions**
```
Use onchainos to get the price.
```

**Good: specific and actionable**
```
To get the current price of a Solana token:

\`\`\`bash
onchainos market price --address <TOKEN_ADDRESS> --chain solana
\`\`\`

**When to use**: When the user asks "what's the price of [token]?" on Solana.
**Output**: Current price in USD, 24h change percentage, 24h volume.
**If the token is not found**: Ask the user to verify the contract address or try `onchainos token search --query <NAME> --chain solana` first.
```

---

## Writing SUMMARY.md

Every plugin **must** include a `SUMMARY.md` file in the same directory as `SKILL.md`. This file is an **English-language** user-facing summary with a fixed 3-section structure. CI lint will reject submissions without it (error `E150`) or with missing sections (error `E151`).

### Required Structure

```markdown
# <plugin-name>

## Overview

<One sentence describing what this platform/protocol is.>

Core operations:

- <Core operation 1, e.g., "Swap tokens across 500+ DEX sources">
- <Core operation 2, e.g., "Query real-time portfolio balances">
- <Core operation 3, e.g., "Place limit orders with slippage protection">

Tags: `defi` `ethereum` `swap` `lending`

## Prerequisites

- <IP/region restrictions, e.g., "No IP restrictions" or "US users excluded">
- <Supported chains, e.g., "Ethereum, Base, Arbitrum, Polygon">
- <Supported tokens, e.g., "All ERC-20 tokens" or "USDC, WETH, DAI">
- <Required tools, e.g., "onchainos CLI installed and authenticated">
- <Any other setup steps>

## Quick Start

<Walk users through the basic workflow in plain language. Describe the key steps,
modes, or options. Use numbered lists or bold headings for each step.>

1. **Step 1 title**: Description of what to do and why.
2. **Step 2 title**: Description of the next action.
3. **Step 3 title**: How to verify everything works.
```

### Example: Polymarket Plugin

```markdown
# polymarket-plugin

## 1. Overview

Polymarket is a decentralized prediction market platform on Polygon where users trade
outcome shares on real-world events.

Core operations:

- Browse and search prediction markets by topic
- Buy and sell YES/NO outcome shares (market orders and limit orders)
- Check portfolio positions and P&L
- Manage open orders (cancel, modify)
- Claim winnings after market settlement

Tags: `prediction-market` `polygon` `trading` `polymarket`

## 2. Prerequisites

- US users are restricted from trading on Polymarket
- Supported chain: Polygon (MATIC)
- Supported tokens: USDC.e for trading, POL for gas fees
- onchainos CLI installed and authenticated
- A funded wallet with USDC.e on Polygon

## 3. Quick Start

1. **Choose a trading mode**
   - Direct mode: trade directly from your wallet. The agent handles token approvals
     automatically. Requires a small amount of POL for gas.
   - Proxy mode: the agent creates a dedicated proxy wallet for you. Polymarket covers
     gas fees. First-time setup requires depositing USDC.e to the proxy address.
     You can switch between modes at any time.

2. **Check balances**
   Ask "check my balance" to see POL and USDC.e across all wallets. If your proxy
   wallet is low, transfer USDC.e to the proxy address like a normal transfer.

3. **Find a market**
   Tell the agent what interests you (e.g., "find BTC price prediction markets").
   It will search for matching markets and show prices, volume, and expiry dates.

4. **Place a trade**
   - Market order: buy or sell at the current best price, fills immediately or cancels.
   - Limit order: set your desired price and wait for a fill. You can set an expiry
     or choose maker-only.
   - Examples: "Buy 10 USDC.e of Yes" / "Place a limit buy for 100 No shares at 0.35"

5. **Manage positions**
   Ask the agent to show your current holdings and P&L, cancel pending orders,
   or withdraw winnings after a market settles.
```

### Key Rules

- **Language**: SUMMARY.md must be written in English.
- **Sections**: All three sections (`## Overview`, `## Prerequisites`, `## Quick Start`) are required.
- **Tags**: Display tags using inline code blocks (`` `tag-name` ``), placed at the end of the Overview section.
- **Placement**: SUMMARY.md must be in the same directory as SKILL.md.

---

## Submitting Strategy Plugins

A **strategy plugin** (`category: strategy`) does not connect to chains/wallets directly. Instead, it calls existing trading plugins (e.g., polymarket-plugin, raydium-plugin) to execute orders. Strategy authors can be attributed and incentivized for trades generated through their strategies.

### plugin.yaml for Strategy Plugins

```yaml
schema_version: 1
name: my-arb-strategy
version: "1.0.0"
description: "Arbitrage between Raydium and Orca"
author:
  name: "alice"
  github: "alice"
license: MIT
category: strategy              # Must be "strategy"
tags:
  - solana
  - arbitrage

dependent_plugin:                # Required — declares which plugins this strategy calls
  - name: raydium-plugin
    version: "^0.2.0"
  - name: orca-plugin
    version: "^0.6.0"

risk_level: high                 # Informational — shown to users
supported_venues:                # Informational — for search/filtering
  - raydium
  - orca

components:
  skill:
    dir: skills/my-arb-strategy

api_calls: []
```

### --strategy-id Flag Requirement

All **trading operations** (buy, sell, swap, order) to dependent plugins **must** include `--strategy-id <strategy-name>` for attribution tracking:

```python
# ✅ Correct — write operation with --strategy-id
subprocess.run(["raydium-plugin", "swap", "--from", "USDC", "--to", "SOL",
                "--amount", "10", "--strategy-id", "my-arb-strategy", "--confirm"])

# ✅ Correct — read-only operation, no --strategy-id needed
subprocess.run(["raydium-plugin", "quote", "--token", "SOL"])

# ✅ Correct — deposit/withdraw do NOT need --strategy-id
subprocess.run(["raydium-plugin", "deposit", "--amount", "100", "--pool", "SOL-USDC"])

# ❌ Wrong — trading operation WITHOUT --strategy-id (AI review will reject)
subprocess.run(["raydium-plugin", "swap", "--from", "USDC", "--to", "SOL",
                "--amount", "10", "--confirm"])
```

### CI Checks for Strategy Plugins

| Phase | Check | Failure |
|-------|-------|---------|
| Phase 1 | `dependent_plugin[].name` exists in registry (E160) | PR blocked |
| Phase 3 | AI scans all write operations for `--strategy-id` flag | Flagged as Critical |
| Phase 3 | No hardcoded private keys, RPC URLs, or API keys | Flagged as Critical |

---

## 6. Submitting Plugins with Source Code (Binary)

> **Important:** SKILL.md is always the entry point. Even if your Plugin includes
> a binary, the SKILL.md is what tells the AI agent how to orchestrate
> everything -- onchainos commands, your binary tools, and your binary commands.

### Who Can Submit Source Code?

Any developer can submit source code for binary compilation. Submit your source
code in your own GitHub repo, add a `build` section to plugin.yaml, and our CI
will compile it.

### How It Works

```
You submit source code → Our CI compiles it → Users install our compiled artifact
You never submit binaries. We never modify your source code.
```

### plugin.yaml with Build Config

Your source code stays in your own GitHub repo. You provide the repo URL and a
pinned commit SHA -- our CI clones at that exact commit, compiles, and publishes.
The commit SHA is the content fingerprint: same SHA = same code, guaranteed.

### How to Get the Commit SHA

Your source code must be pushed to GitHub **before** you can get a valid commit
SHA. The workflow is:

```
# 1. In your source code repo -- develop and push your code first
cd your-source-repo
git add . && git commit -m "v1.0.0"
git push origin main

# 2. Get the full 40-character commit SHA
git rev-parse HEAD
# Output: a1b2c3d4e5f6789012345678901234567890abcd

# 3. Copy this SHA into your plugin.yaml build.source_commit field
```

> The commit must exist on GitHub (not just local). Our CI clones from GitHub at
> this exact SHA.

### Required Source Directory Structure Per Language

**Your repo must compile with a single standard command.** No custom scripts, no
multi-step builds. Our CI runs exactly one build command per language.

**Rust:**
```
your-org/your-tool/
├── Cargo.toml          ← MUST contain [[bin]] with name matching binary_name
├── Cargo.lock           ← commit this (reproducible builds)
└── src/
    └── main.rs          ← your code
```

`Cargo.toml` must have:
```toml
[package]
name = "your-tool"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "your-tool"      # ← this MUST match build.binary_name in plugin.yaml
path = "src/main.rs"
```

**Go:**
```
your-org/your-tool/
├── go.mod               ← MUST have module declaration
├── go.sum               ← commit this
└── main.go              ← must have package main + func main()
```

**TypeScript:**
```
your-org/your-tool/
├── package.json         ← MUST have name, version, and "bin" field
└── src/
    └── index.js         ← Compiled to JS, first line MUST be #!/usr/bin/env node
```

> **Important:** TypeScript plugins are distributed via `bun install -g`, not
> compiled to binary. Your `package.json` must include a `"bin"` field pointing
> to the JS entry file, and the entry file must start with `#!/usr/bin/env node`.
> If you write in TypeScript, compile to JS before pushing to your source repo.

`package.json` example:
```json
{
  "name": "your-tool",
  "version": "1.0.0",
  "type": "module",
  "bin": {
    "your-tool": "src/index.js"
  }
}
```

**Node.js:**
```
your-org/your-tool/
├── package.json         ← MUST have name, version, and "bin" field
└── src/
    └── index.js         ← First line MUST be #!/usr/bin/env node
```

> **Important:** Node.js plugins are distributed via `bun install -g`, not
> compiled to binary. Your `package.json` must include a `"bin"` field, and the
> entry file must start with `#!/usr/bin/env node`.

`package.json` example:
```json
{
  "name": "your-tool",
  "version": "1.0.0",
  "bin": {
    "your-tool": "src/index.js"
  }
}
```

**Python:**
```
your-org/your-tool/
├── pyproject.toml       ← MUST have [build-system], [project] (with name, version), and [project.scripts]
├── setup.py             ← Recommended for compatibility with older pip versions
└── src/
    ├── __init__.py
    └── main.py          ← This path goes in build.main; must have def main() entry
```

> **Important:** Python plugins are distributed via `pip install`, not compiled
> to binary. Your `pyproject.toml` must include `[project.scripts]` to define the
> CLI entry point. We recommend also providing `setup.py` for older pip
> compatibility.

`pyproject.toml` example:
```toml
[build-system]
requires = ["setuptools", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "your-tool"
version = "1.0.0"
requires-python = ">=3.8"

[project.scripts]
your-tool = "src.main:main"
```

### Build Config -- Complete Examples for Each Language

Every `build` field explained:

| Field | Required | Description |
|-------|----------|-------------|
| `lang` | Yes | `rust` \| `go` \| `typescript` \| `node` \| `python` |
| `source_repo` | Yes | GitHub `owner/repo` containing your source code |
| `source_commit` | Yes | Full 40-char commit SHA (get via `git rev-parse HEAD`) |
| `source_dir` | No | Path within repo to source root (default: `.`) |
| `entry` | No | Entry file override (default: auto-detected per language) |
| `binary_name` | Yes | Name of the compiled output binary |
| `main` | TS/Node/Python | Entry point file (e.g. `src/index.js`, `src/main.py`) |
| `targets` | No | Limit build platforms (default: all supported) |

#### Rust

```yaml
build:
  lang: rust
  source_repo: "your-org/your-rust-tool"
  source_commit: "a1b2c3d4e5f6789012345678901234567890abcd"
  source_dir: "."                        # default, can omit
  entry: "Cargo.toml"                    # default for Rust, can omit
  binary_name: "your-tool"              # must match [[bin]] name in Cargo.toml
  targets:                               # optional, omit for all platforms
    - x86_64-unknown-linux-gnu
    - aarch64-apple-darwin
```

CI runs: `cargo fetch` -> `cargo audit` -> `cargo build --release`
Output: native binary (~5-20MB)

#### Go

```yaml
build:
  lang: go
  source_repo: "your-org/your-go-tool"
  source_commit: "b2c3d4e5f6789012345678901234567890abcdef"
  source_dir: "."
  entry: "go.mod"                        # default for Go, can omit
  binary_name: "your-tool"
  targets:
    - x86_64-unknown-linux-gnu
    - aarch64-apple-darwin
```

CI runs: `go mod download` -> `govulncheck` -> `CGO_ENABLED=0 go build -ldflags="-s -w"`
Output: static native binary (~5-15MB)

#### TypeScript

```yaml
build:
  lang: typescript
  source_repo: "your-org/your-ts-tool"
  source_commit: "c3d4e5f6789012345678901234567890abcdef01"
  source_dir: "."
  binary_name: "your-tool"
  main: "src/index.js"                   # REQUIRED -- must be JS (not .ts)
```

Distribution: `bun install -g`
Requires: Bun
Output size: ~KB (source install, no large binary download)

> **Note:** `package.json` must include a `"bin"` field, and entry file must
> start with `#!/usr/bin/env node`. If writing in TypeScript, compile to JS
> before pushing to your source repo.

#### Node.js

```yaml
build:
  lang: node
  source_repo: "your-org/your-node-tool"
  source_commit: "e5f6789012345678901234567890abcdef012345"
  source_dir: "."
  binary_name: "your-tool"
  main: "src/index.js"                   # REQUIRED for Node.js
```

Distribution: `bun install -g`
Requires: Bun
Output size: ~KB (source install)

> **Note:** `package.json` must include a `"bin"` field, and entry file must
> start with `#!/usr/bin/env node`.

> Node.js and TypeScript use the same distribution method (bun install). The only
> difference is that TypeScript must be compiled to JS first.

#### Python

```yaml
build:
  lang: python
  source_repo: "your-org/your-python-tool"
  source_commit: "d4e5f6789012345678901234567890abcdef0123"
  source_dir: "."
  binary_name: "your-tool"
  main: "src/main.py"                    # REQUIRED for Python
```

Distribution: `pip install`
Requires: Python 3.8+ and pip/pip3
Output size: ~KB (source install)

> **Note:** `pyproject.toml` must include `[build-system]`, `[project]`, and
> `[project.scripts]`. We recommend also providing `setup.py` for older pip
> compatibility. Entry function must be `def main()`.

### Local Build Verification

Before submitting, verify your code compiles with the exact command our CI uses:

```
# Rust
cd your-tool && cargo build --release
# Verify: target/release/your-tool exists

# Go
cd your-tool && CGO_ENABLED=0 go build -o your-tool -ldflags="-s -w" .
# Verify: ./your-tool exists

# TypeScript / Node.js
cd your-tool && bun install -g .
# Verify: your-tool --help runs successfully
# Note: package.json must have "bin" field, entry file must have #!/usr/bin/env node

# Python
cd your-tool && pip install .
# Verify: your-tool --help runs successfully
# Note: pyproject.toml must have [project.scripts], entry function must be def main()
```

If these commands fail locally, our CI will also fail.

### Common Build Failures

| Problem | Cause | Fix |
|---------|-------|-----|
| "Binary not found" | `binary_name` doesn't match compiled output | Rust: check `[[bin]] name` in Cargo.toml. Go: check `-o` flag. |
| "source_commit is not valid" | Short SHA or branch name used | Use full 40-char: `git rev-parse HEAD` |
| "source_repo format invalid" | Wrong format | Must be `owner/repo`, not `https://github.com/...` |
| Build fails but works locally | Platform difference | Our CI runs Ubuntu Linux. Ensure your code compiles on Linux. |
| Cargo.lock not found | Not committed | Run `cargo generate-lockfile` and commit `Cargo.lock`. |
| Python import error | Missing dependency | Ensure all deps are in `pyproject.toml` or `requirements.txt`. |

### What You Cannot Do (Binary Plugins)

- Submit pre-compiled binaries (.exe, .dll, .so, .wasm) -- E130
- Declare Binary without a build section -- E110/E111
- Have source code larger than 10MB -- E126
- Include build scripts that download from the internet during compilation

---

## 7. Three Submission Modes

### Mode A -- Direct Submission (Recommended)

Everything lives inside `skills/<name>/` in the plugin-store repo. This is the
simplest approach and recommended for most Plugins.

**Skill-Only Plugin:**

```
skills/my-plugin/
├── .claude-plugin/
│   └── plugin.json   # Required
├── plugin.yaml       # Required
├── SKILL.md          # Required
├── scripts/          # Optional -- Python/shell scripts
├── assets/           # Optional -- HTML, images
├── LICENSE
└── README.md
```

**Plugin with source code (e.g. Rust CLI):**

```
skills/my-rust-tool/
├── .claude-plugin/
│   └── plugin.json   # Required
├── plugin.yaml       # Required (with build section, no source_repo needed)
├── SKILL.md          # Required
├── Cargo.toml        # Build config (or go.mod, package.json, pyproject.toml)
├── Cargo.lock
├── src/
│   └── main.rs       # Source code -- CI compiles directly from this directory
├── LICENSE
└── README.md
```

plugin.yaml uses `components.skill.dir` and optionally a `build` section:

```yaml
components:
  skill:
    dir: "."

# For binary Plugins (omit source_repo to use local source):
build:
  lang: rust            # rust | go | typescript | node | python
  binary_name: my-tool  # compiled output name
```

**When to use**: You want everything in one place. Works for Skill-only Plugins,
Plugins with scripts, and Plugins with compiled source code. CI compiles directly
from the plugin directory — no external repo needed.

### Mode B -- External Repository

Your plugin.yaml points to your own GitHub repo with a pinned commit SHA.
Only `plugin.yaml` (and optionally `LICENSE`, `README.md`) lives in the
plugin-store repo.

```
skills/my-plugin/
├── plugin.yaml       # Points to your external repo
└── LICENSE
```

plugin.yaml uses `components.skill.repo` and `components.skill.commit`:

```yaml
components:
  skill:
    repo: "your-username/my-plugin"
    commit: "d2aa628e063d780c370b0ec075a43df4859be951"
```

The commit must be a **full 40-character SHA** (not a short SHA or branch name).
Get it with:

```
cd your-source-repo
git push origin main
git rev-parse HEAD
# Output: d2aa628e063d780c370b0ec075a43df4859be951
```

**When to use**: Your Plugin has substantial source code, you want to keep it in
your own repo, or you want independent versioning. This is the approach used by
Plugins like `meme-trench-scanner` and `smart-money-signal-copy-trade`.

---

## 8. Onchain OS Integration

### What Is Onchain OS?

[Onchain OS](https://github.com/okx/onchainos-skills) embeds the Agentic Wallet CLI,
providing secure, sandboxed blockchain operations -- wallet signing,
transaction broadcasting, swap execution, contract calls, and more. It uses TEE
(Trusted Execution Environment) signing so private keys never leave the secure
enclave.

### When to Use Onchain OS

Use Onchain OS when your Plugin performs any on-chain write operation:

- Wallet signing
- Transaction broadcasting
- Swap execution
- Contract calls
- Token approvals

### Is Onchain OS Required?

**No. Onchain OS is optional.** Plugins can freely use any on-chain technology — Onchain OS, third-party wallets, direct RPC calls, or any other approach.

For non-blockchain Plugins (analytics, utilities, developer tools, etc.), Onchain OS is simply not applicable.

### Onchain OS Command Reference

| Command | Description | Example |
|---------|-------------|---------|
| `onchainos token` | Token search, info, trending, holders | `onchainos token search --query SOL --chain solana` |
| `onchainos market` | Price, kline charts, portfolio PnL | `onchainos market price --address 0x... --chain ethereum` |
| `onchainos swap` | DEX swap quotes and execution | `onchainos swap quote --from ETH --to USDC --amount 1` |
| `onchainos gateway` | Gas estimation, tx simulation, broadcast | `onchainos gateway gas --chain ethereum` |
| `onchainos portfolio` | Wallet total value and balances | `onchainos portfolio all-balances --address 0x...` |
| `onchainos wallet` | Login, balance, send, history | `onchainos wallet balance --chain solana` |
| `onchainos security` | Token scan, dapp scan, tx scan | `onchainos security token-scan --address 0x...` |
| `onchainos signal` | Smart money / whale signals | `onchainos signal list --chain solana` |
| `onchainos memepump` | Meme token scanning and analysis | `onchainos memepump tokens --chain solana` |
| `onchainos leaderboard` | Top traders by PnL/volume | `onchainos leaderboard list --chain solana` |
| `onchainos payment` | x402 payment protocol | `onchainos payment x402-pay --url ...` |

For the full subcommand list, run `onchainos <command> --help` or see the
[onchainos documentation](https://github.com/okx/onchainos-skills).

### Installing Onchain OS

```
npx skills add okx/onchainos-skills
```

If `onchainos` is not found after installation, add it to your PATH:

```
export PATH="$HOME/.local/bin:$PATH"
```

---

## 9. Review Process

Every plugin submission goes through a 4-phase CI pipeline.

### Phase 1: Static Lint (Automatic, Instant)

Validates Plugin structure, naming conventions, version format, required files,
and safety defaults. Results are posted as a PR comment.

If lint fails, the PR is blocked. Fix the issues and push again.

### Phase 2: Build Verification (Automatic, Binary Plugins Only)

If your Plugin has a `build` section, CI clones your source repo at the pinned
commit SHA, compiles the code, and verifies the binary runs. Build failures
block the PR.

### Phase 3: AI Code Review (Manual Trigger, ~2 Minutes)

An AI reviewer reads your Plugin and generates an 8-section report covering
security, compliance, and quality. The report is posted as a collapsible PR
comment. This stage is **advisory only** -- it does not block merge, but human
reviewers will read the report.

### Phase 4: Summary and Pre-flight (Manual Trigger)

A summary of all previous phases is generated. The pre-flight step auto-injects
the following into the test environment:

- **onchainos CLI** -- the Agentic Wallet CLI
- **Skills** -- your Plugin's skill files
- **plugin-store Skill** -- the plugin-store skill itself
- **HMAC install report** -- a signed report confirming installation integrity

This ensures every Plugin can be verified end-to-end in a realistic environment.

### Human Review (1-3 Business Days)

After all automated phases pass, a maintainer reviews the Plugin for
correctness, security, and quality. They check that the Plugin makes sense,
API calls are accurately declared, SKILL.md is well-written, and there are no
security concerns.

### Common Rejection Reasons

| # | Reason | How to Avoid |
|---|--------|-------------|
| 1 | Missing `plugin.yaml`, `.claude-plugin/plugin.json`, or `SKILL.md` | All three files are required in every Plugin |
| 2 | Version mismatch between `plugin.yaml` and `SKILL.md` | Keep `version` identical in both files |
| 3 | Hardcoded API keys or credentials | Use environment variables, never commit secrets |
| 4 | No risk disclaimer for trading Plugins | Add a disclaimer section in SKILL.md for any Plugin that moves assets |
| 5 | Direct wallet operations without Onchain OS | Use `onchainos wallet` / `onchainos swap` for on-chain writes |
| 6 | Missing LICENSE file | Add a LICENSE file with an SPDX-compatible license |
| 7 | Unpinned dependencies | Pin all dependency versions; use lockfiles |
| 8 | Category mismatch | Choose the category that most accurately describes your Plugin |
| 9 | SKILL.md missing required sections | Include Overview, Pre-flight, Commands, Error Handling, Security Notices |
| 10 | Auto-trading without dry-run mode | All automated trading Plugins must support a dry-run / paper-trade mode |

### Common Lint Errors

| Code | Meaning | Fix |
|------|---------|-----|
| E001 | plugin.yaml not found | Ensure plugin.yaml is at the root of your plugin directory |
| E031 | Invalid name format | Lowercase letters, numbers, and hyphens only |
| E033 | Reserved prefix | Only OKX org members may use the `okx-` prefix |
| E035 | Invalid version | Use semantic versioning: `1.0.0`, not `1.0` or `v1.0.0` |
| E041 | Missing LICENSE | Add a LICENSE file |
| E052 | Missing SKILL.md | Ensure SKILL.md exists in the directory specified by `components.skill.dir` |
| E065 | Missing api_calls | Add `api_calls` field to plugin.yaml (use `[]` if none) |
| E110 | Binary declared without build section | Add `build.lang`, `build.source_repo`, `build.source_commit` |
| E122 | Invalid source_repo format | Use `owner/repo` format, not full URL |
| E123 | Invalid source_commit | Must be full 40-character hex SHA from `git rev-parse HEAD` |
| E130 | Pre-compiled binary submitted | Remove binary files; submit source code, CI compiles it |

### Pre-Submission Checklist

Items reviewed before publication:

```markdown
- [ ] `plugin.yaml`, `.claude-plugin/plugin.json`, and `SKILL.md` all present
- [ ] `name` field is lowercase with hyphens only, 2-40 characters
- [ ] `version` matches in `plugin.yaml`, `plugin.json`, and `SKILL.md`
- [ ] `author.github` matches my GitHub username
- [ ] `license` field uses a valid SPDX identifier
- [ ] `category` is one of the allowed values
- [ ] `api_calls` lists all external API domains (or `[]` if none)
- [ ] SKILL.md has YAML frontmatter with name, description, version, author
- [ ] SKILL.md includes Overview, Pre-flight, Commands, Error Handling sections
- [ ] No hardcoded API keys, tokens, or credentials anywhere
- [ ] No pre-compiled binary files in the submission
- [ ] LICENSE file is present
- [ ] Submission only modifies files inside `skills/my-plugin/`
- [ ] (If trading plugin) Risk disclaimer is included
- [ ] (If trading plugin) Dry-run / paper-trade mode is supported
- [ ] (If binary plugin) Source code compiles locally with CI-equivalent command
- [ ] Local lint passes: `plugin-store lint skills/my-plugin`
```

---

## 10. Risk Levels

Every Plugin is assigned one of three risk levels based on what it does.

| Level | Name | Definition | Extra Requirements |
|-------|------|-----------|-------------------|
| `starter` | Starter | Read-only operations, no asset movement | Standard review |
| `standard` | Standard | Transactions with explicit user confirmation each time | Standard review + confirmation flow check |
| `advanced` | Advanced | Automated strategies, may operate autonomously | See below |

### Advanced-Level Requirements

Plugins at the `advanced` risk level must include all of the following:

1. **Dry-run / paper-trade mode** -- must be the default or clearly documented
2. **Stop-loss mechanism** -- configurable maximum loss threshold
3. **Maximum amount limits** -- configurable per-trade and per-session caps
4. **Risk disclaimer** -- prominent disclaimer in SKILL.md (see the
   `meme-trench-scanner` Plugin for a thorough example)
5. **Two reviewers** -- advanced Plugins require approval from two maintainers

### Absolute Red Lines

The following will result in immediate rejection regardless of risk level:

1. **Hardcoded private keys or seed phrases** in any file
2. **Obfuscated or minified source code** that cannot be reviewed
3. **Network calls to undeclared domains** not listed in `api_calls`
4. **Prompt injection patterns** in SKILL.md (attempts to override agent safety)
5. **Exfiltration of user data** -- sending wallet addresses, balances, or
   trading history to external servers without explicit user consent
6. **Bypassing confirmation flows** -- executing transactions without user
   approval when the Plugin declares `standard` risk level
7. **Unlimited autonomous trading** -- `advanced` Plugins without stop-loss or
   max-amount safeguards
8. **Impersonation** -- using names, descriptions, or branding that falsely
   imply official endorsement by OKX or other organizations
9. **Pre-compiled binaries** -- submit source code; CI compiles it
10. **License violations** -- using code from incompatible licenses without
    attribution

---

## 11. Rules & Restrictions

### What You CAN Do

- Define skills using SKILL.md
- Reference any onchainos CLI command for on-chain operations
- Query external data sources (third-party DeFi APIs, market data, etc.)
- Include reference documentation
- Submit binary source code (we compile it via `build` section)
- Declare `api_calls` for external API domains

### What You CANNOT Do

- Submit pre-compiled binaries (.exe, .dll, .so, etc.) -- must submit source code
- Use the reserved `okx-` prefix (OKX org members only)
- Include prompt injection patterns in SKILL.md
- Exceed file size limits (200KB per file, 5MB total)

---

## 12. FAQ

1. **How long does the review take?**
   Automated checks complete in under 5 minutes. Human review typically takes 1-3 business days.

2. **Can I update my Plugin after it is published?**
   Yes. Modify your files, bump `version` in both `plugin.yaml` and `SKILL.md`, and package a new submission for review.

3. **What are the Plugin naming rules?**
   Lowercase letters, numbers, and hyphens only. Between 2 and 40 characters. No consecutive hyphens. No underscores. The `okx-` prefix is reserved for OKX organization members.

4. **Can I use any programming language?**
   For binary Plugins: Rust, Go, TypeScript (Bun), Node.js (Bun), and Python. For Skill-only Plugins, you can include scripts in any language (Python and shell are common) -- they run as part of the AI agent workflow, not compiled by CI.

5. **Do I have to use Onchain OS?**
   No. Onchain OS is optional. Plugins can freely use any on-chain technology.

6. **How do users install my Plugin?**
   Once published: `npx skills add okx/plugin-store --skill my-plugin`. No plugin-store CLI installation is required on the user's end.

7. **What if the AI review flags something?**
   The AI review is advisory only and does not block release. However, human reviewers will read the AI report. Addressing flagged issues speeds up approval.

8. **My local lint passes but the CI check fails. Why?**
   Ensure you are running the latest version of the plugin-store CLI. Also confirm the submission only modifies files within `skills/your-plugin-name/`.

9. **The build failed in CI but compiles locally. Why?**
   CI compiles on Ubuntu Linux. Ensure your code builds on Linux, not just macOS or Windows. Check the build logs from the CI run for specific errors.

10. **What does error E122 "source_repo is not valid" mean?**
    `build.source_repo` must be in `owner/repo` format (e.g. `your-username/my-server`). Do not include `https://github.com/` or `.git`.

11. **What does error E123 "must be a full 40-character hex SHA" mean?**
    `build.source_commit` must be the full commit hash, not a short SHA or branch name. Run `git rev-parse HEAD` to get the full 40-character hash.

12. **What does error E120 "must also include a Skill component" mean?**
    Every Plugin with a `build` section must also have a SKILL.md. The Skill is the entry point -- it tells the AI agent how to use your binary.

13. **What does error E130 "pre-compiled binary file is not allowed" mean?**
    You submitted a compiled file (.exe, .dll, .so, .wasm, etc.). Remove it -- we compile from your source code, you don't submit binaries.

14. **What does error E110/E111 "requires a build section" mean?**
    You declared a Binary component but did not include a `build` section. Add `build.lang`, `build.source_repo`, and `build.source_commit`.

---
