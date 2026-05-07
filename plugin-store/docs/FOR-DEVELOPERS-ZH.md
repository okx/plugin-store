# Plugin 开发与提交指南

> 本指南将引导你为 Plugin Store 生态系统开发一个 Plugin 并提交审核。
> 完成后，你将拥有一个用户可通过
> `npx skills add okx/plugin-store --skill <name>` 安装的 Plugin。

---

## 目录

1. [什么是 Plugin？](#1-什么是-plugin)
2. [开始之前](#2-开始之前)
3. [快速入门（7 步）](#3-快速入门7-步)
4. [Plugin 结构](#4-plugin-结构)
5. [编写 SKILL.md](#5-编写-skillmd)
6. [编写 SUMMARY.md](#编写-summarymd)
7. [提交策略类 Plugin](#提交策略类-pluginstrategy)
8. [提交包含源代码的 Plugin(#编写-summarymd)
7. [提交包含源代码的 Plugin（Binary）](#6-提交包含源代码的-pluginbinary)
8. [三种提交模式](#7-三种提交模式)
9. [Onchain OS 集成](#8-onchainos-集成)
10. [审核流程](#9-审核流程)
11. [风险等级](#10-风险等级)
12. [规则与限制](#11-规则与限制)
13. [常见问题](#12-常见问题)

---

## 1. 什么是 Plugin？

Plugin 有一个必需的核心：**SKILL.md** —— 一份 Markdown 文档，教 AI 代理如何执行任务。可选地，它还可以包含一个 **Binary**（由我们的 CI 从你的源代码编译而成）。

**SKILL.md 始终是入口。** 即使你的 Plugin 包含 binary，Skill 也是告诉 AI 代理有哪些可用工具以及何时使用它们的核心。

### 两种类型的 Plugin

```
Type A: 仅 Skill（最常见，适用于任何开发者）
────────────────────────────────────────────────
  SKILL.md → 指导 AI → 调用 onchainos CLI
                       + 查询外部数据（免费）

Type B: Skill + Binary（适用于任何开发者，源代码由我们的 CI 编译）
────────────────────────────────────────────────
  SKILL.md → 指导 AI → 调用 onchainos CLI
                       + 调用你的 binary 工具
                       + 查询外部数据（免费）

  你的源代码（在你的 GitHub 仓库中）
    → 我们的 CI 编译
    → 用户安装我们编译的产物
```

Plugin **不限于 Web3**。你可以构建分析仪表板、开发者工具、交易策略、DeFi 集成、安全扫描器、NFT 工具，或任何其他受益于 AI 代理编排的应用。

| 我想要... | 类型 |
|-----------|------|
| 使用 AI agent 通过自然语言实现特定任务，比如调用 CLI 工具创建交易策略 | 仅 Skill |
| 构建一个 CLI 工具并附带 Skill | Skill + Binary（提交源代码，我们编译） |

### Binary Plugin 支持的语言

| 语言 | 构建工具 | 分发方式 |
|------|---------|---------|
| Rust | `cargo build --release` | 原生二进制文件 |
| Go | `go build` | 原生二进制文件 |
| TypeScript | Bun | Bun 全局安装 |
| Node.js | Bun | Bun 全局安装 |
| Python | `pip install` | pip 包 |

### 什么是好的 Plugin

- **实用** —— 解决实际问题或自动化繁琐的工作流程
- **安全** —— 不直接处理私钥，声明所有外部 API 调用，在适当的地方包含风险免责声明
- **文档完善** —— 清晰的 SKILL.md，包含具体示例、错误处理和预检步骤，以便 AI 代理可以从空白环境开始操作

---

## 2. 开始之前

### 前置条件

- **Git** 和 **GitHub 账户**
- **onchainos CLI** 已安装（推荐用于测试你的命令）：
  ```
  npx skills add okx/onchainos-skills
  ```
  安装后，如果找不到 `onchainos`，请将其添加到 PATH：
  ```
  export PATH="$HOME/.local/bin:$PATH"
  ```
- 对你的 Plugin 所涵盖领域有基本了解

> **注意：** plugin-store CLI 仅用于本地 lint，是可选的。用户通过
> `npx skills add okx/plugin-store --skill <name>` 安装你完成的 Plugin ——
> 用户端无需安装 CLI。

### 关键规则

1. **所有链上写操作应使用 onchainos CLI。** 这包括钱包签名、交易广播、swap 执行、合约调用和代币授权。你可以自由查询外部数据源（第三方 API、市场数据提供商、分析服务等）。
2. Onchain OS 是**可选的**。Plugin 可以自由使用任何链上技术。非区块链 Plugin 完全不需要它。

---

## 3. 快速入门（7 步）

本指南创建一个最小的仅 Skill 的 Plugin 并打包。

### 第 2 步：创建 Plugin 目录

```
mkdir -p skills/my-plugin
```

### 第 3 步：创建 plugin.yaml

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

### 第 4 步：创建 .claude-plugin/plugin.json

此文件使 Claude 能够将你的 Plugin 识别为一个 Skill。没有它，AI agent 将无法发现或安装你的 Plugin。

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

> **重要**：`name`、`description` 和 `version` 字段必须与你的 `plugin.yaml` 完全一致。

### 第 5 步：创建 SKILL.md

创建 `skills/my-plugin/SKILL.md`，内容如下：

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

### 第 6 步：本地验证

```
cd /path/to/plugin-store
plugin-store lint skills/my-plugin
```

如果全部通过：

```
Linting skills/my-plugin...

  Plugin 'my-plugin' passed all checks!
```

---

## 4. Plugin 结构

### 目录布局

```
skills/my-plugin/
├── .claude-plugin/
│   └── plugin.json      # 必需 -- Claude Skill 注册元数据
├── plugin.yaml          # 必需 -- Plugin 元数据和清单
├── SKILL.md             # 必需 -- AI 代理的 Skill 定义
├── SUMMARY.md           # 必需 -- 英文摘要（Overview、Prerequisites、Quick Start）
├── src/                 # 可选 -- 二进制 Plugin 源码
│   └── main.rs          #   Rust 示例（或 main.go、main.ts、main.py）
├── Cargo.toml           # 可选 -- 构建配置（或 go.mod、package.json、pyproject.toml）
├── scripts/             # 可选 -- Python 脚本、Shell 脚本
│   ├── bot.py
│   └── config.py
├── assets/              # 可选 -- HTML 仪表板、图片
│   └── dashboard.html
├── references/          # 可选 -- AI 代理的额外文档
│   └── api-reference.md
├── README.md            # 可选 -- 面向开发者的文档
└── LICENSE              # 推荐 -- SPDX 兼容的许可证文件
```

`.claude-plugin/plugin.json`、`plugin.yaml`、`SKILL.md` 和 `SUMMARY.md` 都是**必需的**。其他均为可选。

### .claude-plugin/plugin.json

此文件遵循 [Claude Skill 架构](https://docs.anthropic.com/en/docs/claude-code)，是 Plugin 注册所必需的。它必须与你的 `plugin.yaml` 保持一致。

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

| 字段 | 必需 | 说明 |
|------|------|------|
| `name` | 是 | 必须与 `plugin.yaml` 中的 name 一致 |
| `description` | 是 | 必须与 `plugin.yaml` 中的 description 一致 |
| `version` | 是 | 必须与 `plugin.yaml` 中的 version 一致（semver） |
| `author` | 是 | 名称和可选的邮箱 |
| `license` | 是 | SPDX 标识符（MIT、Apache-2.0 等） |
| `keywords` | 否 | 可搜索的标签 |
| `homepage` | 否 | 项目主页 URL |
| `repository` | 否 | 源代码 URL |

### plugin.yaml 参考

按提交模式分类的示例，选择适合你的用例。

#### 模式 A：仅 Skill（无源码）

最简单的形式——仅用 SKILL.md 编排现有 CLI 工具和命令。

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

#### 模式 A：Skill + 源码（编译二进制，本地源码）

源码直接在 Plugin 目录中。CI 从这里编译——不需要外部仓库。

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
  binary_name: my-tool  # 编译输出名称

api_calls: []
```

目录结构：
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

#### 模式 B：外部仓库（源码在你自己的仓库中）

源码保留在你自己的 GitHub 仓库，固定到特定 commit。CI 在编译时会 clone 你的仓库到指定 commit 进行编译。

**重要：你的 PR 中必须包含以下文件（在 `skills/<name>/` 目录下）：**

| 文件 | 必需 | 说明 |
|------|:----:|------|
| `plugin.yaml` | 是 | 必须包含 `build.source_repo`、`build.source_commit`、`build.source_dir` |
| `SKILL.md` | **是** | 必须包含完整的命令文档。CI 无法从你的远端仓库获取 SKILL.md——它必须在 PR 中提交。CI 会自动注入 pre-flight 安装块。 |
| `LICENSE` | 是 | SPDX 兼容的许可证文件 |
| `.claude-plugin/plugin.json` | 是 | 标准 Claude skill 元数据 |

> **为什么 SKILL.md 必须在 PR 中？** CI 需要向 SKILL.md 注入 pre-flight 安装块（onchainos CLI、二进制下载、遥测上报），并将修改后的文件推送回你的 PR 分支。如果 SKILL.md 在远端仓库，CI 没有推送权限无法写回。此外，SKILL.md 必须在 PR diff 中可审查。

```
skills/defi-yield-optimizer/
├── .claude-plugin/
│   └── plugin.json
├── plugin.yaml
├── SKILL.md              ← 完整的命令文档（必须在 PR 中提交）
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
    dir: "."              # SKILL.md 在插件目录中，不在远端

build:
  lang: rust
  source_repo: "defi-builder/yield-optimizer"    # 你的 GitHub 仓库
  source_commit: "a1b2c3d4e5f6789012345678901234567890abcd"  # 固定 commit
  source_dir: "defi-yield"                       # 你仓库中的子目录（用于 monorepo）
  binary_name: defi-yield

api_calls:
  - "api.defillama.com"
```

`build.source_dir` 字段告诉 CI 你的仓库中哪个子目录包含 `Cargo.toml`（或 `go.mod`）。这对 **monorepo** 很重要——CI 只会编译该子目录，而不是整个仓库。

#### 逐字段参考

| 字段 | 必需 | 说明 | 规则 |
|------|------|------|------|
| `schema_version` | 是 | Schema 版本 | 始终为 `1` |
| `name` | 是 | Plugin 名称 | 小写 `[a-z0-9-]`，2-40 个字符，不可连续使用连字符 |
| `version` | 是 | Plugin 版本 | 语义化版本号 `x.y.z`（引号字符串） |
| `description` | 是 | 一句话摘要 | 不超过 200 个字符 |
| `author.name` | 是 | 作者显示名 | 你的名字或组织名 |
| `author.github` | 是 | GitHub 用户名 | 必须与 PR 作者一致 |
| `author.email` | 否 | 联系邮箱 | 用于安全通知 |
| `license` | 是 | 许可证标识符 | SPDX 格式：`MIT`、`Apache-2.0`、`GPL-3.0` 等 |
| `category` | 是 | Plugin 类别 | 以下之一：`strategy`、`defi-protocol`、`analytics`、`utility`、`security`、`wallet`、`nft` |
| `tags` | 否 | 搜索关键词 | 字符串数组 |
| `type` | 否 | 作者类型 | `"official"`、`"dapp-official"`、`"community-developer"` |
| `github_link` | 否 | 项目 GitHub 地址 | URL，在市场中展示 |
| `components.skill.dir` | 模式 A | Skill 目录路径 | Plugin 目录内的相对路径（使用 `"."` 表示根目录） |
| `components.skill.repo` | 模式 B | 外部仓库 | 格式：`owner/repo` |
| `components.skill.commit` | 模式 B | 固定的 commit | 完整的 40 个字符的十六进制 SHA |
| `build.lang` | 仅 Binary | 源代码语言 | `rust` / `go` / `typescript` / `node` / `python` |
| `build.source_repo` | 仅 Binary | 源代码仓库 | 格式：`owner/repo` |
| `build.source_commit` | 仅 Binary | 固定的 commit SHA | 完整的 40 个字符的十六进制 SHA；通过 `git rev-parse HEAD` 获取 |
| `build.source_dir` | 否 | 源代码子目录 | 仓库内的路径，默认为 `.` |
| `build.binary_name` | 仅 Binary | 输出的二进制名称 | 必须与编译器产生的名称一致 |
| `build.main` | TS/Node/Python | 入口文件 | 例如 `src/index.js` 或 `src/main.py` |
| `api_calls` | 否 | 外部 API 域名 | Plugin 调用的域名字符串数组 |

#### 命名规则

- **允许**：`solana-price-checker`、`defi-yield-optimizer`、`nft-tracker`
- **禁止**：`OKX-Plugin`（大写）、`my_plugin`（下划线）、`a`（太短）
- **保留前缀**：只有 OKX 组织成员可以使用 `okx-` 前缀

---

## 5. 编写 SKILL.md

SKILL.md 是你的 Plugin 的**唯一入口**。它教 AI 代理你的 Plugin 做什么以及如何使用它。对于仅 Skill 的 Plugin，它编排命令和工具。对于 Binary Plugin，它还编排你的自定义工具。

```
仅 Skill 的 Plugin：
  SKILL.md → onchainos 命令

Binary Plugin：
  SKILL.md → onchainos 命令
           + 你的 binary 工具（calculate_yield、find_route 等）
           + 你的 binary 命令（my-tool start、my-tool status 等）
```

### SKILL.md 模板（仅 Skill）

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

### SKILL.md 模板（Binary Plugin）

如果你的 Plugin 包含 binary，SKILL.md 必须告诉 AI 代理 **onchainos 命令**和你的**自定义 binary 工具**：

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

### SKILL.md 作为编排器

你的 SKILL.md 告诉 AI 代理如何使用你 Plugin 的命令和工具：

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

### SKILL.md Frontmatter 字段

| 字段 | 必需 | 说明 |
|------|------|------|
| `name` | 是 | 必须与 plugin.yaml 中的 `name` 一致 |
| `description` | 是 | 简要说明（应与 plugin.yaml 一致） |
| `version` | 是 | 必须与 plugin.yaml 中的 `version` 一致 |
| `author` | 是 | 作者名 |
| `tags` | 否 | 用于可发现性的关键词 |

### SKILL.md 必需章节

- **Overview** —— skill 的功能
- **Pre-flight Checks** —— 前置条件、依赖安装（必须能从空白环境运行）
- **Commands** —— 每个命令的使用时机、输出说明和具体示例
- **Error Handling** —— 错误、原因和解决方案表格
- **Security Notices** —— 风险等级、免责声明

### SKILL.md 最佳实践

1. **具体明确** —— `onchainos token search --query SOL --chain solana` 比"搜索代币"更好
2. **始终包含错误处理** —— AI 代理需要知道失败时该怎么做
3. **使用 skill 路由** —— 告诉 AI 何时应转交给其他 skill
4. **包含预检步骤** —— 依赖安装命令，以便 AI 代理可以从零开始设置
5. **不要重复现有工具的功能** —— 编排而非替换

### 好的与坏的示例

**坏的：模糊的指令**
```
Use onchainos to get the price.
```

**好的：具体且可操作**
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

## 编写 SUMMARY.md

每个 Plugin **必须**包含一个 `SUMMARY.md` 文件，放在与 `SKILL.md` 同级的目录中。该文件是**英文**的面向用户摘要，必须包含固定的三段式结构。CI lint 会在提交时检查：缺少文件报 `E150` 错误，缺少必需章节报 `E151` 错误。

### 必需结构

```markdown
# <plugin-name>

## Overview

<一句话概括该平台/协议是什么。>

Core operations:

- <核心操作 1，例如 "Swap tokens across 500+ DEX sources">
- <核心操作 2，例如 "Query real-time portfolio balances">
- <核心操作 3，例如 "Place limit orders with slippage protection">

Tags: `defi` `ethereum` `swap` `lending`

## Prerequisites

- <IP/地区限制，例如 "No IP restrictions" 或 "US users excluded">
- <支持的链，例如 "Ethereum, Base, Arbitrum, Polygon">
- <支持的代币，例如 "All ERC-20 tokens" 或 "USDC, WETH, DAI">
- <所需工具，例如 "onchainos CLI installed and authenticated">
- <其他前置条件>

## Quick Start

<用通俗语言引导用户完成基本流程，描述关键步骤、模式或选项。>

1. **步骤标题**: 做什么，为什么。
2. **步骤标题**: 下一步操作。
3. **步骤标题**: 如何验证一切正常。
```

### 示例：Polymarket Plugin

```markdown
# polymarket-plugin

## Overview

Polymarket is a decentralized prediction market platform on Polygon where users trade
outcome shares on real-world events.

Core operations:

- Browse and search prediction markets by topic
- Buy and sell YES/NO outcome shares (market orders and limit orders)
- Check portfolio positions and P&L
- Manage open orders (cancel, modify)
- Claim winnings after market settlement

Tags: `prediction-market` `polygon` `trading` `polymarket`

## Prerequisites

- US users are restricted from trading on Polymarket
- Supported chain: Polygon (MATIC)
- Supported tokens: USDC.e for trading, POL for gas fees
- onchainos CLI installed and authenticated
- A funded wallet with USDC.e on Polygon

## Quick Start

1. **选择交易模式**
   直连模式：直接用你的钱包下单，最简单直接。买入时 Agent 会自动完成授权，需少量 POL 作为手续费。
   代理模式：Agent 帮你创建专属代理钱包，交易手续费由 Polymarket 承担。首次使用需完成设置并充值 USDC.e，之后可无感交易。两种模式随时可切换。

2. **余额查询与充值**
   输入"查看余额"即可看到所有钱包的 POL 和 USDC.e 数量。代理钱包余额不足时，向代理地址转入 USDC.e 即可，同普通转账。

3. **选择市场**
   告诉 Agent 你感兴趣的话题（如"帮我找 BTC 涨跌相关的市场"），它会搜索匹配的预测市场并展示价格、成交量和截止时间，帮你判断是否参与。

4. **下单交易**
   立即成交：按当前最优价买入或卖出，全部成交或自动取消，适合想快速交易的场景。
   挂单等待：设定期望价格等待成交，适合不急、想拿更好价格的场景。可设置有效期或仅做挂单方。
   示例："花 10 USDC.e 买入 Yes" / "以 0.35 挂单买 100 份 No"

5. **管理持仓**
   随时让 Agent 查看当前持仓和盈亏，取消未成交的挂单，或在市场结算后提取收益。代理模式首次启用时会自动完成合约授权，后续无需重复。
```

### 关键规则

- **语言**: SUMMARY.md 必须用**英文**编写。
- **章节**: 三个章节（`## Overview`、`## Prerequisites`、`## Quick Start`）全部必需。
- **标签**: 使用行内代码块（`` `tag-name` ``）展示标签，放在 Overview 章节末尾。
- **位置**: SUMMARY.md 必须与 SKILL.md 放在同一目录。

---

---

## 提交策略类 Plugin（Strategy）

**策略类 plugin**（`category: strategy`）不直接连接链或钱包，而是调用已有的交易插件（如 polymarket-plugin、raydium-plugin）来执行订单。策略作者可被归因和激励。

### 策略 Plugin 的 plugin.yaml

```yaml
schema_version: 1
name: my-arb-strategy
version: "1.0.0"
description: "Arbitrage between Raydium and Orca"
author:
  name: "alice"
  github: "alice"
license: MIT
category: strategy              # 必须为 "strategy"
tags:
  - solana
  - arbitrage

dependent_plugin:                # 必填 — 声明本策略调用的交易插件
  - name: raydium-plugin
    version: "^0.2.0"

risk_level: high                 # 信息性字段，展示给用户
supported_venues:                # 信息性字段，用于搜索/筛选
  - raydium

components:
  skill:
    dir: skills/my-arb-strategy

api_calls: []
```

### --strategy-id 标识要求

对依赖插件的所有**交易操作**（buy、sell、swap、order）**必须**包含 `--strategy-id <策略名>` 以实现归因追踪：

```python
# ✅ 正确 — 写操作带 --strategy-id
subprocess.run(["raydium-plugin", "swap", "--from", "USDC", "--to", "SOL",
                "--amount", "10", "--strategy-id", "my-arb-strategy", "--confirm"])

# ✅ 正确 — 只读操作不需要 --strategy-id
subprocess.run(["raydium-plugin", "quote", "--token", "SOL"])

# ✅ 正确 — deposit/withdraw 不需要 --strategy-id
subprocess.run(["raydium-plugin", "deposit", "--amount", "100", "--pool", "SOL-USDC"])

# ❌ 错误 — 交易操作缺少 --strategy-id（AI 审查会拒绝）
subprocess.run(["raydium-plugin", "swap", "--from", "USDC", "--to", "SOL",
                "--amount", "10", "--confirm"])
```

### 策略 Plugin 的 CI 检查

| 阶段 | 检查项 | 失败行为 |
|------|--------|----------|
| Phase 1 | `dependent_plugin[].name` 在 registry 中存在（E160） | PR 拦截 |
| Phase 3 | AI 扫描所有写操作是否带 `--strategy-id` | 标记为 Critical |
| Phase 3 | 禁止硬编码私钥、RPC URL、API key | 标记为 Critical |


## 6. 提交包含源代码的 Plugin（Binary）

> **重要：** SKILL.md 始终是入口。即使你的 Plugin 包含 binary，
> SKILL.md 也是告诉 AI 代理如何编排一切的核心 ——
> onchainos 命令、你的 binary 工具和你的 binary 命令。

### 谁可以提交源代码？

任何开发者都可以提交源代码进行 binary 编译。将你的源代码提交到你自己的 GitHub 仓库，在 plugin.yaml 中添加 `build` 部分，我们的 CI 将编译它。

### 工作原理

```
你提交源代码 → 我们的 CI 编译 → 用户安装我们编译的产物
你永远不提交二进制文件。我们永远不修改你的源代码。
```

### 带构建配置的 plugin.yaml

你的源代码保留在你自己的 GitHub 仓库中。你提供仓库 URL 和一个固定的 commit SHA —— 我们的 CI 在该确切 commit 处克隆、编译并发布。commit SHA 是内容指纹：相同的 SHA = 相同的代码，保证一致。

### 如何获取 Commit SHA

你的源代码必须先推送到 GitHub，**然后**才能获取有效的 commit SHA。工作流程如下：

```
# 1. 在你的源代码仓库中 -- 先开发并推送你的代码
cd your-source-repo
git add . && git commit -m "v1.0.0"
git push origin main

# 2. 获取完整的 40 个字符的 commit SHA
git rev-parse HEAD
# 输出：a1b2c3d4e5f6789012345678901234567890abcd

# 3. 将此 SHA 复制到你的 plugin.yaml 的 build.source_commit 字段中
```

> commit 必须存在于 GitHub 上（不仅是本地）。我们的 CI 会从 GitHub 在该确切 SHA 处克隆。

### 各语言必需的源代码目录结构

**你的仓库必须能用一条标准命令编译。** 不允许自定义脚本，不允许多步骤构建。我们的 CI 每种语言只运行一条构建命令。

**Rust：**
```
your-org/your-tool/
├── Cargo.toml          ← 必须包含 [[bin]]，name 需与 binary_name 一致
├── Cargo.lock           ← 提交此文件（可复现构建）
└── src/
    └── main.rs          ← 你的代码
```

`Cargo.toml` 必须包含：
```toml
[package]
name = "your-tool"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "your-tool"      # ← 必须与 plugin.yaml 中的 build.binary_name 一致
path = "src/main.rs"
```

**Go：**
```
your-org/your-tool/
├── go.mod               ← 必须有 module 声明
├── go.sum               ← 提交此文件
└── main.go              ← 必须有 package main + func main()
```

**TypeScript：**
```
your-org/your-tool/
├── package.json         ← 必须有 name、version 和 "bin" 字段
└── src/
    └── index.js         ← 编译为 JS，第一行必须是 #!/usr/bin/env node
```

> **重要：** TypeScript plugin 通过 `bun install -g` 分发，不编译为二进制文件。你的 `package.json` 必须包含指向 JS 入口文件的 `"bin"` 字段，入口文件必须以 `#!/usr/bin/env node` 开头。如果你使用 TypeScript 编写，请在推送到源代码仓库之前编译为 JS。

`package.json` 示例：
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

**Node.js：**
```
your-org/your-tool/
├── package.json         ← 必须有 name、version 和 "bin" 字段
└── src/
    └── index.js         ← 第一行必须是 #!/usr/bin/env node
```

> **重要：** Node.js plugin 通过 `bun install -g` 分发，不编译为二进制文件。你的 `package.json` 必须包含 `"bin"` 字段，入口文件必须以 `#!/usr/bin/env node` 开头。

`package.json` 示例：
```json
{
  "name": "your-tool",
  "version": "1.0.0",
  "bin": {
    "your-tool": "src/index.js"
  }
}
```

**Python：**
```
your-org/your-tool/
├── pyproject.toml       ← 必须有 [build-system]、[project]（含 name、version）和 [project.scripts]
├── setup.py             ← 推荐，兼容旧版 pip
└── src/
    ├── __init__.py
    └── main.py          ← 此路径填入 build.main；必须有 def main() 入口
```

> **重要：** Python plugin 通过 `pip install` 分发，不编译为二进制文件。你的 `pyproject.toml` 必须包含 `[project.scripts]` 来定义 CLI 入口点。我们建议同时提供 `setup.py` 以兼容旧版 pip。

`pyproject.toml` 示例：
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

### 构建配置 -- 各语言完整示例

每个 `build` 字段说明：

| 字段 | 必需 | 说明 |
|------|------|------|
| `lang` | 是 | `rust` \| `go` \| `typescript` \| `node` \| `python` |
| `source_repo` | 是 | GitHub `owner/repo`，包含你的源代码 |
| `source_commit` | 是 | 完整的 40 个字符的 commit SHA（通过 `git rev-parse HEAD` 获取） |
| `source_dir` | 否 | 仓库内的源代码根路径（默认：`.`） |
| `entry` | 否 | 入口文件覆盖（默认：按语言自动检测） |
| `binary_name` | 是 | 编译输出的二进制名称 |
| `main` | TS/Node/Python | 入口文件（例如 `src/index.js`、`src/main.py`） |
| `targets` | 否 | 限制构建平台（默认：所有支持的平台） |

#### Rust

```yaml
build:
  lang: rust
  source_repo: "your-org/your-rust-tool"
  source_commit: "a1b2c3d4e5f6789012345678901234567890abcd"
  source_dir: "."                        # 默认值，可省略
  entry: "Cargo.toml"                    # Rust 默认值，可省略
  binary_name: "your-tool"              # 必须与 Cargo.toml 中的 [[bin]] name 一致
  targets:                               # 可选，省略则为所有平台
    - x86_64-unknown-linux-gnu
    - aarch64-apple-darwin
```

CI 运行：`cargo fetch` -> `cargo audit` -> `cargo build --release`
输出：原生二进制文件（约 5-20MB）

#### Go

```yaml
build:
  lang: go
  source_repo: "your-org/your-go-tool"
  source_commit: "b2c3d4e5f6789012345678901234567890abcdef"
  source_dir: "."
  entry: "go.mod"                        # Go 默认值，可省略
  binary_name: "your-tool"
  targets:
    - x86_64-unknown-linux-gnu
    - aarch64-apple-darwin
```

CI 运行：`go mod download` -> `govulncheck` -> `CGO_ENABLED=0 go build -ldflags="-s -w"`
输出：静态原生二进制文件（约 5-15MB）

#### TypeScript

```yaml
build:
  lang: typescript
  source_repo: "your-org/your-ts-tool"
  source_commit: "c3d4e5f6789012345678901234567890abcdef01"
  source_dir: "."
  binary_name: "your-tool"
  main: "src/index.js"                   # 必需 -- 必须是 JS（不是 .ts）
```

分发方式：`bun install -g`
依赖：Bun
输出大小：约 KB（源码安装，无需下载大型二进制文件）

> **注意：** `package.json` 必须包含 `"bin"` 字段，入口文件必须以 `#!/usr/bin/env node` 开头。如果使用 TypeScript 编写，请在推送到源代码仓库之前编译为 JS。

#### Node.js

```yaml
build:
  lang: node
  source_repo: "your-org/your-node-tool"
  source_commit: "e5f6789012345678901234567890abcdef012345"
  source_dir: "."
  binary_name: "your-tool"
  main: "src/index.js"                   # Node.js 必需
```

分发方式：`bun install -g`
依赖：Bun
输出大小：约 KB（源码安装）

> **注意：** `package.json` 必须包含 `"bin"` 字段，入口文件必须以 `#!/usr/bin/env node` 开头。

> Node.js 和 TypeScript 使用相同的分发方式（bun install）。唯一的区别是 TypeScript 必须先编译为 JS。

#### Python

```yaml
build:
  lang: python
  source_repo: "your-org/your-python-tool"
  source_commit: "d4e5f6789012345678901234567890abcdef0123"
  source_dir: "."
  binary_name: "your-tool"
  main: "src/main.py"                    # Python 必需
```

分发方式：`pip install`
依赖：Python 3.8+ 和 pip/pip3
输出大小：约 KB（源码安装）

> **注意：** `pyproject.toml` 必须包含 `[build-system]`、`[project]` 和 `[project.scripts]`。我们建议同时提供 `setup.py` 以兼容旧版 pip。入口函数必须是 `def main()`。

### 本地构建验证

提交前，请验证你的代码能用我们 CI 使用的确切命令编译：

```
# Rust
cd your-tool && cargo build --release
# 验证：target/release/your-tool 存在

# Go
cd your-tool && CGO_ENABLED=0 go build -o your-tool -ldflags="-s -w" .
# 验证：./your-tool 存在

# TypeScript / Node.js
cd your-tool && bun install -g .
# 验证：your-tool --help 运行成功
# 注意：package.json 必须有 "bin" 字段，入口文件必须有 #!/usr/bin/env node

# Python
cd your-tool && pip install .
# 验证：your-tool --help 运行成功
# 注意：pyproject.toml 必须有 [project.scripts]，入口函数必须是 def main()
```

如果这些命令在本地失败，我们的 CI 也会失败。

### 常见构建失败

| 问题 | 原因 | 修复 |
|------|------|------|
| "Binary not found" | `binary_name` 与编译输出不匹配 | Rust：检查 Cargo.toml 中的 `[[bin]] name`。Go：检查 `-o` 标志。 |
| "source_commit is not valid" | 使用了短 SHA 或分支名 | 使用完整的 40 个字符：`git rev-parse HEAD` |
| "source_repo format invalid" | 格式错误 | 必须是 `owner/repo`，不是 `https://github.com/...` |
| 构建失败但本地正常 | 平台差异 | 我们的 CI 运行 Ubuntu Linux。确保你的代码在 Linux 上可编译。 |
| Cargo.lock not found | 未提交 | 运行 `cargo generate-lockfile` 并提交 `Cargo.lock`。 |
| Python import error | 缺少依赖 | 确保所有依赖都在 `pyproject.toml` 或 `requirements.txt` 中。 |

### 不允许的操作（Binary Plugin）

- 提交预编译的二进制文件（.exe、.dll、.so、.wasm）—— E130
- 声明 Binary 但没有 build 部分 —— E110/E111
- 源代码大于 10MB —— E126
- 包含在编译期间从互联网下载内容的构建脚本

---

## 7. 三种提交模式

### 模式 A -- 直接提交（推荐）

所有内容都放在 plugin-store 仓库的 `skills/<name>/` 目录内。这是最简单的方式，推荐大多数 Plugin 使用。

**纯 Skill Plugin：**

```
skills/my-plugin/
├── .claude-plugin/
│   └── plugin.json   # 必需
├── plugin.yaml       # 必需
├── SKILL.md          # 必需
├── scripts/          # 可选 -- Python/Shell 脚本
├── assets/           # 可选 -- HTML、图片
├── LICENSE
└── README.md
```

**包含源码的 Plugin（如 Rust CLI）：**

```
skills/my-rust-tool/
├── .claude-plugin/
│   └── plugin.json   # 必需
├── plugin.yaml       # 必需（含 build 配置，无需 source_repo）
├── SKILL.md          # 必需
├── Cargo.toml        # 构建配置（或 go.mod、package.json、pyproject.toml）
├── Cargo.lock
├── src/
│   └── main.rs       # 源码 -- CI 直接从此目录编译
├── LICENSE
└── README.md
```

plugin.yaml 使用 `components.skill.dir`，并可选添加 `build` 配置：

```yaml
components:
  skill:
    dir: "."

# 二进制 Plugin（省略 source_repo 表示使用本地源码）：
build:
  lang: rust            # rust | go | typescript | node | python
  binary_name: my-tool  # 编译输出名称
```

**适用场景**：你希望所有内容集中在一处。适用于纯 Skill Plugin、包含脚本的 Plugin，以及包含编译源码的 Plugin。CI 直接从 Plugin 目录编译——不需要外部仓库。

### 模式 B -- 外部仓库

你的 plugin.yaml 指向你自己的 GitHub 仓库，带有固定的 commit SHA。只有 `plugin.yaml`（以及可选的 `LICENSE`、`README.md`）放在 plugin-store 仓库中。

```
skills/my-plugin/
├── plugin.yaml       # 指向你的外部仓库
└── LICENSE
```

plugin.yaml 使用 `components.skill.repo` 和 `components.skill.commit`：

```yaml
components:
  skill:
    repo: "your-username/my-plugin"
    commit: "d2aa628e063d780c370b0ec075a43df4859be951"
```

commit 必须是**完整的 40 个字符的 SHA**（不是短 SHA 或分支名）。通过以下命令获取：

```
cd your-source-repo
git push origin main
git rev-parse HEAD
# 输出：d2aa628e063d780c370b0ec075a43df4859be951
```

**适用场景**：你的 Plugin 有大量源代码，你想将其保留在自己的仓库中，或者你想要独立的版本管理。`meme-trench-scanner` 和 `smart-money-signal-copy-trade` 等 Plugin 就使用了这种方式。

---

## 8. Onchain OS 集成

### 什么是 Onchain OS？

[Onchain OS](https://github.com/okx/onchainos-skills) 内嵌了 Agentic Wallet CLI，提供安全的沙箱化区块链操作 —— 钱包签名、交易广播、swap 执行、合约调用等。它使用 TEE（可信执行环境）签名，因此私钥永远不会离开安全隔区。

### 何时使用 Onchain OS

当你的 Plugin 执行任何链上写操作时使用 Onchain OS：

- 钱包签名
- 交易广播
- Swap 执行
- 合约调用
- 代币授权

### Onchain OS 是否必须？

**不是。Onchain OS 是可选的。** Plugin 可以自由使用任何链上技术——Onchain OS、第三方钱包、直接 RPC 调用，或任何其他方式。

对于非区块链 Plugin（分析、工具、开发者工具等），Onchain OS 不适用。

### Onchain OS 命令参考

| 命令 | 说明 | 示例 |
|------|------|------|
| `onchainos token` | 代币搜索、信息、趋势、持有者 | `onchainos token search --query SOL --chain solana` |
| `onchainos market` | 价格、K 线图、组合盈亏 | `onchainos market price --address 0x... --chain ethereum` |
| `onchainos swap` | DEX swap 报价和执行 | `onchainos swap quote --from ETH --to USDC --amount 1` |
| `onchainos gateway` | Gas 估算、交易模拟、广播 | `onchainos gateway gas --chain ethereum` |
| `onchainos portfolio` | 钱包总价值和余额 | `onchainos portfolio all-balances --address 0x...` |
| `onchainos wallet` | 登录、余额、转账、历史 | `onchainos wallet balance --chain solana` |
| `onchainos security` | 代币扫描、DApp 扫描、交易扫描 | `onchainos security token-scan --address 0x...` |
| `onchainos signal` | 聪明钱/鲸鱼信号 | `onchainos signal list --chain solana` |
| `onchainos memepump` | Meme 代币扫描和分析 | `onchainos memepump tokens --chain solana` |
| `onchainos leaderboard` | 按盈亏/交易量排名的顶级交易者 | `onchainos leaderboard list --chain solana` |
| `onchainos payment` | x402 支付协议 | `onchainos payment x402-pay --url ...` |

完整子命令列表请运行 `onchainos <command> --help` 或查看 [onchainos 文档](https://github.com/okx/onchainos-skills)。

### 安装 Onchain OS

```
npx skills add okx/onchainos-skills
```

如果安装后找不到 `onchainos`，请将其添加到 PATH：

```
export PATH="$HOME/.local/bin:$PATH"
```

---

## 9. 审核流程

每个 Pull Request 都会经过 4 阶段的 CI 流水线。

### 阶段 1：静态 Lint（自动，即时）

验证 Plugin 结构、命名规范、版本格式、必需文件和安全默认值。结果作为 PR 评论发布。

如果 lint 失败，PR 将被阻止。修复问题后重新推送。

### 阶段 2：构建验证（自动，仅 Binary Plugin）

如果你的 Plugin 有 `build` 部分，CI 会在固定的 commit SHA 处克隆你的源代码仓库，编译代码，并验证二进制文件是否可运行。构建失败将阻止 PR。

### 阶段 3：AI 代码审查（手动触发，约 2 分钟）

AI 审查员阅读你的 Plugin 并生成涵盖安全性、合规性和质量的 8 个部分的报告。报告作为可折叠的 PR 评论发布。此阶段**仅供参考** —— 它不会阻止合并，但人工审查员会阅读该报告。

### 阶段 4：摘要和预检（手动触发）

生成所有先前阶段的摘要。预检步骤会自动注入以下内容到测试环境：

- **onchainos CLI** —— Agentic Wallet CLI
- **Skills** —— 你的 Plugin 的 skill 文件
- **plugin-store Skill** —— plugin-store skill 本身
- **HMAC install report** —— 一份签名报告，确认安装完整性

这确保每个 Plugin 都可以在真实环境中进行端到端验证。

### 人工审查（1-3 个工作日）

所有自动化阶段通过后，维护者将审查 Plugin 的正确性、安全性和质量。他们会检查 Plugin 是否合理，API 调用是否准确声明，SKILL.md 是否编写良好，以及是否存在安全问题。

### 常见拒绝原因

| # | 原因 | 如何避免 |
|---|------|---------|
| 1 | 缺少 `plugin.yaml`、`.claude-plugin/plugin.json` 或 `SKILL.md` | 每个 Plugin 都必须包含这三个文件 |
| 2 | `plugin.yaml` 和 `SKILL.md` 之间的版本不匹配 | 在两个文件中保持 `version` 相同 |
| 3 | 硬编码的 API 密钥或凭据 | 使用环境变量，永远不要提交密钥 |
| 4 | 交易类 Plugin 缺少风险免责声明 | 在 SKILL.md 中为任何涉及资产转移的 Plugin 添加免责声明部分 |
| 5 | 不使用 Onchain OS 进行直接钱包操作 | 使用 `onchainos wallet` / `onchainos swap` 进行链上写操作 |
| 6 | 缺少 LICENSE 文件 | 添加包含 SPDX 兼容许可证的 LICENSE 文件 |
| 7 | 未固定的依赖 | 固定所有依赖版本；使用 lockfile |
| 8 | 类别不匹配 | 选择最准确描述你的 Plugin 的类别 |
| 9 | SKILL.md 缺少必需章节 | 包含 Overview、Pre-flight、Commands、Error Handling、Security Notices |
| 10 | 自动交易缺少 dry-run 模式 | 所有自动化交易 Plugin 必须支持 dry-run / 模拟交易模式 |

### 常见 Lint 错误

| 代码 | 含义 | 修复 |
|------|------|------|
| E001 | 未找到 plugin.yaml | 确保 plugin.yaml 位于你的 plugin 目录根目录 |
| E031 | 无效的名称格式 | 仅允许小写字母、数字和连字符 |
| E033 | 保留前缀 | 只有 OKX 组织成员可以使用 `okx-` 前缀 |
| E035 | 无效的版本 | 使用语义化版本号：`1.0.0`，不是 `1.0` 或 `v1.0.0` |
| E041 | 缺少 LICENSE | 添加 LICENSE 文件 |
| E052 | 缺少 SKILL.md | 确保 SKILL.md 存在于 `components.skill.dir` 指定的目录中 |
| E065 | 缺少 api_calls | 在 plugin.yaml 中添加 `api_calls` 字段（如果没有则使用 `[]`） |
| E110 | 声明了 Binary 但没有 build 部分 | 添加 `build.lang`、`build.source_repo`、`build.source_commit` |
| E122 | 无效的 source_repo 格式 | 使用 `owner/repo` 格式，不是完整 URL |
| E123 | 无效的 source_commit | 必须是通过 `git rev-parse HEAD` 获取的完整 40 个字符的十六进制 SHA |
| E130 | 提交了预编译的二进制文件 | 删除二进制文件；提交源代码，CI 会编译 |

### 提交前检查清单

将以下内容复制到你的 PR 描述中：

```markdown
- [ ] `plugin.yaml`、`.claude-plugin/plugin.json` 和 `SKILL.md` 均已存在
- [ ] `name` 字段为小写加连字符，2-40 个字符
- [ ] `plugin.yaml`、`plugin.json` 和 `SKILL.md` 中的 `version` 一致
- [ ] `author.github` 与我的 GitHub 用户名一致
- [ ] `license` 字段使用有效的 SPDX 标识符
- [ ] `category` 是允许值之一
- [ ] `api_calls` 列出了所有外部 API 域名（如果没有则为 `[]`）
- [ ] SKILL.md 有包含 name、description、version、author 的 YAML frontmatter
- [ ] SKILL.md 包含 Overview、Pre-flight、Commands、Error Handling 章节
- [ ] 任何地方都没有硬编码的 API 密钥、令牌或凭据
- [ ] 提交中没有预编译的二进制文件
- [ ] LICENSE 文件已存在
- [ ] 提交仅修改 `skills/my-plugin/` 内的文件
- [ ] （如果是交易类 plugin）包含风险免责声明
- [ ] （如果是交易类 plugin）支持 dry-run / 模拟交易模式
- [ ] （如果是 binary plugin）源代码能用 CI 等效命令在本地编译
- [ ] 本地 lint 通过：`plugin-store lint skills/my-plugin`
```

---

## 10. 风险等级

每个 Plugin 根据其功能被分配三个风险等级之一。

| 等级 | 名称 | 定义 | 额外要求 |
|------|------|------|---------|
| `starter` | 入门级 | 只读操作，无资产转移 | 标准审查 |
| `standard` | 标准级 | 交易需每次明确的用户确认 | 标准审查 + 确认流程检查 |
| `advanced` | 高级 | 自动化策略，可能自主运行 | 见下文 |

### 高级风险等级要求

`advanced` 风险等级的 Plugin 必须包含以下所有内容：

1. **Dry-run / 模拟交易模式** —— 必须是默认模式或有清晰的文档说明
2. **止损机制** —— 可配置的最大亏损阈值
3. **最大金额限制** —— 可配置的单笔交易和单次会话上限
4. **风险免责声明** —— SKILL.md 中的醒目免责声明（参见 `meme-trench-scanner` Plugin 的详尽示例）
5. **两名审查员** —— 高级 Plugin 需要两名维护者批准

### 绝对红线

以下情况将导致立即拒绝，无论风险等级如何：

1. **硬编码的私钥或助记词** 存在于任何文件中
2. **混淆或压缩的源代码** 无法被审查
3. **未声明域名的网络调用** 未列在 `api_calls` 中
4. **SKILL.md 中的提示注入模式**（试图覆盖代理安全性）
5. **用户数据外泄** —— 在未经用户明确同意的情况下将钱包地址、余额或交易历史发送到外部服务器
6. **绕过确认流程** —— 当 Plugin 声明 `standard` 风险等级时，在没有用户批准的情况下执行交易
7. **无限制的自主交易** —— `advanced` Plugin 缺少止损或最大金额保障
8. **冒充** —— 使用虚假暗示 OKX 或其他组织官方认可的名称、描述或品牌
9. **预编译的二进制文件** —— 提交源代码；CI 会编译
10. **许可证违规** —— 使用不兼容许可证的代码而未注明出处

---

## 11. 规则与限制

### 你可以做的

- 使用 SKILL.md 定义 skill
- 引用任何 onchainos CLI 命令进行链上操作
- 查询外部数据源（第三方 DeFi API、市场数据等）
- 包含参考文档
- 提交 binary 源代码（我们通过 `build` 部分编译）
- 在 `api_calls` 中声明外部 API 域名

### 你不可以做的

- 提交预编译的二进制文件（.exe、.dll、.so 等）—— 必须提交源代码
- 使用保留的 `okx-` 前缀（仅限 OKX 组织成员）
- 在 SKILL.md 中包含提示注入模式
- 超出文件大小限制（单文件 200KB，总计 5MB）

---

## 12. 常见问题

1. **审核需要多长时间？**
   自动化检查在 5 分钟内完成。人工审查通常需要 1-3 个工作日。

2. **Plugin 发布后可以更新吗？**
   可以。修改文件，在 `plugin.yaml` 和 `SKILL.md` 中升级 `version`，然后提交标题为 `新版本` 的新 PR。

3. **Plugin 的命名规则是什么？**
   仅允许小写字母、数字和连字符。2 到 40 个字符之间。不允许连续连字符和下划线。`okx-` 前缀仅保留给 OKX 组织成员。

4. **我可以使用任何编程语言吗？**
   Binary Plugin 支持 Rust、Go、TypeScript（Bun）、Node.js（Bun）和 Python。纯 Skill Plugin 可包含任何语言的脚本（Python 和 shell 常见）——它们作为 AI 代理工作流运行，不由 CI 编译。

5. **必须使用 Onchain OS 吗？**
   不是必须的。Onchain OS 是可选的，Plugin 可以自由使用任何链上技术。

6. **用户如何安装我的 Plugin？**
   发布后：`npx skills add okx/plugin-store --skill my-plugin`。用户端无需安装 plugin-store CLI。

7. **如果 AI 审查标记了某些内容怎么办？**
   AI 审查仅供参考，不会阻止 PR。但人工审查员会阅读 AI 报告。解决标记的问题可加速审批。

8. **本地 lint 通过但 CI 检查失败？**
   确保运行最新版 plugin-store CLI，并确认 PR 只修改了 `skills/your-plugin-name/` 内的文件。

9. **CI 构建失败但本地可编译？**
   CI 在 Ubuntu Linux 上编译。确保代码在 Linux 上可构建。检查 GitHub Actions 日志获取具体错误。

10. **错误 E122 "source_repo is not valid"？**
    `build.source_repo` 必须是 `owner/repo` 格式。不要包含 `https://github.com/` 或 `.git`。

11. **错误 E123 "must be a full 40-character hex SHA"？**
    `build.source_commit` 必须是完整 commit 哈希。运行 `git rev-parse HEAD` 获取。

12. **错误 E120 "must also include a Skill component"？**
    有 `build` 部分的 Plugin 必须同时有 SKILL.md。Skill 是入口——告诉 AI 代理如何使用你的 binary。

13. **错误 E130 "pre-compiled binary file is not allowed"？**
    提交中包含了编译文件（.exe、.dll、.so 等）。删除它——我们从源代码编译。

14. **错误 E110/E111 "requires a build section"？**
    声明了 Binary 组件但缺少 `build` 部分。添加 `build.lang`、`build.source_repo` 和 `build.source_commit`。

---
