"""SOL/USDC spot strategy — MUTABLE file for auto-research.

6-signal ensemble adapted for spot-only (long or flat).
Returns Signal(target_position, confidence, reason) where target_position in [0.0, 1.0].

Signals (each votes LONG or FLAT):
1. Momentum — N-bar return > 0
2. VShort Momentum — 3-bar return > 0
3. EMA Crossover — fast(7) > slow(26)
4. RSI(8) — between oversold(30) and overbought(75)
5. MACD — line > signal
6. BB Compression — width squeezed + price above midline

BTC overlay — half-vote weight based on BTC momentum direction.

Entry: >=4 of 6 votes + BTC bonus -> go LONG
Exit: votes drop below threshold OR ATR trailing stop hit -> go FLAT
"""
from __future__ import annotations


class Signal:
    __slots__ = ("target_position", "confidence", "reason")

    def __init__(self, target_position: float, confidence: float, reason: str):
        self.target_position = target_position
        self.confidence = confidence
        self.reason = reason


# ── Parameters (tunable by auto-research) ────────────────────────────

MOMENTUM_PERIOD = 24
VSHORT_PERIOD = 10
EMA_FAST = 8
EMA_SLOW = 40
RSI_PERIOD = 8
RSI_OVERSOLD = 30
RSI_OVERBOUGHT = 75
MACD_FAST = 12
MACD_SLOW = 30
MACD_SIGNAL = 9
BB_PERIOD = 20
BB_STD = 2.0
BB_SQUEEZE_THRESHOLD = 0.03  # width/midline ratio
ATR_PERIOD = 14
ATR_TRAILING_MULT = 5.5
PROFIT_TIGHTEN_PCT = 0.03  # tighten exit threshold when up 3%+
ENTRY_THRESHOLD = 5.0   # votes needed to enter (out of 6 + BTC bonus)
EXIT_THRESHOLD = 2.5    # votes below this -> exit
BTC_MOMENTUM_PERIOD = 24
MIN_HOLD_BARS = 8       # minimum bars (2h) to hold before exiting
MIN_MOMENTUM_PCT = 0.04 # minimum momentum return to enter (must exceed round-trip cost)
COOLDOWN_BARS = 1       # bars to wait after exit before re-entering

# ── Mean-reversion entry (alternative path) ──
MR_RSI_THRESHOLD = 22       # RSI must be deeply oversold
MR_BB_PROXIMITY = 0.005     # price within 0.5% of lower BB
MR_DROP_BARS = 8            # lookback for recent drop
MR_DROP_PCT = 0.04          # must have dropped 4%+ recently
MR_POS_SIZE = 0.25          # smaller position for mean-rev trades
MR_ATR_MULT = 3.0           # tighter trailing stop for mean-rev


# ── Helpers ──────────────────────────────────────────────────────────

def _ema(values: list[float], period: int) -> list[float]:
    """Exponential moving average. Returns list same length as input (NaN-free)."""
    if not values:
        return []
    result = [values[0]]
    k = 2.0 / (period + 1)
    for i in range(1, len(values)):
        result.append(values[i] * k + result[-1] * (1 - k))
    return result


def _rsi(closes: list[float], period: int) -> float:
    """RSI of the last `period` bars."""
    if len(closes) < period + 1:
        return 50.0  # neutral
    gains = []
    losses = []
    for i in range(-period, 0):
        delta = closes[i] - closes[i - 1]
        if delta > 0:
            gains.append(delta)
            losses.append(0.0)
        else:
            gains.append(0.0)
            losses.append(abs(delta))
    avg_gain = sum(gains) / period
    avg_loss = sum(losses) / period
    if avg_loss == 0:
        return 100.0
    rs = avg_gain / avg_loss
    return 100.0 - (100.0 / (1.0 + rs))


