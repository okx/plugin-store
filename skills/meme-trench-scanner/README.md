# Meme Trench Scanner - Meme 扫链

Solana Meme automated trading bot — scans 11 Launchpads, detects signals, executes trades, manages exits. All on-chain operations powered by [onchainos](https://github.com/okx/onchainos-skills) Agentic Wallet (TEE signing, no API key needed).

Solana Meme 自动交易机器人 — 覆盖 11 个 Launchpad，检测信号，执行交易，管理退出。所有链上操作由 [onchainos](https://github.com/okx/onchainos-skills) Agentic Wallet 驱动（TEE 签名，无需 API Key）。

## Features / 功能

- **11 Launchpad Coverage / 覆盖 11 个 Launchpad** — pump.fun, Believe, LetsBonk, and more
- **Triple Signal Detection / 三重信号检测** — TX acceleration + Volume surge + B/S ratio
- **5m/15m Precision / 5 分钟/15 分钟精度** — Raw trades calculation for buy/sell ratio
- **Deep Safety Checks / 深度安全检测** — Dev rug history, Bundler holdings, LP Lock, Aped wallets
- **7-Layer Exit System / 7 层退出系统** — Emergency exit, FAST_DUMP crash detection, stop loss, trailing stop, tiered TP
- **TOP_ZONE Filter / 价格位置过滤** — Skips tokens near ATH (>85%) to avoid chasing
- **TraderSoul AI / AI 观察系统** — Records trading behavior and personality tags
- **Web Dashboard / 实时仪表盘** — http://localhost:3241

## Install / 安装

```bash
npx skills add okx/plugin-store-community --skill meme-trench-scanner
```

## Risk Warning / 风险提示

> Meme tokens are the highest-risk asset class. Tokens can go to zero within minutes. Always test in Paper Mode first.

> Meme 代币是最高风险资产类别，可能在几分钟内归零。请先在纸盘模式下测试。

## License

MIT
