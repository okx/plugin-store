# Polymarket Signal Bridge

将 Polymarket 预测市场赔率、Hyperliquid 资金费率与 OKX 链上聪明钱信号三合一，自动识别多空共振与背离机会。

## 功能介绍

| 数据来源 | 获取内容 | 告诉你什么 |
|--------|--------|---------|
| Polymarket | 事件赔率、订单簿深度、钱包持仓 | 市场大众的判断 |
| Hyperliquid | 资金费率、持仓量、钱包盈亏 | 专业交易者的仓位 |
| OKX OnchainOS | 聪明钱信号、排行榜 | 顶级钱包实际在做什么 |

三者方向一致为共振信号，置信度高。三者方向相反为背离信号，存在潜在错误定价。

## 使用示例

- Polymarket 现在什么市场最热
- 帮我扫描 Polymarket 和 Hyperliquid 的共振信号
- 分析钱包地址在 Hyperliquid 上的持仓
- 钱包在 Polymarket 上有哪些仓位
- BTC 在 Hyperliquid 上的资金费率是多少
- 今天有什么值得关注的交易机会

## 安装

```bash
npx skills add okx/plugin-store --skill polymarketsignalbridge
```

## 环境要求

- OKX OnchainOS CLI
- 网络可访问 Polymarket 和 Hyperliquid 公开 API
- 无需私钥或 API Key，所有操作均为只读查询

## 风险说明

本插件仅提供数据聚合与分析，不自动执行任何交易。所有交易决策由用户自行判断。预测市场和永续合约交易存在本金损失风险，请仅使用可承受损失的资金。本插件不构成投资建议。

## 许可证

MIT
