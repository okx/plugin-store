# Starter Coach — Your On-Chain Trading Bot Builder

A conversational 6-step skill that guides any user — beginner or experienced — to design, backtest, paper trade, and deploy their own automated DEX spot-trading bot on OKX DEX.

你的链上交易机器人构建教练 — 从零到上线，一次对话搞定。

## How It Works / 使用流程

```
Step 1: Onboard        — Welcome + legal disclaimer
Step 2: Profile        — Risk appetite, budget, chain preference
Step 3: Build Strategy — Choose goal, configure entry/exit rules
Step 4: Backtest       — Validate strategy on historical data
Step 5: Paper Trade    — Simulate live trades with zero risk
Step 6: Go Live        — Explicit CONFIRM gate + live execution
```

## Features / 功能

- **7 Goal Archetypes / 7 种策略目标** — Stack Sats, Buy the Dip, Follow Smart Money, Copy-trade Wallet, Snipe Tokens, Ride the Trend, Something Else
- **Bilingual / 双语支持** — English and Chinese, auto-detected
- **Backtest Engine / 回测引擎** — Historical validation with performance disclaimer
- **Paper Trade Gate / 纸盘门槛** — Must pass paper trading before live mode unlocks
- **Live Mode Safeguard / 上线确认机制** — Requires explicit "CONFIRM", logged to audit file
- **Kelly Sizing / Kelly 仓位管理** — Configurable risk-adjusted position sizing
- **TEE Signing / TEE 签名** — All trades via OnchainOS Agentic Wallet, no API key needed

## Install / 安装

```bash
plugin-store install starter-coach
```

Or:

```bash
npx skills add okx/plugin-store --skill starter-coach
```

## Usage / 使用方法

Once installed, just tell your AI agent:

```
Start starter coach
```

```
帮我创建一个交易机器人
```

The coach will guide you through the full flow interactively.

## Risk Warning / 风险提示

> DEX trading involves significant financial risk. Past backtest performance does not guarantee future results. Always complete paper trading before going live. Never invest more than you can afford to lose.

> DEX 交易存在重大财务风险。历史回测结果不代表未来收益。请务必先完成纸盘交易再上线。切勿投入超出承受能力的资金。

## License

MIT
