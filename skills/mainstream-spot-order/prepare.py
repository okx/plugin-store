"""Backtest engine for multi-chain spot trading system.

FIXED — never modified by auto-research. Provides:
- load_candles(csv) -> list of dicts
- align_bars(base, btc) -> aligned pairs
- run_backtest(base, btc, strategy_mod, config) -> results dict
- compute_score(results) -> float
"""
from __future__ import annotations

import csv
import math
import os

import config


def load_candles(csv_path: str) -> list[dict]:
    """Load CSV into list of candle dicts, oldest-first, deduplicated by ts."""
    if not os.path.exists(csv_path):
        raise FileNotFoundError(f"No data file: {csv_path}")
    seen = {}
    with open(csv_path, "r") as f:
        reader = csv.DictReader(f)
        for row in reader:
            try:
                ts = int(row["ts"])
                seen[ts] = {
                    "ts": ts,
                    "o": float(row["o"]),
                    "h": float(row["h"]),
                    "l": float(row["l"]),
                    "c": float(row["c"]),
                    "vol": float(row["vol"]),
                }
            except (ValueError, KeyError):
                continue
    return [seen[ts] for ts in sorted(seen.keys())]


def align_bars(base: list[dict], btc: list[dict]) -> list[tuple[dict, dict]]:
    """Align base and BTC candles by timestamp. Only keep bars present in both."""
    btc_map = {b["ts"]: b for b in btc}
    pairs = []
    for s in base:
        if s["ts"] in btc_map:
            pairs.append((s, btc_map[s["ts"]]))
    return pairs


