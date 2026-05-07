# mainstream-spot-order

Multi-chain DEX spot trading system with 6-signal ensemble, auto-research strategy optimization, and per-pair backtesting across SOL, ETH, BTC, BNB, AVAX, DOGE.

## Prerequisites

- **onchainos CLI** >= 2.0.0 — [install](https://docs.onchainos.com)
- **Python** >= 3.9 (stdlib only, zero pip dependencies)
- Agentic wallet logged in: `onchainos wallet login`

## Quick Start

```bash
# 1. Login to wallet
onchainos wallet login

# 2. Collect candle data
python3 collect.py --pair SOL --backfill    # one-time historical fill
python3 collect.py --pair SOL --daemon      # continuous collector + dashboard

# 3. Backtest
python3 backtest.py --pair SOL

# 4. Paper trade (default)
python3 live.py --pair SOL

# 5. Dashboard
# Open http://localhost:3250
```

## Supported Pairs

| Pair | Chain | Family |
|------|-------|--------|
| SOL | Solana | solana |
| ETH | Ethereum | evm |
| BTC | Ethereum (WBTC) | evm |
| BNB | BSC | evm |
| AVAX | Avalanche | evm |
| DOGE | Ethereum (ERC-20) | evm |

Add custom pairs: `python3 config.py --add-pair LINK --chain 1 --mint 0x... --decimals 18`

## Architecture

```
config.py (data) → okx.py (I/O) → collect.py (pipeline) → prepare.py (backtest)
                                                                  ↓
                                    strategy.py (brain) ← backtest.py (runner)
                                         ↓
                                    live.py (execution)
```

Only `strategy.py` is mutable (by auto-research). All other files are FIXED.

## Risk Warning

This skill is for educational and research purposes only. Spot trading carries substantial risk of loss. Always start with paper trading (`PAPER_TRADE = True`) and only switch to live after extensive validation.

## License

MIT
