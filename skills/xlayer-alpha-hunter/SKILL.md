---
name: xlayer-alpha-hunter
description: "X Layer Alpha Hunter — 聪明钱监控 + 自动交易 + 每日复盘全集。基于SM行为分析优化：只跟SM:4+高置信度信号 + 币种质量过滤。"
version: "4.0.0"
author: hermes-agent
tags:
  - x-layer
  - smart-money
  - auto-trade
  - alpha
  - token-quality-filter
  - onchainos
  - daily-report
---

# X Layer Alpha Hunter

## Overview

X Layer Alpha Hunter 是一款基于 OKX Onchain OS 的智能交易策略插件，通过监控 X Layer 链上聪明钱（Smart Money）活动，捕捉机构级别的 alpha 信号，辅助用户实现自动化跟单交易。

核心功能：
- **SM 信号监控**：追踪 OKX 预分类的聪明钱钱包（walletType=1），实时捕捉其买入/卖出行为
- **Alpha 信号识别**：通过 convergence_score（收敛分数）和 alpha_score（alpha 分数）双重过滤，只跟高置信度信号
- **新币 Alpha 发现**：监控尚未被 SM 关注的低市值代币，提前埋伏
- **自动交易执行**：支持 dry-run 模式和安全自动卖出
- **每日复盘报告**：自动生成交易日志和策略分析报告

本插件为 **dry-run 优先**设计，首次使用默认不执行真实交易。

---

## Pre-flight Checks

使用本插件前，确保：

1. **onchainos CLI 已安装并认证**
   ```bash
   npx skills add okx/onchainos-skills
   onchainos wallet status  # 确认已登录
   ```

2. **X Layer 钱包余额充足**
   ```bash
   onchainos wallet balance --chain xlayer
   ```

3. **Telegram Bot 已配置**（可选，用于实时推送）
   - 确保 Hermes Agent 已配置 Telegram Bot

---

## Core Concepts

### 聪明钱信号（Smart Money Signals）

OKX 预分类了 X Layer 上的聪明钱钱包（KOL/Whale/SM 三类），通过 `onchainos signal list` 获取实时信号：

| 信号类型 | soldRatio | 含义 | 操作 |
|---------|-----------|------|------|
| **BUY** | < 20 | SM 在吸筹 | 关注/买入 |
| **NEUTRAL** | 20-80 | 分歧 | 观察 |
| **SELL** | > 80 | SM 在出货 | 谨慎/卖出 |

### Convergence Score（收敛分数）

多钱包同向信号 = convergence。收敛分数计算：

```
buy_strength = max(0, (20 - soldRatio) / 20)   # soldRatio=0 → 1.0
sell_strength = max(0, (soldRatio - 80) / 20)  # soldRatio=100 → 1.0
convergence_score = trigger_wallet_count × direction_weight
```

阈值：
- 0 = 无信号
- 0.1-2.9 = 弱信号（记录）
- 3.0-5.9 = 中信号（Telegram 告警）
- 6.0+ = 强信号（执行操作）

### 买入条件（should_buy）

```
SM BUY 信号数量 >= 4
AND convergence_score >= 5.0
AND alpha_score >= 6.0
AND riskLevel = 1（低风险）
AND top10 持仓 < 30%
AND 无 "高流量陷阱" 标签
AND 非新增闭锁期代币
```

### 卖出条件（should_sell）

```
条件A：soldRatio == 100%（SM 完全出货）
OR 条件B：持仓盈利 >= 50%
OR 条件C：持仓亏损 <= -15%（止损）
```

---

## Commands

### 1. 获取 SM 信号

```bash
# 获取 X Layer 最新聪明钱信号（按时间排序）
onchainos signal list --chain xlayer --limit 100

# 获取特定代币的 SM 活动详情
onchainos tracker activities --tracker-type smart_money --trade-type 1 --chain xlayer
```

**输出示例**：
```
代币: XDOG
SM 钱包数量: 4
soldRatio: 45.78%
方向: SELL
信号时间: 2026-05-13 22:03
```

### 2. 查询代币安全信息

