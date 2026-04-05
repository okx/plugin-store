# polymarket-agent-skills

Polymarket prediction market integration for AI agents — trading, market data, WebSocket streaming, cross-chain bridge, and gasless transactions. Maintained by [Polymarket](https://github.com/Polymarket).

## Features

- **Prediction Market Trading** — Place, cancel, and manage bets on Polymarket events
- **Market Data** — Real-time odds, volume, liquidity, and historical data
- **WebSocket Streaming** — Live price feeds and order book updates
- **Cross-Chain Bridge** — Bridge assets from Ethereum, Arbitrum, Base, Optimism, BNB, Solana to Polygon
- **Gasless Transactions** — Execute trades without holding MATIC for gas

## Supported Chains

Polygon (primary), Ethereum, Arbitrum, Base, Optimism, BNB, Solana (bridge sources)

## Install

```bash
npx skills add okx/plugin-store-community --skill polymarket-agent-skills
```

Or install via Claude Marketplace:

```bash
npx skills add Polymarket/agent-skills
```

## Source

- Plugin Store entry: [okx/plugin-store-community](https://github.com/okx/plugin-store-community/tree/main/submissions/polymarket-agent-skills)
- Upstream repo: [Polymarket/agent-skills](https://github.com/Polymarket/agent-skills)

## License

MIT
