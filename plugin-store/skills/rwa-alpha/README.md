# RWA Alpha — Real World Asset Intelligence Trading

Macro event detection + Polymarket confirmation + on-chain price action → auto-trade tokenized treasury, gold, yield, and governance tokens via OKX DEX.

RWA 宏观事件驱动交易 — NewsNow 宏观事件检测 + Polymarket 概率确认 + 链上价格行为 → 自动交易代币化国债、黄金、收益和治理代币。

## How It Works / 工作原理

```
1. Macro Detection  — Scans NewsNow for macro events (rate decisions, credit cycles)
2. Polymarket Gate  — Confirms signal with prediction market probability
3. On-chain Action  — Trades tokenized RWA tokens via OKX DEX
4. Exit Management  — NAV premium/discount based take profit and stop loss
```

## Three Modes / 三种模式

| Mode | Risk | Target Tokens |
|------|------|---------------|
| Yield Optimizer | Conservative | USDY, OUSG, bIB01, STBT |
| Macro Trader | Balanced | PAXG, ONDO, CFG, PENDLE |
| Full Alpha | Aggressive | PLUME, OM, GFI, TRU |

## Features / 功能

- **Macro Event Detection / 宏观事件检测** — NewsNow RSS monitoring for rate decisions, credit events
- **Polymarket Confirmation / 预测市场确认** — Uses market probability as signal filter
- **NAV Premium Tracking / 净值溢价追踪** — Buys discount, sells premium
- **Multi-chain / 多链支持** — Ethereum + Solana via Agentic Wallet
- **Three Modes / 三种模式** — Yield Optimizer, Macro Trader, Full Alpha
- **Paper Mode Default / 默认纸盘模式** — Live mode requires explicit confirmation
- **TEE Signing / TEE 签名** — All trades via OnchainOS Agentic Wallet, no API key needed
- **Web Dashboard / 实时仪表盘** — Real-time positions, signals, macro feed

## Install / 安装

```bash
plugin-store install rwa-alpha
```

## Supported Tokens / 支持代币

`USDY` `OUSG` `bIB01` `STBT` `PAXG` `ONDO` `CFG` `PENDLE` `PLUME` `OM` `GFI` `TRU`

## Risk Warning / 风险提示

> RWA trading involves liquidity risk, macro prediction errors, smart contract risk, and slippage. Always start in paper mode. Never invest more than you can afford to lose.

> RWA 交易存在流动性风险、宏观预测误差、智能合约风险和滑点风险。请务必先在纸盘模式下测试，切勿投入超出承受能力的资金。

## License

MIT
