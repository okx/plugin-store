# Smart Money Signal Copy Trade - 聪明钱信号跟单

Smart money signal tracker — polls Smart Money / KOL / Whale buy signals every 20 seconds, executes copy trades after 15-check safety verification. Cost-aware take profit ensures real profitability after fees. All on-chain operations powered by [onchainos](https://github.com/okx/onchainos-skills) Agentic Wallet (TEE signing, no API key needed).

聪明钱信号跟单策略 — 每 20 秒轮询 Smart Money / KOL / 鲸鱼买入信号，通过 15 项安全验证后执行跟单。成本感知止盈确保扣除手续费后真正盈利。所有链上操作由 [onchainos](https://github.com/okx/onchainos-skills) Agentic Wallet 驱动（TEE 签名，无需 API Key）。

## Features / 功能

- **Smart Money Signals / 聪明钱信号** — Tracks SmartMoney, KOL, and Whale wallet activity in real-time
- **Co-Rider Consensus / 共乘共识** — Triggers when 3+ smart wallets buy the same token simultaneously
- **15-Check Deep Safety / 15 项深度安全验证** — MC, liquidity, holders, Dev rug, Bundler, LP burn, K1 pump detection
- **Cost-Aware TP / 成本感知止盈** — TP thresholds include breakeven calculation (fees + slippage)
- **7-Layer Exit System / 7 层退出系统** — Liquidity emergency, hard stop, time-decay SL, tiered TP, trailing stop, trend stop
- **Session Risk Control / 会话风控** — Consecutive loss pause, cumulative loss stop
- **Hot-Reload Config / 热重载配置** — Modify config.py without restarting the bot
- **Web Dashboard / 实时仪表盘** — http://localhost:3248

## Install / 安装

```bash
npx skills add okx/plugin-store-community --skill smart-money-signal-copy-trade
```

## Risk Warning / 风险提示

> Smart money signals do not guarantee profits. Signal delays and manipulation can happen at any time. Always test in Paper Mode first.

> 聪明钱信号不保证盈利，信号延迟和操纵随时可能发生。请先在纸盘模式下测试。

## License

MIT
