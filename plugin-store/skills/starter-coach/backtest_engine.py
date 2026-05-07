"""
Backtest engine — replay a validated spec against historical OHLCV data.

Hard-rejects any spec where live_only == true (must go through paper_gate instead).
"""
from __future__ import annotations

import json
from dataclasses import dataclass, field
from typing import Any

from harness import validate_spec, LIVE_ONLY_TYPES
from primitives.entry import MarketContext, Bar, evaluate_entry
from primitives.exit import Position, ExitSignal, evaluate_all_exits
from primitives.filter import evaluate_filters
from primitives.sizing import compute_size
from primitives.risk import PortfolioState, check_all_overlays


@dataclass
class Trade:
    token: str
    entry_price: float
    exit_price: float
    entry_bar: int
    exit_bar: int
    size_usd: float
    pnl_usd: float
    pnl_pct: float
    exit_reason: str


@dataclass
class BacktestResult:
    ok: bool = False
    error: str = ""
    trades: list[Trade] = field(default_factory=list)
    total_pnl_usd: float = 0.0
    total_pnl_pct: float = 0.0
    win_rate: float = 0.0
    max_drawdown_pct: float = 0.0
    sharpe: float = 0.0
    trade_count: int = 0


def _compute_indicators(bars: list[Bar], spec: dict) -> dict[str, Any]:
    """Pre-compute indicators needed by the spec's entry/exit/filters."""
    closes = [b.close for b in bars]
    indicators: dict[str, Any] = {"ema": {}, "sma": {}, "rsi": {}, "macd": {}, "bbands": {}}

    # Collect periods needed
    entry = spec.get("entry", {})
    etype = entry.get("type", "")

    # SMA/EMA
    for period in _extract_ma_periods(spec):
        if len(closes) >= period:
            indicators["sma"][period] = _sma(closes, period)
            indicators["ema"][period] = _ema(closes, period)

    # RSI
    if etype == "rsi_threshold":
        p = entry.get("period", 14)
        indicators["rsi"][p] = _rsi(closes, p)

    # MACD
    if etype == "macd_cross":
        fast = entry.get("fast_period", 12)
        slow = entry.get("slow_period", 26)
        sig = entry.get("signal_period", 9)
        macd_l, signal_l, hist_l = _macd(closes, fast, slow, sig)
        indicators["macd"] = {"macd": macd_l, "signal": signal_l, "hist": hist_l}

    # Bollinger Bands
    if etype == "bollinger_touch":
        p = entry.get("period", 20)
        std = entry.get("std_dev", 2.0)
        upper, middle, lower = _bbands(closes, p, std)
        indicators["bbands"] = {"upper": upper, "middle": middle, "lower": lower}

    return indicators


def _extract_ma_periods(spec: dict) -> set[int]:
    periods: set[int] = set()
    entry = spec.get("entry", {})
    if entry.get("type") == "ma_cross":
        periods.add(entry.get("fast_period", 10))
        periods.add(entry.get("slow_period", 50))
    for filt in spec.get("filters", []):
        if filt.get("type") == "market_regime":
            periods.add(filt.get("ma_period", 200))
    return periods


def _sma(values: list[float], period: int) -> list[float]:
    result: list[float] = []
    for i in range(len(values)):
        if i < period - 1:
            result.append(0.0)
        else:
            result.append(sum(values[i - period + 1:i + 1]) / period)
    return result


def _ema(values: list[float], period: int) -> list[float]:
    result: list[float] = []
    k = 2 / (period + 1)
    for i, v in enumerate(values):
        if i == 0:
            result.append(v)
        else:
            result.append(v * k + result[-1] * (1 - k))
    return result


def _rsi(values: list[float], period: int) -> list[float]:
    result: list[float] = [50.0]  # default for first value
    gains: list[float] = []
    losses: list[float] = []
    for i in range(1, len(values)):
        delta = values[i] - values[i - 1]
        gains.append(max(delta, 0))
        losses.append(max(-delta, 0))
        if i < period:
            result.append(50.0)
            continue
        if i == period:
            avg_gain = sum(gains[-period:]) / period
            avg_loss = sum(losses[-period:]) / period
        else:
            avg_gain = (avg_gain * (period - 1) + gains[-1]) / period
            avg_loss = (avg_loss * (period - 1) + losses[-1]) / period
        if avg_loss == 0:
            result.append(100.0)
        else:
            rs = avg_gain / avg_loss
            result.append(100 - 100 / (1 + rs))
    return result


def _macd(values: list[float], fast: int, slow: int, signal: int):
    fast_ema = _ema(values, fast)
    slow_ema = _ema(values, slow)
    macd_line = [f - s for f, s in zip(fast_ema, slow_ema)]
    signal_line = _ema(macd_line, signal)
    hist = [m - s for m, s in zip(macd_line, signal_line)]
    return macd_line, signal_line, hist


def _bbands(values: list[float], period: int, std_dev: float):
    sma_vals = _sma(values, period)
    upper, lower = [], []
    for i in range(len(values)):
        if i < period - 1:
            upper.append(0.0)
            lower.append(0.0)
        else:
            window = values[i - period + 1:i + 1]
            mean = sma_vals[i]
            variance = sum((x - mean) ** 2 for x in window) / period
            std = variance ** 0.5
            upper.append(mean + std_dev * std)
            lower.append(mean - std_dev * std)
    return upper, sma_vals, lower


