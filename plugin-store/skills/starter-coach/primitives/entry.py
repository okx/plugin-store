"""
Entry trigger evaluators — 12 primitives.

Each function: evaluate_<type>(ctx, params) -> bool
  ctx: MarketContext with OHLCV bars, indicators, OnchainOS data
  params: dict from the spec's entry block (minus the "type" key)
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any
import time as _time

from onchainos import OnchainOS, WalletEvent, RankingItem


@dataclass
class Bar:
    ts: float       # unix seconds
    open: float
    high: float
    low: float
    close: float
    volume: float


@dataclass
class MarketContext:
    """Shared context passed to all evaluators."""
    bars: list[Bar] = field(default_factory=list)   # newest last
    current_price: float = 0.0
    timeframe: str = "5m"
    chain: str = "solana"
    token: str = ""
    # Pre-computed indicators (populated by engine before eval)
    ema: dict[int, list[float]] = field(default_factory=dict)   # period -> values
    sma: dict[int, list[float]] = field(default_factory=dict)
    rsi: dict[int, list[float]] = field(default_factory=dict)   # period -> values
    macd: dict[str, list[float]] = field(default_factory=dict)  # "macd"/"signal"/"hist"
    bbands: dict[str, list[float]] = field(default_factory=dict)  # "upper"/"middle"/"lower"
    # OnchainOS live data (populated for live_only triggers)
    onchainos: OnchainOS | None = None
    wallet_events: list[WalletEvent] = field(default_factory=list)
    ranking_items: list[RankingItem] = field(default_factory=list)


# ── Backtestable entries ─────────────────────────────────────────────────────

def evaluate_price_drop(ctx: MarketContext, params: dict) -> bool:
    """E-01: Fire when price drops pct% from lookback high."""
    pct = params["pct"]
    lookback = params["lookback_bars"]
    if len(ctx.bars) < lookback:
        return False
    window = ctx.bars[-lookback:]
    high = max(b.high for b in window)
    if high == 0:
        return False
    drop = (high - ctx.current_price) / high * 100
    return drop >= pct


def evaluate_price_breakout(ctx: MarketContext, params: dict) -> bool:
    """E-02: Fire when price breaks above lookback high (or below low)."""
    direction = params["direction"]
    lookback = params["lookback_bars"]
    confirm_pct = params.get("confirm_pct", 0)
    if len(ctx.bars) < lookback + 1:
        return False
    window = ctx.bars[-(lookback + 1):-1]  # exclude current bar
    if direction == "up":
        level = max(b.high for b in window)
        threshold = level * (1 + confirm_pct / 100)
        return ctx.current_price > threshold
    else:
        level = min(b.low for b in window)
        threshold = level * (1 - confirm_pct / 100)
        return ctx.current_price < threshold


def evaluate_ma_cross(ctx: MarketContext, params: dict) -> bool:
    """E-03: Fire when fast MA crosses above slow MA (golden cross)."""
    fast_p = params["fast_period"]
    slow_p = params["slow_period"]
    ma_type = params.get("ma_type", "EMA")
    source = ctx.ema if ma_type == "EMA" else ctx.sma
    fast = source.get(fast_p, [])
    slow = source.get(slow_p, [])
    if len(fast) < 2 or len(slow) < 2:
        return False
    # Cross: prev fast <= slow, now fast > slow
    return fast[-2] <= slow[-2] and fast[-1] > slow[-1]


def evaluate_rsi_threshold(ctx: MarketContext, params: dict) -> bool:
    """E-04: Fire when RSI crosses above/below a level."""
    period = params["period"]
    level = params["level"]
    direction = params["direction"]
    rsi = ctx.rsi.get(period, [])
    if len(rsi) < 2:
        return False
    if direction == "cross_up":
        return rsi[-2] <= level and rsi[-1] > level
    else:  # cross_down
        return rsi[-2] >= level and rsi[-1] < level


def evaluate_volume_spike(ctx: MarketContext, params: dict) -> bool:
    """E-05: Fire when current bar volume > multiplier * rolling average."""
    multiplier = params["multiplier"]
    avg_bars = params["avg_bars"]
    if len(ctx.bars) < avg_bars + 1:
        return False
    avg_window = ctx.bars[-(avg_bars + 1):-1]
    avg_vol = sum(b.volume for b in avg_window) / len(avg_window)
    if avg_vol == 0:
        return False
    return ctx.bars[-1].volume > multiplier * avg_vol


def evaluate_time_schedule(ctx: MarketContext, params: dict) -> bool:
    """E-06: Fire on fixed cadence. Engine calls this once per interval tick."""
    # The engine is responsible for calling this at the right time
    # based on interval and anchor_utc. When called, it always fires.
    return True


def evaluate_macd_cross(ctx: MarketContext, params: dict) -> bool:
    """E-09: Fire when MACD line crosses signal line."""
    direction = params["direction"]
    macd_line = ctx.macd.get("macd", [])
    signal_line = ctx.macd.get("signal", [])
    if len(macd_line) < 2 or len(signal_line) < 2:
        return False
    if direction == "cross_up":
        return macd_line[-2] <= signal_line[-2] and macd_line[-1] > signal_line[-1]
    else:
        return macd_line[-2] >= signal_line[-2] and macd_line[-1] < signal_line[-1]


def evaluate_bollinger_touch(ctx: MarketContext, params: dict) -> bool:
    """E-10: Fire when price touches upper or lower Bollinger Band."""
    band = params["band"]
    band_key = band  # "upper" or "lower"
    bb = ctx.bbands.get(band_key, [])
    if not bb:
        return False
    if band == "lower":
        return ctx.current_price <= bb[-1]
    else:
        return ctx.current_price >= bb[-1]


# ── Live-only entries ────────────────────────────────────────────────────────

def evaluate_smart_money_buy(ctx: MarketContext, params: dict) -> bool:
    """E-07: Fire when >= N smart-money wallets buy within window. Live-only."""
    min_wallets = params["min_wallets"]
    window_min = params["window_min"]
    min_usd = params.get("min_usd_each", 0)
    now = _time.time()
    cutoff = now - window_min * 60
    sm_buys: set[str] = set()
    for ev in ctx.wallet_events:
        if ev.side == "buy" and ev.timestamp >= cutoff and ev.usd_amount >= min_usd:
            sm_buys.add(ev.wallet)
    return len(sm_buys) >= min_wallets


def evaluate_dev_buy(ctx: MarketContext, params: dict) -> bool:
    """E-08: Fire when deployer wallet buys above threshold. Live-only."""
    min_usd = params["min_usd"]
    window_min = params.get("window_min", 1440)
    now = _time.time()
    cutoff = now - window_min * 60
    for ev in ctx.wallet_events:
        if ev.side == "buy" and ev.timestamp >= cutoff and ev.usd_amount >= min_usd:
            # Engine must pre-filter wallet_events to dev wallet only
            return True
    return False


def evaluate_ranking_entry(ctx: MarketContext, params: dict) -> bool:
    """E-11: Fire when token enters top-N of ranking list. Live-only."""
    top_n = params["top_n"]
    for item in ctx.ranking_items:
        if item.rank <= top_n:
            return True
    return False


def evaluate_wallet_copy_buy(ctx: MarketContext, params: dict) -> bool:
    """E-12: Fire when tracked wallet buys above threshold. Live-only."""
    min_usd = params["min_usd"]
    for ev in ctx.wallet_events:
        if ev.side == "buy" and ev.usd_amount >= min_usd:
            return True
    return False


# ── Dispatcher ───────────────────────────────────────────────────────────────

ENTRY_EVALUATORS: dict[str, Any] = {
    "price_drop": evaluate_price_drop,
    "price_breakout": evaluate_price_breakout,
    "ma_cross": evaluate_ma_cross,
    "rsi_threshold": evaluate_rsi_threshold,
    "volume_spike": evaluate_volume_spike,
    "time_schedule": evaluate_time_schedule,
    "smart_money_buy": evaluate_smart_money_buy,
    "dev_buy": evaluate_dev_buy,
    "macd_cross": evaluate_macd_cross,
    "bollinger_touch": evaluate_bollinger_touch,
    "ranking_entry": evaluate_ranking_entry,
    "wallet_copy_buy": evaluate_wallet_copy_buy,
}


def evaluate_entry(ctx: MarketContext, entry_spec: dict) -> bool:
    """Dispatch to the correct entry evaluator."""
    entry_type = entry_spec["type"]
    params = {k: v for k, v in entry_spec.items() if k != "type"}
    evaluator = ENTRY_EVALUATORS.get(entry_type)
    if evaluator is None:
        raise ValueError(f"Unknown entry type: {entry_type}")
    return evaluator(ctx, params)
