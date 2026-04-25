# Hyperliquid Auto Scalper

Intelligent multi-coin scalping skill for Hyperliquid Plugin Store.

## Features
- Automatic rotation across BTC-PERP, ETH-PERP, SOL-PERP
- RSI Mean-Reversion signals
- Strict low funding rate filter
- Micro-batch execution (10 small orders per signal)
- Low-risk parameters (2x leverage, tiny sizes)
- Fully optimized for Plugin Store Developer Challenge Season 1

## Project Structure
hyperliquid-auto-scalper/
├── src/
│   ├── main.ts
│   ├── strategy.ts
│   ├── execution.ts
│   ├── indicators.ts
│   ├── rotation.ts
│   └── risk.ts
├── tsconfig.json
├── package.json
├── plugin.yaml
├── README.md
├── SKILL.md
├── SUMMARY.md
└── .claude-plugin/plugin.json
## Build
```bash
npm install
npm run build
```
## Usage
After installing the skill, simply tell the agent:
"Run hyperliquid auto scalper"
The strategy will run automatically and all trades will be attributed correctly.

## Strategy Logic
Only trades when funding rate is very low
RSI < 30 → Buy
RSI > 70 → Sell
Creates 10 micro orders per signal  - TWAP (Time-Averaged Price) is an algorithmic trading strategy that breaks down a large buy or sell order into multiple smaller orders. Apply TWAP to get the best price.