# Top Rank Tokens Sniper - 榜单狙击手

OKX ranking leaderboard sniper — scans Solana 1h gainers Top 20 every 10 seconds, filters new entries through 3-level safety + Momentum scoring, then automatically snipes entries. Ranking Exit ensures positions are closed when momentum fades. All on-chain operations powered by [onchainos](https://github.com/okx/onchainos-skills) Agentic Wallet (TEE signing, no API key needed).

OKX 涨幅榜狙击手 — 每 10 秒扫描 Solana 1 小时涨幅榜 Top 20，新上榜代币经过三级安全过滤 + 动量评分后自动狙击入场。排名退出机制确保动量消退时及时平仓。所有链上操作由 [onchainos](https://github.com/okx/onchainos-skills) Agentic Wallet 驱动（TEE 签名，无需 API Key）。

## Features / 功能

- **Leaderboard Scanning / 榜单扫描** — Monitors Solana 1h gainers Top 20 every 10 seconds
- **3-Level Safety / 三级安全过滤** — 13 Slot Guard + 9 Advanced Safety + 3 Holder Risk checks
- **Momentum Scoring / 动量评分** — Composite score (0-125) from buy ratio, price change, traders, liquidity
- **Ranking Exit / 排名退出** — Highest priority: auto-sell 100% when token drops off Top 20
- **6-Layer Exit System / 6 层退出系统** — Ranking exit, hard stop, quick stop, trailing stop, time stop, tiered TP
- **Session Risk Control / 会话风控** — Daily loss limit, consecutive loss pause, cumulative loss stop
- **Wallet Audit / 钱包审计** — Periodic on-chain balance reconciliation
- **Web Dashboard / 实时仪表盘** — http://localhost:3244

## Install / 安装

```bash
npx skills add okx/plugin-store-community --skill top-rank-tokens-sniper
```

## Risk Warning / 风险提示

> Leaderboard data may be manipulated by wash trading. Rankings do not represent genuine market consensus. Always test in Paper Mode first.

> 涨幅榜数据可能被刷量操纵，排名不代表真正的市场共识。请先在纸盘模式下测试。

## License

MIT
