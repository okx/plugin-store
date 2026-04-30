"""
Otto KOL Follow v0.1 — Hot-reloadable configuration.

Every parameter below is tunable without changing bot.py or the SKILL protocol.
The AI agent reads these values through bot.py or via grep when orchestrating
a trade in reactive mode.

⚠️  Disclaimer: values are reference defaults sized for a generalist user on
Hyperliquid. They are NOT investment advice and may not fit your risk tolerance,
experience, or jurisdiction. Adjust before going live.
"""

# ── Run mode ─────────────────────────────────────────────────────────────────
DRY_RUN = True             # True = paper mode (no --confirm passed to hyperliquid-plugin)
PAUSED  = False            # True = refuse all new trades regardless of DRY_RUN

# ── Sizing ───────────────────────────────────────────────────────────────────
# DEFAULT_SIZE_USD is the position NOTIONAL value (size × mark), NOT margin.
# Margin used = notional / leverage. With $25 notional at 3x leverage, the
# trade locks ~$8.33 of margin.
DEFAULT_SIZE_USD        = 25      # per-trade notional in USD
MIN_SIZE_USD            = 10
MAX_SIZE_USD            = 500
MAX_POSITION_PCT_EQUITY = 0.10    # no single trade can exceed 10% of HL equity

# ── Leverage caps ────────────────────────────────────────────────────────────
# KOL consensus is a lagging, noise-prone signal. Cap leverage conservatively.
MAX_LEVERAGE_KOL      = 3
MAX_LEVERAGE_ABSOLUTE = 10

# ── Risk controls ────────────────────────────────────────────────────────────
SL_PCT                   = 0.02   # 2% stop-loss
TP_PCT                   = 0.04   # 4% take-profit (2:1 RR)
SESSION_MAX_DRAWDOWN_PCT = 0.15   # halt all new trades after cumulative -15%
MAX_CONCURRENT_POSITIONS = 3
COOLDOWN_HOURS           = 6      # min hours between fires on the SAME coin (KOL is slow)

# ── KOL signal thresholds ────────────────────────────────────────────────────
# Calibrated against live signals.useotto.xyz data 2026-04-30:
# - Daily distinct-author count typically 30-40 (our Twitter list size is 50)
# - sentiment_score range 25-90; confidence = abs(score - 50) / 50, so 0.50
#   corresponds to score >= 75 OR score <= 25 (genuine consensus)
# - KOL pipeline runs hourly; signal age can reach 55-60 min mid-cycle
MIN_KOL_COUNT         = 25    # min cohort sample size for a valid consensus read
MIN_CONFIDENCE_KOL    = 0.50  # gate at score >= 75 or <= 25 (real consensus)
WINDOW_HOURS_DEFAULT  = 24    # rolling sentiment window

# ── Market filters ───────────────────────────────────────────────────────────
MIN_VOLUME_USD        = 10_000_000    # daily HL volume floor
ASSET_BLOCKLIST       = []            # coins to never trade

# ── Signal feed ──────────────────────────────────────────────────────────────
SIGNAL_FEED_BASE    = "https://signals.useotto.xyz"
SIGNAL_FEED_TIMEOUT = 4
SIGNAL_FEED_RETRIES = 1
MAX_SIGNAL_AGE_SEC  = 4500    # 75 min — covers full hourly producer cycle + buffer

# ── Logging ──────────────────────────────────────────────────────────────────
LOG_TRADES_TO_FILE = True
LOG_FILE           = "otto_kol_follow_trades.jsonl"
