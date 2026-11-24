# Wallet Tracker (Mcap) -- Wallet Copy-Trade Bot

Monitor target Solana wallets for meme token trades and auto-mirror buys/sells with comprehensive safety checks. Two follow modes: MC_TARGET (wait for market cap proof, safer) or INSTANT (follow immediately). Tiered take-profit, trailing stop, mirror sell, time stop, and 4-tier risk grading. All on-chain operations powered by onchainos Agentic Wallet (TEE signing, no private keys needed).

钱包跟单策略 -- 监控目标钱包持仓变化，自动跟买跟卖。支持 MC 目标模式（更安全）和即时跟单。梯度止盈、追踪止损、镜像卖出、时间止损、四级风控评级。onchainos Agentic Wallet TEE 签名，无需私钥。

## Features

- **Two Follow Modes** -- MC_TARGET (wait for market cap proof) or INSTANT (immediate)
- **5 Exit Triggers** -- Mirror sell, stop loss, tiered take-profit, trailing stop, time stop
- **4-Tier Risk Grading** -- Honeypot, rug history, wash trading, liquidity drain detection
- **Safety Gates** -- Liquidity, holders, top10, dev hold, bundle checks before every trade
- **Post-Trade Monitoring** -- Active dump, LP drain, coordinated selling → auto exit
- **TEE Signing** -- onchainos Agentic Wallet, private keys never leave secure enclave
- **Paper Mode** -- MODE="paper" + PAUSED=True by default, safe to test
- **Web Dashboard** -- Positions, watch list, trades, live feed at http://localhost:3248
- **Zero Dependencies** -- Python 3.8+ stdlib only + onchainos CLI

## Install

```bash
npx skills add okx/plugin-store --skill wallet-tracker-mcap
```

## Prerequisites

```bash
# 1. onchainos CLI >= 2.1.0
onchainos --version

# 2. Login to Agentic Wallet
onchainos wallet login <your-email>

# 3. No pip install needed -- stdlib only
```

## Risk Warning

> Wallet copy-trading involves real financial risk. Target wallets may trade tokens that fail, lose all liquidity, or face regulatory scrutiny. Always test in Paper Mode (MODE="paper") first. This tool is for educational and research purposes only -- not investment advice.

> 钱包跟单涉及真实财务风险。目标钱包可能交易失败、流动性归零或面临监管审查的代币。请始终先在模拟模式（MODE="paper"）下测试。本工具仅供教育和研究用途，不构成投资建议。

## License

MIT
