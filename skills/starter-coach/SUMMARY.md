## Overview

Starter Coach is a conversational 6-step skill that guides users to design, backtest, paper trade, and deploy their own automated DEX spot-trading bot on OKX DEX.

Core operations:

- Guide users through a 6-step flow: Onboard → Profile → Build Strategy → Backtest → Paper Trade → Go Live
- Generate a validated JSON strategy spec from user inputs (goal, risk, budget, chain)
- Run backtests on historical data with performance metrics and disclaimer
- Gate live mode behind paper trading requirement and explicit CONFIRM acknowledgment
- Execute DEX spot trades via onchainos Agentic Wallet (TEE signing)

Tags: `trading-bot` `strategy-builder` `dex` `onchainos` `solana` `ethereum` `paper-trade`

## Prerequisites

- No IP/region restrictions
- Supported chains: Solana, Ethereum, and all OKX DEX-supported chains
- Supported tokens: All tokens available on OKX DEX
- onchainos CLI installed and authenticated (`onchainos --version` and `onchainos wallet status`)
- Python 3.8+ (standard library only — no `pip install` required)
- Funded wallet on your chosen chain for live trading

## Quick Start

1. **Install the skill**: `plugin-store install starter-coach`
2. **Start the coach**: Tell your agent "Start starter coach" or "帮我创建一个交易机器人"
3. **Complete the 6-step flow**: Answer questions about your goal, risk appetite, budget, and chain preference — the coach builds your strategy spec interactively
4. **Review backtest results**: The coach runs a backtest and shows win rate, P&L, and trade count — past performance does not guarantee future results
5. **Paper trade first**: Simulate live trades with zero risk to validate your strategy before going live
6. **Go live**: Type `CONFIRM` when prompted — your acknowledgment is logged and live execution begins via onchainos Agentic Wallet