def _atr(bars: list[dict], period: int) -> float:
    """Average True Range over last `period` bars."""
    if len(bars) < period + 1:
        return 0.0
    trs = []
    for i in range(-period, 0):
        h = bars[i]["h"]
        l = bars[i]["l"]
        pc = bars[i - 1]["c"]
        tr = max(h - l, abs(h - pc), abs(l - pc))
        trs.append(tr)
    return sum(trs) / len(trs) if trs else 0.0


def _bb(closes: list[float], period: int, num_std: float) -> tuple[float, float, float]:
    """Bollinger Bands: (upper, middle, lower)."""
    if len(closes) < period:
        mid = closes[-1] if closes else 0
        return mid, mid, mid
    window = closes[-period:]
    mid = sum(window) / period
    variance = sum((x - mid) ** 2 for x in window) / period
    std = variance ** 0.5
    return mid + num_std * std, mid, mid - num_std * std


# ── State ────────────────────────────────────────────────────────────

def init_state() -> dict:
    """Initialize strategy state."""
    return {
        "in_position": False,
        "trailing_stop": 0.0,
        "entry_price": 0.0,
        "highest_since_entry": 0.0,
        "bars_held": 0,
        "cooldown": 0,
        "entry_type": "trend",  # "trend" or "meanrev"
    }


# ── Core ─────────────────────────────────────────────────────────────