```bash
# 代币基本信息
onchainos token hot-tokens --chain xlayer --limit 50

# 代币高级信息（风险评级、top10 持仓等）
onchainos token advanced-info --chain xlayer --token-address <地址>

# 安全扫描
onchainos security token-scan --chain xlayer --token-address <地址>
```

### 3. 执行交易（Dry-run 优先）

```bash
# Swap 执行（先加 --dry-run 测试）
onchainos swap execute \
  --chain xlayer \
  --from-token USDT \
  --to-token <代币地址> \
  --amount 10 \
  --slippage 1

# 检查钱包余额
onchainos wallet balance --chain xlayer
```

### 4. 运行完整策略脚本

```bash
# 监控模式（仅查看信号，不交易）
python3 /root/scripts/run_xlayer_trading.py --mode monitor

# 自动交易模式（需要 tradable=true）
python3 /root/scripts/run_xlayer_trading.py --mode auto

# 每日复盘（分析昨日交易）
python3 /root/scripts/run_xlayer_trading.py --review

# 回测验证
python3 /root/scripts/backtest_xlayer.py
```

---

## Configuration

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `TRADABLE` | `false` | `true` 启用真实交易，`false` 仅监控 |
| `CONVERGENCE_THRESHOLD` | `5.0` | 收敛分数阈值 |
| `ALPHA_THRESHOLD` | `6.0` | Alpha 分数阈值 |
| `MIN_SM_COUNT` | `4` | 最少 SM 钱包数量 |
| `STOP_LOSS_PCT` | `-15` | 止损百分比 |
| `TAKE_PROFIT_PCT` | `50` | 止盈百分比 |

### 交易限制

| 参数 | 默认值 | 最大值 |
|------|--------|--------|
| 单笔交易上限 | $10 | $100 |
| 单次会话上限 | $50 | $500 |
| 最大持仓时间 | 24h | 72h |

---

## Error Handling

| 错误 | 原因 | 解决方案 |
|------|------|----------|
| `API error (code=81001)` | 参数错误 | 检查 chain-index=196, 钱包地址格式 |
| `Insufficient balance` | 余额不足 | 充值 X Layer 原生代币 |
| `Swap failed: slippage` | 滑点超限 | 增大 slippage 或等待更好的价格 |
| `SM signal lost` | 信号窗口关闭 | SM 通常有 3-5 分钟操作窗口 |
| `Token risk level > 1` | 代币风险高 | 自动过滤，不建议交易 |

---

## Security Notices

### 风险免责声明

**⚠️ 重要风险提示**

1. **市场风险**：加密货币市场波动剧烈，本插件仅供参考，不构成投资建议
2. **智能合约风险**：即使通过安全扫描的代币也可能存在合约漏洞
3. **SM 跟踪风险**：SM 信号有延迟，可能存在踩踏风险
4. **流动性风险**：低市值代币可能存在流动性不足问题

### 安全操作规范

- **Dry-run 优先**：首次使用务必先加 `--dry-run` 测试
- **小额试探**：单笔交易不超过持仓的 10%
- **止损纪律**：亏损达到 -15% 必须止损，不要扛单
- **分散风险**：单次持仓不超过 3 个代币
- **定期复盘**：每日运行 `--review` 检查策略效果

### 数据来源声明

- SM 数据来源：OKX Onchain OS API
- 代币信息：OKX Token API + 安全扫描服务
- 交易执行：通过 OKX Agentic Wallet TEE 安全执行

---

## Daily Workflow

### 每日运行流程

```
1. 早间检查（9:00 AM）
   python3 /root/scripts/run_xlayer_trading.py --review
   → 查看昨日交易报告，调整参数

2. 全天监控（每 15 分钟）
   python3 /root/scripts/run_xlayer_trading.py --mode monitor
   → 监控 SM 信号，等待机会

3. 信号触发时
   python3 /root/scripts/run_xlayer_trading.py --mode auto
   → 自动执行买入/卖出

4. 收盘复盘（11:59 PM）
   → 生成当日交易汇总
```

### 报告输出位置

- 每日报告：`~/.hermes/cron/output/daily_report_YYYY-MM-DD.txt`
- 交易日志：`~/.hermes/cron/output/trades.db`（SQLite）
- 信号记录：`~/.hermes/cron/output/xlayer_alpha_YYYYMMDD_*.json`