def run_backtest(base_candles: list[dict], btc_candles: list[dict],
                 strategy_mod, cfg=config) -> dict:
    """Run backtest over aligned candles using strategy module.

    strategy_mod must have:
      - init_state() -> dict
      - on_bar(state, base_bars, btc_bars, bar_idx) -> Signal

    Signal must have: target_position (0.0-1.0), confidence, reason

    Returns results dict with equity curve, trades, metrics.
    """
    pairs = align_bars(base_candles, btc_candles)
    if len(pairs) < 50:
        return {"error": "Not enough aligned bars", "count": len(pairs)}

    state = strategy_mod.init_state()
    equity = cfg.INITIAL_USDC
    position = 0.0        # fraction of equity in base (0.0 = flat, 1.0 = full)
    base_held = 0.0       # base token units held
    usdc_held = equity    # USDC held

    equity_curve = []
    trades = []
    bar_returns = []
    peak_equity = equity

    # Need lookback window — pass full history up to current bar
    base_history = []
    btc_history = []

    for i, (base_bar, btc_bar) in enumerate(pairs):
        base_history.append(base_bar)
        btc_history.append(btc_bar)

        # Current portfolio value
        base_price = base_bar["c"]
        current_equity = usdc_held + base_held * base_price

        # Get signal
        try:
            signal = strategy_mod.on_bar(state, base_history, btc_history, i)
        except Exception:
            signal = None

        target = signal.target_position if signal else position
        target = max(0.0, min(1.0, target))

        # Execute position change
        if abs(target - position) > 0.01:
            if target > position:
                # Buy base: spend USDC
                delta = target - position
                usdc_to_spend = delta * current_equity
                usdc_to_spend = min(usdc_to_spend, usdc_held)
                cost = usdc_to_spend * cfg.COST_PER_LEG
                slippage = usdc_to_spend * cfg.SLIPPAGE_PCT
                effective_usdc = usdc_to_spend - cost - slippage
                if effective_usdc > 0 and base_price > 0:
                    base_bought = effective_usdc / base_price
                    base_held += base_bought
                    usdc_held -= usdc_to_spend
                    trades.append({
                        "bar": i, "ts": base_bar["ts"], "side": "BUY",
                        "price": base_price, "usdc": usdc_to_spend,
                        "base": base_bought, "reason": getattr(signal, "reason", ""),
                    })
            else:
                # Sell base: receive USDC
                delta = position - target
                base_to_sell = delta * base_held / max(position, 0.01)
                base_to_sell = min(base_to_sell, base_held)
                gross_usdc = base_to_sell * base_price
                cost = gross_usdc * cfg.COST_PER_LEG
                slippage = gross_usdc * cfg.SLIPPAGE_PCT
                net_usdc = gross_usdc - cost - slippage
                if net_usdc > 0:
                    usdc_held += net_usdc
                    base_held -= base_to_sell
                    trades.append({
                        "bar": i, "ts": base_bar["ts"], "side": "SELL",
                        "price": base_price, "usdc": net_usdc,
                        "base": base_to_sell, "reason": getattr(signal, "reason", ""),
                    })

            position = target

        # Record equity
        new_equity = usdc_held + base_held * base_price
        if equity > 0:
            bar_ret = (new_equity - equity) / equity
        else:
            bar_ret = 0.0
        bar_returns.append(bar_ret)
        equity = new_equity
        peak_equity = max(peak_equity, equity)
        equity_curve.append({"ts": base_bar["ts"], "equity": round(equity, 2)})

    # Metrics
    total_bars = len(pairs)
    num_trades = len(trades)
    final_equity = equity
    total_return = (final_equity - cfg.INITIAL_USDC) / cfg.INITIAL_USDC

    # Max drawdown
    max_dd = 0.0
    peak = cfg.INITIAL_USDC
    for pt in equity_curve:
        peak = max(peak, pt["equity"])
        dd = (peak - pt["equity"]) / peak if peak > 0 else 0
        max_dd = max(max_dd, dd)

    # Sharpe ratio (annualized, assuming 15m bars)
    if bar_returns:
        mean_ret = sum(bar_returns) / len(bar_returns)
        var_ret = sum((r - mean_ret) ** 2 for r in bar_returns) / max(len(bar_returns) - 1, 1)
        std_ret = math.sqrt(var_ret) if var_ret > 0 else 1e-9
        bars_per_year = 365.25 * 24 * 4  # 15m bars
        sharpe = (mean_ret / std_ret) * math.sqrt(bars_per_year)
    else:
        sharpe = 0.0

    # Buy and hold comparison
    if pairs:
        bnh_return = (pairs[-1][0]["c"] - pairs[0][0]["c"]) / pairs[0][0]["c"]
    else:
        bnh_return = 0.0

    return {
        "total_bars": total_bars,
        "num_trades": num_trades,
        "final_equity": round(final_equity, 2),
        "total_return": round(total_return, 4),
        "max_drawdown": round(max_dd, 4),
        "sharpe": round(sharpe, 4),
        "buy_and_hold_return": round(bnh_return, 4),
        "trades": trades,
        "equity_curve": equity_curve,
    }


def compute_score(results: dict) -> float:
    """Composite score for strategy evaluation.

    score = sharpe * sqrt(min(trades/20, 1.0))
            - max_drawdown * 2.0
            - (trades / total_bars) * 0.1
            - underperformance_penalty
    """
    if "error" in results:
        return -999.0

    sharpe = results.get("sharpe", 0.0)
    max_dd = results.get("max_drawdown", 1.0)
    num_trades = results.get("num_trades", 0)
    total_bars = results.get("total_bars", 1)
    total_return = results.get("total_return", 0.0)
    bnh_return = results.get("buy_and_hold_return", 0.0)

    # Trade sufficiency factor
    trade_factor = math.sqrt(min(num_trades / config.MIN_TRADES_FOR_SCORE, 1.0))

    # Core score
    score = sharpe * trade_factor

    # Drawdown penalty (doubled for spot)
    score -= max_dd * 2.0

    # Overtrading penalty
    if total_bars > 0:
        score -= (num_trades / total_bars) * 0.1

    # Buy-and-hold underperformance penalty
    if bnh_return > 0 and total_return < bnh_return:
        underperformance = bnh_return - total_return
        score -= underperformance * 1.0

    return round(score, 4)