def on_bar(state: dict, sol_bars: list[dict], btc_bars: list[dict],
           bar_idx: int) -> Signal:
    """Process one bar. Returns Signal with target_position in [0.0, 1.0]."""

    # Need enough history
    min_lookback = max(EMA_SLOW, MACD_SLOW + MACD_SIGNAL, BB_PERIOD, MOMENTUM_PERIOD) + 5
    if len(sol_bars) < min_lookback:
        return Signal(0.0, 0.0, "insufficient_history")

    closes = [b["c"] for b in sol_bars]
    price = closes[-1]

    # ── Signal 1: Momentum ──
    mom_ret = (price - closes[-MOMENTUM_PERIOD - 1]) / closes[-MOMENTUM_PERIOD - 1]
    vote_momentum = 1.0 if mom_ret > 0 else 0.0

    # ── Signal 2: VShort Momentum ──
    vshort_ret = (price - closes[-VSHORT_PERIOD - 1]) / closes[-VSHORT_PERIOD - 1]
    vote_vshort = 1.0 if vshort_ret > 0 else 0.0

    # ── Signal 3: EMA Crossover ──
    ema_f = _ema(closes, EMA_FAST)
    ema_s = _ema(closes, EMA_SLOW)
    vote_ema = 1.0 if ema_f[-1] > ema_s[-1] else 0.0

    # ── Signal 4: RSI ──
    rsi_val = _rsi(closes, RSI_PERIOD)
    vote_rsi = 1.0 if RSI_OVERSOLD < rsi_val < RSI_OVERBOUGHT else 0.0

    # ── Signal 5: MACD ──
    ema_macd_fast = _ema(closes, MACD_FAST)
    ema_macd_slow = _ema(closes, MACD_SLOW)
    macd_line = [f - s for f, s in zip(ema_macd_fast, ema_macd_slow)]
    macd_signal = _ema(macd_line, MACD_SIGNAL)
    vote_macd = 1.0 if macd_line[-1] > macd_signal[-1] else 0.0

    # ── Signal 6: BB Compression ──
    bb_upper, bb_mid, bb_lower = _bb(closes, BB_PERIOD, BB_STD)
    bb_width = (bb_upper - bb_lower) / bb_mid if bb_mid > 0 else 0
    vote_bb = 1.0 if (bb_width < BB_SQUEEZE_THRESHOLD and price > bb_mid) else 0.0

    # ── BTC overlay (half-vote) ──
    btc_bonus = 0.0
    if len(btc_bars) > BTC_MOMENTUM_PERIOD + 1:
        btc_closes = [b["c"] for b in btc_bars]
        btc_ret = (btc_closes[-1] - btc_closes[-BTC_MOMENTUM_PERIOD - 1]) / btc_closes[-BTC_MOMENTUM_PERIOD - 1]
        btc_bonus = 0.5 if btc_ret > 0 else 0.0

    total_votes = vote_momentum + vote_vshort + vote_ema + vote_rsi + vote_macd + vote_bb + btc_bonus

    # ── ATR trailing stop ──
    atr = _atr(sol_bars, ATR_PERIOD)

    if state["in_position"]:
        state["bars_held"] += 1
        state["highest_since_entry"] = max(state["highest_since_entry"], price)
        atr_mult = MR_ATR_MULT if state.get("entry_type") == "meanrev" else ATR_TRAILING_MULT
        state["trailing_stop"] = state["highest_since_entry"] - atr_mult * atr

        # Adaptive exit: tighten threshold when in profit to lock gains
        unrealized = (price - state["entry_price"]) / state["entry_price"] if state["entry_price"] > 0 else 0
        effective_exit = EXIT_THRESHOLD + (1.0 if unrealized >= PROFIT_TIGHTEN_PCT else 0.0)

        # Exit conditions (respect minimum hold period, except trailing stop + profit target)
        if price <= state["trailing_stop"]:
            state["in_position"] = False
            state["bars_held"] = 0
            state["cooldown"] = COOLDOWN_BARS
            return Signal(0.0, 0.8, f"trailing_stop hit={state['trailing_stop']:.2f}")

        if state["bars_held"] >= MIN_HOLD_BARS and total_votes < effective_exit:
            state["in_position"] = False
            state["bars_held"] = 0
            state["cooldown"] = COOLDOWN_BARS
            return Signal(0.0, 0.6, f"votes_low={total_votes:.1f}")

        # Stay in position — maintain original size
        return Signal(state.get("pos_size", 1.0), total_votes / 6.5, f"hold votes={total_votes:.1f}")

    else:
        # Cooldown after exit
        if state["cooldown"] > 0:
            state["cooldown"] -= 1
            return Signal(0.0, 0.0, f"cooldown={state['cooldown']}")

        # BTC downtrend veto — don't enter when BTC is falling
        btc_veto = False
        if len(btc_bars) > BTC_MOMENTUM_PERIOD + 1:
            btc_closes = [b["c"] for b in btc_bars]
            btc_mom = (btc_closes[-1] - btc_closes[-BTC_MOMENTUM_PERIOD - 1]) / btc_closes[-BTC_MOMENTUM_PERIOD - 1]
            btc_veto = btc_mom < -0.05  # BTC down >5% → veto

        # Price above open of 2 bars ago (broader bullish check)
        green_candle = price > sol_bars[-2]["o"]

        # Entry condition: votes + momentum magnitude filter + no BTC veto + green candle
        if total_votes >= ENTRY_THRESHOLD and mom_ret >= MIN_MOMENTUM_PCT and not btc_veto and green_candle:
            # Scale position size: 0.6 at threshold, up to 1.0 at max votes
            pos_size = min(1.0, 0.6 + 0.4 * (total_votes - ENTRY_THRESHOLD) / 2.5)
            state["in_position"] = True
            state["entry_price"] = price
            state["highest_since_entry"] = price
            state["trailing_stop"] = price - ATR_TRAILING_MULT * atr
            state["pos_size"] = pos_size
            state["entry_type"] = "trend"
            reasons = []
            if vote_momentum: reasons.append("mom")
            if vote_vshort: reasons.append("vshort")
            if vote_ema: reasons.append("ema")
            if vote_rsi: reasons.append("rsi")
            if vote_macd: reasons.append("macd")
            if vote_bb: reasons.append("bb")
            if btc_bonus: reasons.append("btc+")
            return Signal(pos_size, total_votes / 6.5,
                          f"entry votes={total_votes:.1f} size={pos_size:.0%} [{'+'.join(reasons)}]")

        # ── Mean-reversion entry (alternative path) ──
        # Catches oversold bounces near BB lower band when trend entry doesn't fire
        if not btc_veto and rsi_val <= MR_RSI_THRESHOLD and len(closes) > MR_DROP_BARS:
            # Price near lower Bollinger Band
            bb_dist = (price - bb_lower) / price if price > 0 else 1.0
            near_lower_bb = bb_dist <= MR_BB_PROXIMITY

            # Recent significant drop (confirms oversold is real, not just low vol)
            recent_high = max(closes[-MR_DROP_BARS - 1:-1])
            drop_pct = (recent_high - price) / recent_high if recent_high > 0 else 0
            significant_drop = drop_pct >= MR_DROP_PCT

            # BTC not in freefall (relaxed vs trend entry — allow mild BTC weakness)
            btc_ok = True
            if len(btc_bars) > BTC_MOMENTUM_PERIOD + 1:
                btc_closes = [b["c"] for b in btc_bars]
                btc_mom = (btc_closes[-1] - btc_closes[-BTC_MOMENTUM_PERIOD - 1]) / btc_closes[-BTC_MOMENTUM_PERIOD - 1]
                btc_ok = btc_mom > -0.08  # allow up to 8% BTC drop (vs 5% for trend)

            if near_lower_bb and significant_drop and btc_ok:
                state["in_position"] = True
                state["entry_price"] = price
                state["highest_since_entry"] = price
                state["trailing_stop"] = price - MR_ATR_MULT * atr
                state["pos_size"] = MR_POS_SIZE
                state["entry_type"] = "meanrev"
                return Signal(MR_POS_SIZE, 0.5,
                              f"MR entry rsi={rsi_val:.0f} drop={drop_pct:.1%} bb_dist={bb_dist:.1%}")

        return Signal(0.0, 0.0, f"flat votes={total_votes:.1f}")


