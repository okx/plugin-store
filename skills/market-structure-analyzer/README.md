# Market Structure Analyzer v3.0

Crypto market-structure research agent with a live auto-refreshing dashboard. Delivers institutional-grade analysis using OKX CeFi CLI + OnchainOS CLI + direct HTTP — the same data Glassnode charges $49-999/month for.

## Features

- **Live Dashboard** — K-line candlestick charts (TradingView Lightweight Charts v4), RSI/MACD/Bollinger Bands overlays, timeframe selector (5m-1D), glass morphism UI
- **12-Signal Composite Score** — weighted scoring from -100 to +100 (BULLISH/NEUTRAL/BEARISH) with real-time updates
- **24+ Real-Time Indicators** — funding rates, OI + delta, basis, taker volume, long/short ratios, liquidation pressure, realized volatility, Fear & Greed, BTC dominance, stablecoin dry powder
- **Options Quant** — gamma wall (market-maker support/resistance), 25-delta skew, ATM IV, butterfly spread
- **On-Chain** — MVRV ratio + realized price (CoinMetrics), smart money signals + DEX hot tokens (OnchainOS)
- **Dune Analytics Exchange Flows** — ETH/stablecoin CEX net flows, per-exchange breakdown, whale transfer classification (optional)
- **Multi-Token** — BTC, ETH (Tier 1, full indicators), SOL/BNB/DOGE/AVAX/ARB/XRP/LINK (Tier 2), PEPE (Tier 3)
- **Zero Dependencies** — Python stdlib only, no pip install needed

## Quick Start

```bash
# Live dashboard (recommended)
python3 msa_server.py
# → http://localhost:8420

# CLI-only mode (backward compatible)
python3 scripts/fetch_market_data.py BTC ETH SOL 2>/dev/null
```

## Install

```bash
npx skills add okx/plugin-store --skill market-structure-analyzer
```

## Data Sources

| Source | Data | Cost |
|--------|------|------|
| OKX CeFi CLI (`okx market`) | Derivatives, price, OI, funding, L/S, candles | Free |
| OnchainOS CLI (`onchainos`) | Smart money signals, DEX hot tokens | Free |
| OKX Direct HTTP | Options chain (gamma wall, skew), taker volume | Free |
| CoinMetrics | MVRV, realized price (30d history) | Free |
| CoinGecko | BTC dominance, total market cap | Free |
| Alternative.me | Fear & Greed Index | Free |
| DefiLlama | Stablecoin market cap | Free |
| Dune Analytics | Exchange flows, whale transfers (optional, via MCP) | Free |

## Risk Warning

> This tool provides market data and analysis for informational purposes only. It is NOT financial or trading advice. All data is READ-ONLY from public APIs. Always verify data with primary sources and do your own research before making any trading decisions.

## License

MIT