def run_backtest(
    spec: dict,
    bars: list[dict],
    initial_equity: float = 10000.0,
) -> BacktestResult:
    """
    Run a backtest on historical data.

    Args:
        spec: validated strategy spec dict
        bars: list of {ts, open, high, low, close, volume} dicts
        initial_equity: starting capital in USD

    Returns:
        BacktestResult
    """
    # 1. Validate
    ok, errors, meta = validate_spec(spec)
    if not ok:
        return BacktestResult(ok=False, error=f"Spec validation failed: {'; '.join(errors)}")

    if meta["live_only"]:
        return BacktestResult(
            ok=False,
            error="live_only spec cannot be backtested. Use paper_gate for graduation."
        )

    # 2. Parse bars
    parsed_bars = [
        Bar(ts=b["ts"], open=b["open"], high=b["high"],
            low=b["low"], close=b["close"], volume=b["volume"])
        for b in bars
    ]
    if len(parsed_bars) < 50:
        return BacktestResult(ok=False, error="Insufficient bars (need >= 50)")

    # 3. Pre-compute indicators
    indicators = _compute_indicators(parsed_bars, spec)

    # 4. Simulate
    equity = initial_equity
    peak_equity = equity
    max_dd = 0.0
    trades: list[Trade] = []
    position: Position | None = None
    portfolio = PortfolioState(
        equity_usd=equity, peak_equity_usd=equity, session_start_ts=parsed_bars[0].ts
    )
    returns: list[float] = []

    entry_spec = spec["entry"]
    exit_spec = spec["exit"]
    sizing_spec = spec["sizing"]
    filters = spec.get("filters", [])
    overlays = spec.get("risk_overlays", [])

    for i in range(50, len(parsed_bars)):
        ctx = MarketContext(
            bars=parsed_bars[:i + 1],
            current_price=parsed_bars[i].close,
            timeframe=spec.get("instrument", {}).get("timeframe", "5m"),
            token=spec.get("instrument", {}).get("symbol", ""),
        )
        ctx.ema = indicators.get("ema", {})
        ctx.sma = indicators.get("sma", {})
        ctx.rsi = indicators.get("rsi", {})
        ctx.macd = indicators.get("macd", {})
        ctx.bbands = indicators.get("bbands", {})

        # Check exits first
        if position is not None:
            sig = evaluate_all_exits(ctx, position, exit_spec)
            if sig:
                pnl_pct = (ctx.current_price - position.entry_price) / position.entry_price * 100
                sell_frac = sig.sell_pct / 100
                pnl_usd = position.size_usd * sell_frac * pnl_pct / 100
                trades.append(Trade(
                    token=position.token, entry_price=position.entry_price,
                    exit_price=ctx.current_price, entry_bar=position.entry_bar_idx,
                    exit_bar=i, size_usd=position.size_usd * sell_frac,
                    pnl_usd=pnl_usd, pnl_pct=pnl_pct, exit_reason=sig.reason,
                ))
                equity += pnl_usd
                returns.append(pnl_pct)
                if pnl_usd < 0:
                    portfolio.consecutive_losses += 1
                else:
                    portfolio.consecutive_losses = 0
                if sig.sell_pct >= 100:
                    position = None
                    portfolio.open_positions -= 1
                else:
                    position.size_usd *= (1 - sell_frac)
                portfolio.equity_usd = equity
                if equity > peak_equity:
                    peak_equity = equity
                    portfolio.peak_equity_usd = peak_equity
                dd = (peak_equity - equity) / peak_equity * 100 if peak_equity > 0 else 0
                max_dd = max(max_dd, dd)

        # Try entry if no position
        if position is None:
            # Check risk overlays
            portfolio.equity_usd = equity
            overlay_ok, _ = check_all_overlays(portfolio, overlays)
            if not overlay_ok:
                continue
            # Check filters
            filter_ok, _ = evaluate_filters(ctx, filters)
            if not filter_ok:
                continue
            # Check entry
            if evaluate_entry(ctx, entry_spec):
                size = compute_size(equity, ctx, sizing_spec)
                if size > 0:
                    position = Position(
                        token=ctx.token, entry_price=ctx.current_price,
                        entry_ts=parsed_bars[i].ts, entry_bar_idx=i,
                        size_usd=size, peak_price=ctx.current_price,
                    )
                    portfolio.open_positions += 1
                    portfolio.trades_today += 1

    # 5. Close any remaining position at last bar
    if position is not None:
        final_price = parsed_bars[-1].close
        pnl_pct = (final_price - position.entry_price) / position.entry_price * 100
        pnl_usd = position.size_usd * pnl_pct / 100
        trades.append(Trade(
            token=position.token, entry_price=position.entry_price,
            exit_price=final_price, entry_bar=position.entry_bar_idx,
            exit_bar=len(parsed_bars) - 1, size_usd=position.size_usd,
            pnl_usd=pnl_usd, pnl_pct=pnl_pct, exit_reason="end_of_data",
        ))
        equity += pnl_usd
        returns.append(pnl_pct)

    # 6. Compute stats
    total_pnl = equity - initial_equity
    total_pnl_pct = total_pnl / initial_equity * 100 if initial_equity > 0 else 0
    wins = sum(1 for t in trades if t.pnl_usd > 0)
    win_rate = wins / len(trades) * 100 if trades else 0
    sharpe = 0.0
    if returns:
        avg_ret = sum(returns) / len(returns)
        if len(returns) > 1:
            variance = sum((r - avg_ret) ** 2 for r in returns) / (len(returns) - 1)
            std_ret = variance ** 0.5
            sharpe = (avg_ret / std_ret) * (252 ** 0.5) if std_ret > 0 else 0

    return BacktestResult(
        ok=True,
        trades=trades,
        total_pnl_usd=round(total_pnl, 2),
        total_pnl_pct=round(total_pnl_pct, 2),
        win_rate=round(win_rate, 1),
        max_drawdown_pct=round(max_dd, 2),
        sharpe=round(sharpe, 2),
        trade_count=len(trades),
    )
