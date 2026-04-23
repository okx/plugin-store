# Polymarket BTC 5-min Momentum — Summary

**What it is:** A small Python strategy that bets the direction of Polymarket's rolling 5-minute BTC Up/Down markets based on the last 15 minutes of Binance BTC/USDT 1-minute candles.

**How it trades:** For each upcoming 5-min slot, the script computes momentum (close/open % change). If |momentum| exceeds the threshold (default 0.05%), it places a FOK (fill-or-kill) market buy on the matching outcome token via `polymarket-plugin buy --strategy-id polymarket-btc-5min-momentum`. Otherwise it skips the slot.

**Attribution:** Every buy carries `--strategy-id polymarket-btc-5min-momentum`, so `polymarket-plugin` reports the order to the OKX backend for strategy PnL attribution.

**Dependencies:**
- Python ≥ 3.9 (stdlib only; no pip installs)
- `polymarket-plugin` ≥ 0.4.10 on `PATH`
- Public Binance klines API (no auth)

**Quickstart:**
```bash
python3 strategy.py --dry-run              # preview one slot
python3 strategy.py --amount 2.5           # one real bet
python3 strategy.py --loop --amount 2.5    # run continuously
```

**Safety:**
- FOK market orders; no resting exposure across slots.
- `--dry-run` forwards to `polymarket-plugin buy --dry-run` (no on-chain action).
- No secrets or private keys — all wallet operations happen inside `polymarket-plugin` / `onchainos`.

**Disclaimer:** This is a reference / demonstration strategy. A single momentum window is not profitable edge over ~50/50 binary markets with Polymarket's fee structure. Use it as a template for building richer strategies that leverage attribution reporting.
