# 开发者指南：构建和提交Plugin

> 为 Plugin Store 生态系统构建Plugin并提交审核。
> 阅读本指南后，你将拥有一个可以通过
> `npx skills add MigOKG/plugin-store --skill <name>` 安装的Plugin。

---

## 目录

1. [你可以构建什么？](#1-你可以构建什么)
2. [快速开始（7 步）](#2-快速开始7-步)
3. [Plugin结构](#3-Plugin结构)
4. [三种提交模式](#4-三种提交模式)
5. [OnchainOS 集成](#5-onchainos-集成)
6. [审核流程](#6-审核流程)
7. [风险等级](#7-风险等级)
8. [常见问题](#8-常见问题)

---

## 1. 你可以构建什么？

Plugin**不限于 Web3**。你可以构建分析仪表盘、开发者工具、交易策略、DeFi 集成、
安全扫描器、NFT 工具，或任何能受益于 AI 代理编排的应用。

### 两种Plugin类型

| 类型 | 包含内容 | 适用场景 |
|------|---------|---------|
| **纯 Skill** | 仅 `SKILL.md`（可选附带 scripts、assets、references） | 策略、工作流、数据查询，以及编排现有 CLI 的任何场景 |
| **Skill + 二进制** | `SKILL.md` 加上编译后的 CLI 工具（由 CI 编译源码） | 自定义计算、专有算法、复杂数据处理 |

即使Plugin包含二进制文件，**SKILL.md 始终是入口点**。Skill 告诉 AI 代理哪些工具
可用以及何时使用。

### 二进制Plugin支持的语言

| 语言 | 构建工具 | 分发方式 |
|------|---------|---------|
| Rust | `cargo build --release` | 原生二进制 |
| Go | `go build` | 原生二进制 |
| TypeScript | Bun | Bun 全局安装 |
| Node.js | Bun | Bun 全局安装 |
| Python | `pip install` | pip 包 |

### 优秀Plugin的特质

- **实用** -- 解决真实问题或自动化繁琐的工作流
- **安全** -- 不直接处理私钥，声明所有外部 API 调用，在适当的地方包含风险免责声明
- **文档完善** -- 清晰的 SKILL.md，包含具体示例、错误处理和预检查，以便 AI 代理可以从空白环境运行

---

## 2. 快速开始（7 步）

本教程创建一个最小的纯 Skill Plugin并提交。

### 步骤 1：Fork 并克隆

```bash
gh repo fork MigOKG/plugin-store --clone
cd plugin-store
```

### 步骤 2：创建Plugin目录

```bash
mkdir -p skills/my-plugin
```

### 步骤 3：创建 plugin.yaml

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
    dir: "."

api_calls: []
EOF
```

### 步骤 4：创建 .claude-plugin/plugin.json

```bash
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

> **重要**：`name`、`description` 和 `version` 字段必须与 `plugin.yaml` 完全一致。

### 步骤 5：创建 SKILL.md

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

### 步骤 6：本地验证

```bash
cd /path/to/plugin-store
cargo run --manifest-path cli/Cargo.toml -- lint skills/my-plugin
```

如果全部通过：

```
Linting skills/my-plugin...

  Plugin 'my-plugin' passed all checks!
```

### 步骤 7：提交 Pull Request

```bash
git checkout -b submit/my-plugin
git add skills/my-plugin
git commit -m "[new-plugin] my-plugin v1.0.0"
git push origin submit/my-plugin
```

然后从你的 fork 向 `MigOKG/plugin-store` 发起 Pull Request。使用以下标题：

```
[new-plugin] my-plugin v1.0.0
```

每个 PR 应该只包含**一个Plugin**，并且只修改 `skills/my-plugin/` 内的文件。

---

## 3. Plugin结构

### 目录布局

```
skills/my-plugin/
├── .claude-plugin/
│   └── plugin.json      # 必需 -- Claude Skill 注册元数据
├── plugin.yaml          # 必需 -- Plugin元数据和清单
├── SKILL.md             # 必需 -- AI 代理的 Skill 定义
├── scripts/             # 可选 -- Python 脚本、Shell 脚本
│   ├── bot.py
│   └── config.py
├── assets/              # 可选 -- HTML 仪表盘、图片
│   └── dashboard.html
├── references/          # 可选 -- AI 代理的额外文档
│   └── api-reference.md
├── README.md            # 可选 -- 面向开发者的文档
└── LICENSE              # 推荐 -- SPDX 兼容的许可证文件
```

`.claude-plugin/plugin.json`、`plugin.yaml` 和 `SKILL.md` 均为**必需文件**。其他均为可选。

### .claude-plugin/plugin.json

此文件遵循 [Claude Skill 架构](https://docs.anthropic.com/en/docs/claude-code)，是Plugin注册的必需文件。其内容必须与 `plugin.yaml` 保持一致。

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
| `version` | 是 | 必须与 `plugin.yaml` 中的 version 一致（语义化版本） |
| `author` | 是 | 姓名和可选的邮箱 |
| `license` | 是 | SPDX 标识符（MIT、Apache-2.0 等） |
| `keywords` | 否 | 可搜索的标签 |
| `homepage` | 否 | 项目主页 URL |
| `repository` | 否 | 源代码 URL |

### plugin.yaml 参考

#### 最小示例（纯 Skill，直接提交）

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
    dir: "."

api_calls: []
```

#### 外部仓库示例（模式 B）

当源代码位于你自己的 GitHub 仓库时，使用 `repo` 和 `commit` 代替 `dir`：

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

#### 二进制Plugin示例（Skill + 编译的 CLI）

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
    dir: "."

build:
  lang: rust
  source_repo: "defi-builder/yield-optimizer"
  source_commit: "a1b2c3d4e5f6789012345678901234567890abcd"
  source_dir: "."
  binary_name: defi-yield

api_calls:
  - "api.defillama.com"
```

#### 逐字段参考

| 字段 | 必需 | 说明 | 规则 |
|------|------|------|------|
| `schema_version` | 是 | Schema 版本 | 始终为 `1` |
| `name` | 是 | Plugin名称 | 小写 `[a-z0-9-]`，2-40 字符，不可连续连字符 |
| `version` | 是 | Plugin版本 | 语义化版本 `x.y.z`（带引号的字符串） |
| `description` | 是 | 一行摘要 | 不超过 200 字符 |
| `author.name` | 是 | 作者显示名 | 你的姓名或组织名 |
| `author.github` | 是 | GitHub 用户名 | 必须与 PR 作者一致 |
| `author.email` | 否 | 联系邮箱 | 用于安全通知 |
| `license` | 是 | 许可证标识 | SPDX 格式：`MIT`、`Apache-2.0`、`GPL-3.0` 等 |
| `category` | 是 | Plugin分类 | 以下之一：`trading-strategy`、`defi-protocol`、`analytics`、`utility`、`security`、`wallet`、`nft` |
| `tags` | 否 | 搜索关键词 | 字符串数组 |
| `type` | 否 | 作者类型 | `"official"`、`"dapp-official"`、`"community-developer"` |
| `link` | 否 | 项目主页 | URL，在市场中展示 |
| `components.skill.dir` | 模式 A | Skill 目录路径 | Plugin目录内的相对路径（使用 `"."` 表示Plugin根目录） |
| `components.skill.repo` | 模式 B | 外部仓库 | 格式：`owner/repo` |
| `components.skill.commit` | 模式 B | 固定 commit | 完整 40 字符十六进制 SHA |
| `build.lang` | 仅二进制 | 源语言 | `rust` / `go` / `typescript` / `node` / `python` |
| `build.source_repo` | 仅二进制 | 源代码仓库 | 格式：`owner/repo` |
| `build.source_commit` | 仅二进制 | 固定 commit SHA | 完整 40 字符十六进制；通过 `git rev-parse HEAD` 获取 |
| `build.source_dir` | 否 | 源码子目录 | 仓库内路径，默认 `.` |
| `build.binary_name` | 仅二进制 | 输出二进制名 | 必须与编译器产出的文件名一致 |
| `build.main` | TS/Node/Python | 入口文件 | 例如 `src/index.js` 或 `src/main.py` |
| `api_calls` | 否 | 外部 API 域名 | Plugin调用的域名字符串数组 |

#### 命名规则

- **允许**：`solana-price-checker`、`defi-yield-optimizer`、`nft-tracker`
- **禁止**：`OKX-Plugin`（大写）、`my_plugin`（下划线）、`a`（太短）
- **保留前缀**：`okx-`、`official-`、`plugin-store-` -- 仅 OKX 组织成员可使用 `okx-`

### SKILL.md 参考

SKILL.md 是你Plugin的**唯一入口点**。它告诉 AI 代理你的Plugin做什么以及如何使用。

#### 完整模板

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

#### 二进制Plugin的 SKILL.md

当你的Plugin包含二进制工具时，SKILL.md 必须同时记录**二进制工具和 CLI 命令**：

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

#### SKILL.md 前置元数据字段

| 字段 | 必需 | 说明 |
|------|------|------|
| `name` | 是 | 必须与 plugin.yaml 中的 `name` 一致 |
| `description` | 是 | 简要描述（应与 plugin.yaml 一致） |
| `version` | 是 | 必须与 plugin.yaml 中的 `version` 一致 |
| `author` | 是 | 作者名称 |
| `tags` | 否 | 可发现性关键词 |

#### SKILL.md 必需章节

- **Overview** -- Skill 的功能
- **Pre-flight Checks** -- 前置条件、依赖安装（必须能从空白环境运行）
- **Commands** -- 每个命令的使用时机、输出描述和具体示例
- **Error Handling** -- 错误、原因和解决方案表格
- **Security Notices** -- 风险等级、免责声明

#### SKILL.md 最佳实践

1. **具体明确** -- `onchainos token search --query SOL --chain solana` 优于 "搜索代币"
2. **始终包含错误处理** -- AI 代理需要知道失败时该怎么做
3. **使用 Skill 路由** -- 告诉 AI 何时应转交给其他 Skill
4. **包含预检查** -- 依赖安装命令，让 AI 代理能从零开始设置环境
5. **不要重复 onchainos 功能** -- 编排现有命令，而非替代它们

---

## 4. 三种提交模式

### 模式 A -- 直接提交（推荐）

所有文件放在 plugin-store 仓库的 `skills/<name>/` 下。这是最简单的方式，
推荐大多数Plugin使用。

```
skills/my-plugin/
├── .claude-plugin/
│   └── plugin.json   # 必需
├── plugin.yaml       # 必需
├── SKILL.md          # 必需
├── scripts/          # 可选
├── assets/           # 可选
├── LICENSE
└── README.md
```

plugin.yaml 使用 `components.skill.dir`：

```yaml
components:
  skill:
    dir: "."
```

**适用场景**：你愿意将所有源码直接放在 plugin-store 仓库中。适合纯 Skill Plugin
和包含少量脚本的Plugin。

### 模式 B -- 外部仓库

你的 plugin.yaml 指向你自己的 GitHub 仓库和固定的 commit SHA。只有 `plugin.yaml`
（以及可选的 `LICENSE`、`README.md`）放在 plugin-store 仓库中。

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

commit 必须是**完整的 40 字符 SHA**（不是短 SHA 或分支名）。获取方式：

```bash
cd your-source-repo
git push origin main
git rev-parse HEAD
# Output: d2aa628e063d780c370b0ec075a43df4859be951
```

**适用场景**：你的Plugin有大量源码，你希望保留在自己的仓库中，或者需要独立的版本管理。
`meme-trench-scanner` 和 `smart-money-signal-copy-trade` 等Plugin使用此方式。

### 模式 C -- 市场导入

如果你已经有一个兼容 Claude 市场的仓库，可以自动生成提交：

```bash
plugin-store import your-username/my-plugin
```

这会自动读取你的仓库结构、检测构建语言、生成 `plugin.yaml`、fork plugin-store
仓库、创建分支并打开 PR。

**前置条件**：已安装并认证 `gh` CLI（`gh auth login`）。

**适用场景**：你已有一个可用的 Claude 市场Plugin，想以最小成本在 Plugin Store 中上架。

---

## 5. OnchainOS 集成

### 什么是 OnchainOS？

[OnchainOS](https://github.com/okx/onchainos-skills) 是 Agentic Wallet CLI，
提供安全的沙箱化区块链操作 -- 钱包签名、交易广播、Swap 执行、合约调用等。
它使用 TEE（可信执行环境）签名，私钥永远不会离开安全飞地。

### 何时使用 OnchainOS

当你的Plugin执行任何链上写操作时，应使用 OnchainOS：

- 钱包签名
- 交易广播
- Swap 执行
- 合约调用
- 代币授权

### OnchainOS 是否必需？

**OnchainOS 推荐使用但非必需。** Plugin不限于 Web3。

但是：

- 使用 OnchainOS 进行链上操作的Plugin会获得**更高的信任分数**和**更好的市场可见度**
- 不使用 OnchainOS 的链上Plugin需要**额外的安全审核**，因为它们在沙箱环境外处理区块链操作
- 使用第三方钱包（MetaMask、Phantom）或直接 RPC 调用（ethers.js、web3.js）进行
  链上写操作的Plugin将面临更严格的审核，如果无法证明等效安全性可能会被拒绝

对于非区块链Plugin（分析、工具、开发者工具等），OnchainOS 不适用。

### OnchainOS 命令参考

| 命令 | 说明 | 示例 |
|------|------|------|
| `onchainos token` | 代币搜索、信息、趋势、持有者 | `onchainos token search --query SOL --chain solana` |
| `onchainos market` | 价格、K 线图、组合 PnL | `onchainos market price --address 0x... --chain ethereum` |
| `onchainos swap` | DEX swap 报价和执行 | `onchainos swap quote --from ETH --to USDC --amount 1` |
| `onchainos gateway` | Gas 估算、交易模拟、广播 | `onchainos gateway gas --chain ethereum` |
| `onchainos portfolio` | 钱包总价值和余额 | `onchainos portfolio all-balances --address 0x...` |
| `onchainos wallet` | 登录、余额、发送、历史 | `onchainos wallet balance --chain solana` |
| `onchainos security` | 代币扫描、DApp 扫描、交易扫描 | `onchainos security token-scan --address 0x...` |
| `onchainos signal` | 聪明钱 / 鲸鱼信号 | `onchainos signal list --chain solana` |
| `onchainos memepump` | Meme 代币扫描和分析 | `onchainos memepump tokens --chain solana` |
| `onchainos leaderboard` | 按 PnL/交易量排名的顶级交易者 | `onchainos leaderboard list --chain solana` |
| `onchainos payment` | x402 支付协议 | `onchainos payment x402-pay --url ...` |

完整的子命令列表请运行 `onchainos <command> --help` 或查看
[onchainos 文档](https://github.com/okx/onchainos-skills)。

### 安装 OnchainOS

```bash
curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | bash
```

如果安装后找不到 `onchainos`，将其添加到 PATH：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

---

## 6. 审核流程

每个 Pull Request 都会经过 4 阶段的 CI 流水线。

### 阶段 1：静态 Lint（自动，即时）

验证Plugin结构、命名约定、版本格式、必需文件和安全默认值。结果会以 PR 评论形式发布。

如果 lint 失败，PR 会被阻止。修复问题后重新推送。

### 阶段 2：构建验证（自动，仅二进制Plugin）

如果你的Plugin有 `build` 部分，CI 会在固定的 commit SHA 处克隆你的源码仓库、
编译代码并验证二进制文件可运行。构建失败会阻止 PR。

### 阶段 3：AI 代码审查（自动，约 2 分钟）

AI 审查器读取你的Plugin并生成涵盖安全性、合规性和质量的 8 部分报告。报告以可折叠
PR 评论的形式发布。此阶段**仅供参考** -- 不阻止合并，但人工审查者会阅读报告。

### 阶段 4：汇总和预检（自动）

生成所有前序阶段的汇总。预检步骤会自动注入以下内容到测试环境中：

- **onchainos CLI** -- Agentic Wallet CLI
- **Skills** -- 你的Plugin Skill 文件
- **plugin-store Skill** -- plugin-store Skill 本身
- **HMAC 安装报告** -- 签名的报告，确认安装完整性

这确保每个Plugin都能在真实环境中进行端到端验证。

### 人工审核（1-3 个工作日）

所有自动阶段通过后，维护者会审核Plugin的正确性、安全性和质量。他们会检查Plugin
是否合理、API 调用是否准确声明、SKILL.md 是否撰写良好，以及是否存在安全问题。

### 十大拒绝原因

| # | 原因 | 如何避免 |
|---|------|---------|
| 1 | 缺少 `plugin.yaml`、`.claude-plugin/plugin.json` 或 `SKILL.md` | 每个Plugin都必须包含这三个文件 |
| 2 | `plugin.yaml` 和 `SKILL.md` 之间版本不一致 | 保持两个文件中的 `version` 完全相同 |
| 3 | 硬编码 API 密钥或凭据 | 使用环境变量，切勿提交密钥 |
| 4 | 交易Plugin缺少风险免责声明 | 在 SKILL.md 中为任何涉及资产操作的Plugin添加免责声明 |
| 5 | 不通过 OnchainOS 进行直接钱包操作 | 使用 `onchainos wallet` / `onchainos swap` 进行链上写操作 |
| 6 | 缺少 LICENSE 文件 | 添加包含 SPDX 兼容许可证的 LICENSE 文件 |
| 7 | 未固定依赖版本 | 固定所有依赖版本；使用 lockfile |
| 8 | 分类不匹配 | 选择最准确描述你Plugin的分类 |
| 9 | SKILL.md 缺少必需章节 | 包含 Overview、Pre-flight、Commands、Error Handling、Security Notices |
| 10 | 自动交易无模拟模式 | 所有自动交易Plugin必须支持 dry-run / 模拟交易模式 |

### 常见 Lint 错误

| 代码 | 含义 | 修复方式 |
|------|------|---------|
| E001 | 未找到 plugin.yaml | 确保 plugin.yaml 在Plugin目录根路径 |
| E031 | 名称格式无效 | 仅允许小写字母、数字和连字符 |
| E033 | 保留前缀 | 名称不要以 `okx-`、`official-` 或 `plugin-store-` 开头 |
| E035 | 版本无效 | 使用语义化版本：`1.0.0`，而非 `1.0` 或 `v1.0.0` |
| E041 | 缺少 LICENSE | 添加 LICENSE 文件 |
| E052 | 缺少 SKILL.md | 确保 SKILL.md 存在于 `components.skill.dir` 指定的目录中 |
| E065 | 缺少 api_calls | 在 plugin.yaml 中添加 `api_calls` 字段（如果没有则使用 `[]`） |
| E110 | 声明了二进制但缺少 build 部分 | 添加 `build.lang`、`build.source_repo`、`build.source_commit` |
| E122 | source_repo 格式无效 | 使用 `owner/repo` 格式，而非完整 URL |
| E123 | source_commit 无效 | 必须是通过 `git rev-parse HEAD` 获取的完整 40 字符十六进制 SHA |
| E130 | 提交了预编译的二进制文件 | 删除二进制文件；提交源码，由 CI 编译 |

### 提交前检查清单

将以下内容复制到你的 PR 描述中：

```markdown
- [ ] `plugin.yaml`、`.claude-plugin/plugin.json` 和 `SKILL.md` 均已提供
- [ ] `name` 字段为小写加连字符，2-40 字符
- [ ] `version` 在 `plugin.yaml`、`plugin.json` 和 `SKILL.md` 中一致
- [ ] `author.github` 与我的 GitHub 用户名一致
- [ ] `license` 字段使用有效的 SPDX 标识符
- [ ] `category` 为允许的值之一
- [ ] `api_calls` 列出了所有外部 API 域名（如果没有则为 `[]`）
- [ ] SKILL.md 包含 name、description、version、author 的 YAML 前置元数据
- [ ] SKILL.md 包含 Overview、Pre-flight、Commands、Error Handling 章节
- [ ] 没有硬编码的 API 密钥、令牌或凭据
- [ ] 没有预编译的二进制文件
- [ ] LICENSE 文件已提供
- [ ] PR 标题格式为：`[new-plugin] my-plugin v1.0.0`
- [ ] PR 分支名格式为：`submit/my-plugin`
- [ ] PR 仅修改 `skills/my-plugin/` 内的文件
- [ ] （交易Plugin）已包含风险免责声明
- [ ] （交易Plugin）支持 dry-run / 模拟交易模式
- [ ] （二进制Plugin）源码可用等效 CI 命令在本地编译
- [ ] 本地 lint 通过：`cargo run --manifest-path cli/Cargo.toml -- lint skills/my-plugin`
```

---

## 7. 风险等级

每个Plugin根据其功能被分配三个风险等级之一。

| 等级 | 名称 | 定义 | 额外要求 |
|------|------|------|---------|
| `starter` | 入门级 | 只读操作，无资产变动 | 标准审核 |
| `standard` | 标准级 | 每次交易需用户明确确认 | 标准审核 + 确认流程检查 |
| `advanced` | 高级 | 自动化策略，可自主运行 | 见下文 |

### 高级风险等级要求

`advanced` 风险等级的Plugin必须包含以下全部内容：

1. **Dry-run / 模拟交易模式** -- 必须为默认模式或有清晰的文档说明
2. **止损机制** -- 可配置的最大损失阈值
3. **最大金额限制** -- 可配置的单笔和单次会话上限
4. **风险免责声明** -- SKILL.md 中醒目的免责声明（参见 `meme-trench-scanner` Plugin）
5. **两位审核者** -- 高级Plugin需要两位维护者批准

### 绝对红线

以下情况无论风险等级如何都将被立即拒绝：

1. **硬编码的私钥或助记词** 出现在任何文件中
2. **混淆或压缩的源码** 无法审查
3. **未声明域名的网络调用** 未在 `api_calls` 中列出
4. **SKILL.md 中的提示注入模式** （试图绕过代理安全机制）
5. **用户数据外泄** -- 未经用户明确同意就将钱包地址、余额或交易记录发送到外部服务器
6. **绕过确认流程** -- 在Plugin声明为 `standard` 风险等级时未经用户批准就执行交易
7. **无限制的自动交易** -- `advanced` Plugin缺少止损或最大金额保障
8. **冒充** -- 使用暗示 OKX 或其他组织官方背书的名称、描述或品牌
9. **预编译二进制** -- 提交源码；CI 负责编译
10. **许可证违规** -- 使用不兼容许可证的代码且未注明归属

---

## 8. 常见问题

**审核需要多长时间？**

自动检查在 5 分钟内完成。人工审核通常需要 1-3 个工作日。

**Plugin发布后可以更新吗？**

可以。修改文件，在 `plugin.yaml` 和 `SKILL.md` 中升级 `version`，然后以
`[update] my-plugin v1.1.0` 为标题提交新的 PR。

**Plugin命名规则是什么？**

仅允许小写字母、数字和连字符。长度 2 到 40 个字符。不允许连续连字符。
不允许下划线。`okx-` 前缀仅限 OKX 组织成员使用。

**可以使用任何编程语言吗？**

二进制Plugin支持 Rust、Go、TypeScript (Bun)、Node.js (Bun) 和 Python。
纯 Skill Plugin可以包含任何语言的脚本（Python 和 Shell 脚本最常见）-- 它们
作为 AI 代理工作流的一部分运行，不由 CI 编译。

**必须使用 OnchainOS 吗？**

不是。OnchainOS 推荐用于区块链操作但非必需。非区块链Plugin完全不需要。
不使用 OnchainOS 的区块链Plugin将经过额外的安全审核。

**用户如何安装我的Plugin？**

你的 PR 合并后，用户通过以下命令安装：

```bash
npx skills add MigOKG/plugin-store --skill my-plugin
```

用户端无需安装 plugin-store CLI。

**AI 审查标记了某些问题怎么办？**

AI 审查仅供参考，不会阻止 PR。但人工审查者会阅读 AI 报告。解决标记的问题可以
加速审批。

**本地 lint 通过但 GitHub 检查失败，为什么？**

确保你运行的是最新版本的 plugin-store CLI。同时确认你的 PR 仅修改了
`skills/your-plugin-name/` 内的文件。

**CI 构建失败但本地编译成功，为什么？**

CI 在 Ubuntu Linux 上编译。确保你的代码可以在 Linux 上构建，而不仅仅是 macOS
或 Windows。查看 GitHub Actions 运行日志以获取具体错误信息。

**在哪里可以获得帮助？**

- 在 GitHub 上提交 [issue](https://github.com/MigOKG/plugin-store/issues)
- 查看 `skills/` 中的现有Plugin作为示例
- 提交前在本地运行 lint 命令 -- 它能捕获大多数问题
- 如果 PR 检查失败，查看 [GitHub Actions 日志](https://github.com/MigOKG/plugin-store/actions)