# ── Analytics (for dashboard) ─────────────────────────────────────

def analyze(state: dict, sol_bars: list[dict], btc_bars: list[dict]) -> dict:
    """Compute all intermediate signal values for dashboard display.

    Returns dict with: votes breakdown, RSI, ATR, BB, momentum, BTC overlay,
    position state, and entry/exit thresholds.
    """
    min_lookback = max(EMA_SLOW, MACD_SLOW + MACD_SIGNAL, BB_PERIOD, MOMENTUM_PERIOD) + 5
    if len(sol_bars) < min_lookback:
        return {"ready": False, "reason": "insufficient_history"}

    closes = [b["c"] for b in sol_bars]
    price = closes[-1]

    # Signal 1: Momentum
    mom_ret = (price - closes[-MOMENTUM_PERIOD - 1]) / closes[-MOMENTUM_PERIOD - 1]
    vote_momentum = 1.0 if mom_ret > 0 else 0.0

    # Signal 2: VShort Momentum
    vshort_ret = (price - closes[-VSHORT_PERIOD - 1]) / closes[-VSHORT_PERIOD - 1]
    vote_vshort = 1.0 if vshort_ret > 0 else 0.0

    # Signal 3: EMA Crossover
    ema_f = _ema(closes, EMA_FAST)
    ema_s = _ema(closes, EMA_SLOW)
    ema_fast_val = ema_f[-1]
    ema_slow_val = ema_s[-1]
    vote_ema = 1.0 if ema_fast_val > ema_slow_val else 0.0

    # Signal 4: RSI
    rsi_val = _rsi(closes, RSI_PERIOD)
    vote_rsi = 1.0 if RSI_OVERSOLD < rsi_val < RSI_OVERBOUGHT else 0.0

    # Signal 5: MACD
    ema_macd_fast = _ema(closes, MACD_FAST)
    ema_macd_slow = _ema(closes, MACD_SLOW)
    macd_line = [f - s for f, s in zip(ema_macd_fast, ema_macd_slow)]
    macd_signal = _ema(macd_line, MACD_SIGNAL)
    macd_val = macd_line[-1]
    macd_sig_val = macd_signal[-1]
    vote_macd = 1.0 if macd_val > macd_sig_val else 0.0

    # Signal 6: BB Compression
    bb_upper, bb_mid, bb_lower = _bb(closes, BB_PERIOD, BB_STD)
    bb_width = (bb_upper - bb_lower) / bb_mid if bb_mid > 0 else 0
    vote_bb = 1.0 if (bb_width < BB_SQUEEZE_THRESHOLD and price > bb_mid) else 0.0
    bb_squeeze = bb_width < BB_SQUEEZE_THRESHOLD

    # BTC overlay
    btc_bonus = 0.0
    btc_mom = 0.0
    if len(btc_bars) > BTC_MOMENTUM_PERIOD + 1:
        btc_closes = [b["c"] for b in btc_bars]
        btc_mom = (btc_closes[-1] - btc_closes[-BTC_MOMENTUM_PERIOD - 1]) / btc_closes[-BTC_MOMENTUM_PERIOD - 1]
        btc_bonus = 0.5 if btc_mom > 0 else 0.0

    total_votes = vote_momentum + vote_vshort + vote_ema + vote_rsi + vote_macd + vote_bb + btc_bonus

    # ATR
    atr = _atr(sol_bars, ATR_PERIOD)

    # Position analytics
    in_pos = state.get("in_position", False)
    entry_price = state.get("entry_price", 0)
    trailing_stop = state.get("trailing_stop", 0)
    highest = state.get("highest_since_entry", 0)
    bars_held = state.get("bars_held", 0)
    entry_type = state.get("entry_type", "trend")
    unrealized = (price - entry_price) / entry_price if in_pos and entry_price > 0 else 0

    # BTC veto status
    btc_veto = btc_mom < -0.05 if btc_mom else False

    # Mean-reversion readiness
    bb_dist = (price - bb_lower) / price if price > 0 else 1.0
    mr_near_bb = bb_dist <= MR_BB_PROXIMITY
    recent_drop = 0
    if len(closes) > MR_DROP_BARS:
        recent_high = max(closes[-MR_DROP_BARS - 1:-1])
        recent_drop = (recent_high - price) / recent_high if recent_high > 0 else 0

    return {
        "ready": True,
        "price": price,
        # Individual votes
        "vote_momentum": vote_momentum,
        "vote_vshort": vote_vshort,
        "vote_ema": vote_ema,
        "vote_rsi": vote_rsi,
        "vote_macd": vote_macd,
        "vote_bb": vote_bb,
        "btc_bonus": btc_bonus,
        "total_votes": total_votes,
        # Thresholds
        "entry_threshold": ENTRY_THRESHOLD,
        "exit_threshold": EXIT_THRESHOLD,
        # Raw values
        "rsi": rsi_val,
        "rsi_oversold": RSI_OVERSOLD,
        "rsi_overbought": RSI_OVERBOUGHT,
        "atr": atr,
        "atr_pct": atr / price if price > 0 else 0,
        "bb_upper": bb_upper,
        "bb_mid": bb_mid,
        "bb_lower": bb_lower,
        "bb_width": bb_width,
        "bb_squeeze": bb_squeeze,
        "bb_squeeze_threshold": BB_SQUEEZE_THRESHOLD,
        "mom_ret": mom_ret,
        "vshort_ret": vshort_ret,
        "btc_mom": btc_mom,
        "btc_veto": btc_veto,
        "ema_fast": ema_fast_val,
        "ema_slow": ema_slow_val,
        "macd": macd_val,
        "macd_signal": macd_sig_val,
        "macd_hist": macd_val - macd_sig_val,
        # Position
        "in_position": in_pos,
        "entry_price": entry_price,
        "trailing_stop": trailing_stop,
        "highest_since_entry": highest,
        "bars_held": bars_held,
        "entry_type": entry_type,
        "unrealized_pct": unrealized,
        # Mean-reversion readiness
        "mr_rsi_ready": rsi_val <= MR_RSI_THRESHOLD,
        "mr_near_bb": mr_near_bb,
        "mr_drop_pct": recent_drop,
    }
