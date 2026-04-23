# poly-hl-signal-bridge

Cross-analyze Polymarket prediction odds with Hyperliquid funding rates and OKX smart-money signals to surface convergence and divergence opportunities.

## What it does

This skill connects three data sources that most traders look at separately:

| Source | Data | What it tells you |
|--------|------|-------------------|
| Polymarket | Event odds, order book depth, wallet positions | What the crowd thinks will happen |
| Hyperliquid | Funding rates, open interest, wallet PnL | How professional traders are positioned |
| OKX OnchainOS | Smart-money signals, leaderboard | What top wallets are actually doing |

When all three align → **convergence signal** (high confidence).  
When they point opposite directions → **divergence** (potential mispricing to investigate).

## Install

```bash
npx skills add okx/plugin-store --skill poly-hl-signal-bridge
```

## Usage examples

- "What's trending on Polymarket right now?"
- "Scan for convergence signals between Polymarket and Hyperliquid"
- "Analyze wallet 0xabc... on Hyperliquid"
- "What positions does 0xdef... hold on Polymarket?"
- "Is the BTC funding rate bullish or bearish on Hyperliquid?"

## Requirements

- OKX OnchainOS CLI (`npx skills add okx/onchainos-skills`)
- Internet access to Polymarket and Hyperliquid public APIs
- No private keys or API keys required for read-only analysis

## License

MIT
