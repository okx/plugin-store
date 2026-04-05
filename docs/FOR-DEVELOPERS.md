# Developer Guide: Build and Submit Plugins

> Build plugins for the Plugin Store ecosystem and submit them for review.
> By the end of this guide you will have a working plugin that users can install
> via `npx skills add okx/plugin-store --skill <name>`.

---

## Table of Contents

1. [What Can You Build?](#1-what-can-you-build)
2. [Quick Start (5 Minutes)](#2-quick-start-5-minutes)
3. [Plugin Structure](#3-plugin-structure)
4. [Three Submission Modes](#4-three-submission-modes)
5. [OnchainOS Integration](#5-onchainos-integration)
6. [Review Process](#6-review-process)
7. [Risk Levels](#7-risk-levels)
8. [FAQ](#8-faq)

---

## 1. What Can You Build?

Plugins are **not limited to Web3**. You can build analytics dashboards, developer
utilities, trading strategies, DeFi integrations, security scanners, NFT tools,
or anything else that benefits from AI-agent orchestration.

### Two Plugin Types

| Type | What It Contains | Best For |
|------|-----------------|----------|
| **Pure Skill** | `SKILL.md` only (plus optional scripts, assets, references) | Strategies, workflows, data queries, anything that orchestrates existing CLIs |
| **Skill + Binary** | `SKILL.md` plus compiled CLI tool (source code compiled by CI) | Custom computation, proprietary algorithms, complex data processing |

Even when a plugin includes a binary, **SKILL.md is always the entry point**.
The Skill tells the AI agent what tools are available and when to use them.

### Supported Languages for Binary Plugins

| Language | Build Tool | Distribution |
|----------|-----------|-------------|
| Rust | `cargo build --release` | Native binary |
| Go | `go build` | Native binary |
| TypeScript | `npm install -g` | npm package |
| Node.js | `npm install -g` | npm package |
| Python | `pip install` | pip package |

### What Makes a Good Plugin

- **Useful** -- solves a real problem or automates a tedious workflow
- **Safe** -- does not handle private keys directly, declares all external API calls, includes risk disclaimers where appropriate
- **Well-documented** -- clear SKILL.md with concrete examples, error handling, and pre-flight checks so the AI agent can operate from a blank environment

---

## 2. Quick Start (5 Minutes)

This walkthrough creates a minimal Skill-only plugin and submits it.

### Step 1: Fork and Clone

```bash
gh repo fork okx/plugin-store --clone
cd plugin-store
```

### Step 2: Create Your Plugin Directory

```bash
mkdir -p skills/my-plugin
```

### Step 3: Create plugin.yaml

```bash
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
    dir: skills/my-plugin

api_calls: []
EOF
```

### Step 4: Create SKILL.md

```bash
cat > skills/my-plugin/SKILL.md << 'SKILLEOF'
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

```bash
# The command the AI agent should run
example-command --flag value
```

**When to use**: Describe when the AI agent should invoke this command.
**Output**: Describe what the command returns.
**Example**:

```bash
example-command --flag "real-value"
# Expected output: ...
```

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| "Something failed" | Why it happens | What the AI agent should do |

## Security Notices

- This plugin is read-only and does not perform transactions.
SKILLEOF
```

### Step 5: Validate Locally

```bash
cd /path/to/plugin-store
cargo run --manifest-path cli/Cargo.toml -- lint skills/my-plugin
```

If everything passes:

```
Linting skills/my-plugin...

  Plugin 'my-plugin' passed all checks!
```

### Step 6: Submit a Pull Request

```bash
git checkout -b submit/my-plugin
git add skills/my-plugin
git commit -m "[new-plugin] my-plugin v1.0.0"
git push origin submit/my-plugin
```

Then open a Pull Request from your fork to `okx/plugin-store`. Use this title:

```
[new-plugin] my-plugin v1.0.0
```

Each PR should contain **one plugin only** and should only modify files inside
`skills/my-plugin/`.

---

## 3. Plugin Structure

### Directory Layout

```
skills/my-plugin/
├── plugin.yaml          # Required -- plugin metadata and manifest
├── SKILL.md             # Required -- skill definition for the AI agent
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

Both `plugin.yaml` and `SKILL.md` are **required**. Everything else is optional.

### plugin.yaml Reference

#### Minimal Example (Skill-Only, Direct Submission)

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
  - analytics

components:
  skill:
    dir: skills/sol-price-checker

api_calls: []
```

#### External Repo Example (Mode B)

When your source code lives in your own GitHub repo, use `repo` and `commit`
instead of `dir`:

```yaml
schema_version: 1
name: my-trading-bot
version: "1.0.0"
description: "Automated trading bot with safety checks"
author:
  name: "Your Name"
  github: "your-github-username"
license: MIT
category: trading-strategy
tags:
  - trading
  - solana

components:
  skill:
    repo: "your-username/my-trading-bot"
    commit: "d2aa628e063d780c370b0ec075a43df4859be951"

api_calls: []
```

#### Binary Plugin Example (Skill + Compiled CLI)

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
    dir: skills/defi-yield-optimizer

build:
  lang: rust
  source_repo: "defi-builder/yield-optimizer"
  source_commit: "a1b2c3d4e5f6789012345678901234567890abcd"
  source_dir: "."
  binary_name: defi-yield

api_calls:
  - "api.defillama.com"
```

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
| `category` | Yes | Plugin category | One of: `trading-strategy`, `defi-protocol`, `analytics`, `utility`, `security`, `wallet`, `nft` |
| `tags` | No | Search keywords | Array of strings |
| `type` | No | Author type | `"official"`, `"dapp-official"`, `"community-developer"` |
| `link` | No | Project homepage | URL, displayed in the marketplace |
| `components.skill.dir` | Mode A | Skill directory path | Relative path to directory containing SKILL.md |
| `components.skill.repo` | Mode B | External repository | Format: `owner/repo` |
| `components.skill.commit` | Mode B | Pinned commit | Full 40-character hex SHA |
| `build.lang` | Binary only | Source language | `rust` / `go` / `typescript` / `node` / `python` |
| `build.source_repo` | Binary only | Source code repo | Format: `owner/repo` |
| `build.source_commit` | Binary only | Pinned commit SHA | Full 40-character hex; get via `git rev-parse HEAD` |
| `build.source_dir` | No | Source subdirectory | Path within repo, default `.` |
| `build.binary_name` | Binary only | Output binary name | Must match what the compiler produces |
| `build.main` | TS/Node/Python | Entry point file | e.g., `src/index.js` or `src/main.py` |
| `api_calls` | No | External API domains | Array of domain strings the plugin calls |

#### Naming Rules

- **Allowed**: `solana-price-checker`, `defi-yield-optimizer`, `nft-tracker`
- **Forbidden**: `OKX-Plugin` (uppercase), `my_plugin` (underscores), `a` (too short)
- **Reserved prefixes**: `okx-`, `official-`, `plugin-store-` -- only OKX org members may use `okx-`

### SKILL.md Reference

SKILL.md is the **single entry point** for your plugin. It teaches the AI agent
what your plugin does and how to use it.

#### Full Template

```markdown
---
name: my-plugin
description: "Brief description of what this skill does"
version: "1.0.0"
author: "Your Name"
tags:
  - keyword1
  - keyword2
---

# My Plugin

## Overview

[2-3 sentences: what does this skill enable the AI agent to do?]

## Pre-flight Checks

Before using this skill, ensure:

1. [Prerequisite 1, e.g., "The `onchainos` CLI is installed and configured"]
2. [Prerequisite 2, e.g., "A valid API_KEY environment variable is set"]

## Commands

### [Command Name]

\`\`\`bash
onchainos <command> <subcommand> --flag value
\`\`\`

**When to use**: [Describe when the AI agent should use this command]
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
| "Rate limited" | Too many API requests | Wait 10 seconds and retry |
| "Insufficient balance" | Not enough tokens | Check balance first |

## Security Notices

- [Risk level and what operations the plugin performs]
- [Any disclaimers for trading or financial operations]

## Skill Routing

- For token swaps -> use `okx-dex-swap` skill
- For wallet balances -> use `okx-wallet-portfolio` skill
- For security scanning -> use `okx-security` skill
```

#### SKILL.md for Binary Plugins

When your plugin includes a binary tool, your SKILL.md must document **both**
the binary tools and any CLI commands:

```markdown
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
```

#### SKILL.md Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Must match `name` in plugin.yaml |
| `description` | Yes | Brief description (should match plugin.yaml) |
| `version` | Yes | Must match `version` in plugin.yaml |
| `author` | Yes | Author name |
| `tags` | No | Keywords for discoverability |

#### SKILL.md Required Sections

- **Overview** -- what the skill does
- **Pre-flight Checks** -- prerequisites, dependency installs (must be runnable from a blank environment)
- **Commands** -- each command with when-to-use, output description, and concrete example
- **Error Handling** -- table of errors, causes, and resolutions
- **Security Notices** -- risk level, disclaimers

#### SKILL.md Best Practices

1. **Be specific** -- `onchainos token search --query SOL --chain solana` is better than "search for tokens"
2. **Always include error handling** -- the AI agent needs to know what to do when things fail
3. **Use skill routing** -- tell the AI when to defer to other skills
4. **Include pre-flight checks** -- dependency installation commands so the AI agent can set up from scratch
5. **Do not duplicate onchainos capabilities** -- orchestrate existing commands, do not replace them

---

## 4. Three Submission Modes

### Mode A -- Direct Submission (Recommended)

Everything lives inside `skills/<name>/` in the plugin-store repo. This is the
simplest approach and recommended for most plugins.

```
skills/my-plugin/
├── plugin.yaml
├── SKILL.md
├── scripts/          # Optional
├── assets/           # Optional
├── LICENSE
└── README.md
```

plugin.yaml uses `components.skill.dir`:

```yaml
components:
  skill:
    dir: skills/my-plugin
```

**When to use**: You are comfortable putting all source code directly in the
plugin-store repo. Works well for Skill-only plugins and plugins with small
scripts.

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

```bash
cd your-source-repo
git push origin main
git rev-parse HEAD
# Output: d2aa628e063d780c370b0ec075a43df4859be951
```

**When to use**: Your plugin has substantial source code, you want to keep it in
your own repo, or you want independent versioning. This is the approach used by
plugins like `meme-trench-scanner` and `smart-money-signal-copy-trade`.

### Mode C -- Marketplace Import

If you already have a Claude marketplace-compatible repo, auto-generate the
submission:

```bash
plugin-store import your-username/my-plugin
```

This automatically reads your repo structure, detects the build language,
generates `plugin.yaml`, forks the plugin-store repo, creates a branch, and
opens a PR.

**Prerequisites**: `gh` CLI installed and authenticated (`gh auth login`).

**When to use**: You already have a working Claude marketplace plugin and want to
cross-list it in the plugin store with minimal effort.

---

## 5. OnchainOS Integration

### What Is OnchainOS?

[OnchainOS](https://github.com/okx/onchainos-skills) is the Agentic Wallet CLI
that provides secure, sandboxed blockchain operations -- wallet signing,
transaction broadcasting, swap execution, contract calls, and more. It uses TEE
(Trusted Execution Environment) signing so private keys never leave the secure
enclave.

### When to Use OnchainOS

Use OnchainOS when your plugin performs any on-chain write operation:

- Wallet signing
- Transaction broadcasting
- Swap execution
- Contract calls
- Token approvals

### Is OnchainOS Required?

**OnchainOS is recommended but NOT required.** Plugins are not limited to Web3.

However:

- Plugins that use OnchainOS for chain operations get a **higher trust score**
  and **better visibility** in the marketplace
- Non-OnchainOS chain plugins need **extra security review** because they handle
  blockchain operations outside the sandboxed environment
- Plugins that use third-party wallets (MetaMask, Phantom) or direct RPC calls
  (ethers.js, web3.js) for on-chain write operations will face stricter review
  and may be rejected if they cannot demonstrate equivalent safety

For non-blockchain plugins (analytics, utilities, developer tools, etc.),
OnchainOS is simply not applicable.

### OnchainOS Command Reference

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

### Installing OnchainOS

```bash
curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | bash
```

If `onchainos` is not found after installation, add it to your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

---

## 6. Review Process

Every pull request goes through a 4-stage pipeline.

### Stage 1: Static Lint (Automatic, Instant)

Validates plugin structure, naming conventions, version format, required files,
and safety defaults. Results are posted as a PR comment.

If lint fails, the PR is blocked. Fix the issues and push again.

### Stage 2: AI Code Review (Automatic, ~2 Minutes)

An AI reviewer reads your plugin and generates an 8-section report covering
security, compliance, and quality. The report is posted as a collapsible PR
comment. This stage is **advisory only** -- it does not block merge, but human
reviewers will read the report.

### Stage 3: Build Verification (Automatic, Binary Plugins Only)

If your plugin has a `build` section, CI clones your source repo at the pinned
commit SHA, compiles the code, and verifies the binary runs. Build failures
block the PR.

### Stage 4: Human Review (1-3 Business Days)

A maintainer reviews the plugin for correctness, security, and quality. They
check that the plugin makes sense, API calls are accurately declared, SKILL.md
is well-written, and there are no security concerns.

### Top 10 Rejection Reasons

| # | Reason | How to Avoid |
|---|--------|-------------|
| 1 | Missing `plugin.yaml` or `SKILL.md` | Both files are required in every plugin |
| 2 | Version mismatch between `plugin.yaml` and `SKILL.md` | Keep `version` identical in both files |
| 3 | Hardcoded API keys or credentials | Use environment variables, never commit secrets |
| 4 | No risk disclaimer for trading plugins | Add a disclaimer section in SKILL.md for any plugin that moves assets |
| 5 | Direct wallet operations without OnchainOS | Use `onchainos wallet` / `onchainos swap` for on-chain writes |
| 6 | Missing LICENSE file | Add a LICENSE file with an SPDX-compatible license |
| 7 | Unpinned dependencies | Pin all dependency versions; use lockfiles |
| 8 | Category mismatch | Choose the category that most accurately describes your plugin |
| 9 | SKILL.md missing required sections | Include Overview, Pre-flight, Commands, Error Handling, Security Notices |
| 10 | Auto-trading without dry-run mode | All automated trading plugins must support a dry-run / paper-trade mode |

### Common Lint Errors

| Code | Meaning | Fix |
|------|---------|-----|
| E001 | plugin.yaml not found | Ensure plugin.yaml is at the root of your plugin directory |
| E031 | Invalid name format | Lowercase letters, numbers, and hyphens only |
| E033 | Reserved prefix | Do not start your name with `okx-`, `official-`, or `plugin-store-` |
| E035 | Invalid version | Use semantic versioning: `1.0.0`, not `1.0` or `v1.0.0` |
| E041 | Missing LICENSE | Add a LICENSE file |
| E052 | Missing SKILL.md | Ensure SKILL.md exists in the directory specified by `components.skill.dir` |
| E065 | Missing api_calls | Add `api_calls` field to plugin.yaml (use `[]` if none) |
| E110 | Binary declared without build section | Add `build.lang`, `build.source_repo`, `build.source_commit` |
| E122 | Invalid source_repo format | Use `owner/repo` format, not full URL |
| E123 | Invalid source_commit | Must be full 40-character hex SHA from `git rev-parse HEAD` |
| E130 | Pre-compiled binary submitted | Remove binary files; submit source code, CI compiles it |

### Pre-Submission Checklist

Copy this into your PR description:

```markdown
- [ ] `plugin.yaml` and `SKILL.md` both present
- [ ] `name` field is lowercase with hyphens only, 2-40 characters
- [ ] `version` matches in both `plugin.yaml` and `SKILL.md`
- [ ] `author.github` matches my GitHub username
- [ ] `license` field uses a valid SPDX identifier
- [ ] `category` is one of the allowed values
- [ ] `api_calls` lists all external API domains (or `[]` if none)
- [ ] SKILL.md has YAML frontmatter with name, description, version, author
- [ ] SKILL.md includes Overview, Pre-flight, Commands, Error Handling sections
- [ ] No hardcoded API keys, tokens, or credentials anywhere
- [ ] No pre-compiled binary files in the submission
- [ ] LICENSE file is present
- [ ] PR title follows format: `[new-plugin] my-plugin v1.0.0`
- [ ] PR only modifies files inside `skills/my-plugin/`
- [ ] (If trading plugin) Risk disclaimer is included
- [ ] (If trading plugin) Dry-run / paper-trade mode is supported
- [ ] (If binary plugin) Source code compiles locally with CI-equivalent command
- [ ] Local lint passes: `cargo run --manifest-path cli/Cargo.toml -- lint skills/my-plugin`
```

---

## 7. Risk Levels

Every plugin is assigned one of three risk levels based on what it does.

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
   `meme-trench-scanner` plugin for a thorough example)
5. **Two reviewers** -- advanced plugins require approval from two maintainers

### Absolute Red Lines

The following will result in immediate rejection regardless of risk level:

1. **Hardcoded private keys or seed phrases** in any file
2. **Obfuscated or minified source code** that cannot be reviewed
3. **Network calls to undeclared domains** not listed in `api_calls`
4. **Prompt injection patterns** in SKILL.md (attempts to override agent safety)
5. **Exfiltration of user data** -- sending wallet addresses, balances, or
   trading history to external servers without explicit user consent
6. **Bypassing confirmation flows** -- executing transactions without user
   approval when the plugin declares `standard` risk level
7. **Unlimited autonomous trading** -- `advanced` plugins without stop-loss or
   max-amount safeguards
8. **Impersonation** -- using names, descriptions, or branding that falsely
   imply official endorsement by OKX or other organizations
9. **Pre-compiled binaries** -- submit source code; CI compiles it
10. **License violations** -- using code from incompatible licenses without
    attribution

---

## 8. FAQ

**How long does the review take?**

Automated checks complete in under 5 minutes. Human review typically takes
1-3 business days.

**Can I update my plugin after it is published?**

Yes. Modify your files, bump `version` in both `plugin.yaml` and `SKILL.md`,
and submit a new PR with the title `[update] my-plugin v1.1.0`.

**What are the plugin naming rules?**

Lowercase letters, numbers, and hyphens only. Between 2 and 40 characters.
No consecutive hyphens. No underscores. The `okx-` prefix is reserved for OKX
organization members.

**Can I use any programming language?**

For binary plugins, supported languages are Rust, Go, TypeScript, Node.js, and
Python. For Skill-only plugins, you can include scripts in any language (Python
and shell scripts are common) -- they run as part of the AI agent workflow, not
compiled by CI.

**Do I have to use OnchainOS?**

No. OnchainOS is recommended for blockchain operations but not required.
Non-blockchain plugins do not need it at all. Blockchain plugins that do not use
OnchainOS will go through additional security review.

**How do users install my plugin?**

After your PR is merged, users install via:

```bash
npx skills add okx/plugin-store --skill my-plugin
```

No plugin-store CLI installation is required on the user's end.

**What if the AI review flags something?**

The AI review is advisory only and does not block your PR. However, human
reviewers will read the AI report. Addressing flagged issues speeds up approval.

**My local lint passes but the GitHub check fails. Why?**

Ensure you are running the latest version of the plugin-store CLI. Also confirm
your PR only modifies files within `skills/your-plugin-name/`.

**The build failed in CI but compiles locally. Why?**

CI compiles on Ubuntu Linux. Ensure your code builds on Linux, not just macOS
or Windows. Check the build logs in the GitHub Actions run for specific errors.

**Where can I get help?**

- Open an [issue](https://github.com/okx/plugin-store/issues) on GitHub
- Look at existing plugins in `skills/` for working examples
- Run the lint command locally before submitting -- it catches most issues
- Check [GitHub Actions logs](https://github.com/okx/plugin-store/actions) if
  your PR checks fail
