"""
Otto Alpha Sniper v0.1 — Hot-reloadable configuration.

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
DEFAULT_SIZE_USD        = 25      # per-trade notional in USD; agent may override per call
MIN_SIZE_USD            = 10
MAX_SIZE_USD            = 500
MAX_POSITION_PCT_EQUITY = 0.10    # no single trade can exceed 10% of HL equity

# ── Leverage caps per mode ───────────────────────────────────────────────────
# Protocol max is per-coin (e.g. BTC 50x, memecoins 3x). Always min() these
# against the per-coin cap before submitting.
MAX_LEVERAGE_TRENDING = 5
MAX_LEVERAGE_KOL      = 3
MAX_LEVERAGE_FUNDING  = 10
MAX_LEVERAGE_ABSOLUTE = 20

# ── Risk controls ────────────────────────────────────────────────────────────
SL_PCT                  = 0.02    # 2% stop-loss
TP_PCT                  = 0.04    # 4% take-profit (2:1 RR)
SESSION_MAX_DRAWDOWN_PCT = 0.15   # halt all new trades after cumulative -15%
MAX_CONCURRENT_POSITIONS = 3

# ── Signal thresholds ────────────────────────────────────────────────────────
# Calibrated against live signals.useotto.xyz data 2026-04-30:
# - trending: score = token-alpha confidence/100. Real values 0.4-0.85; 0.50
#   floor keeps quality high while catching mid-cap trending picks.
# - kol: sentiment_score 25-90 → confidence 0.0-0.80; 0.50 = score ≥75 or ≤25
# - kol_count: typical 30-40 distinct authors per 24h window (list size 50)
# - funding: producer not yet shipped (Tier 2 deferred)
MIN_SCORE                = 0.50   # trending mode floor
MIN_CONFIDENCE_KOL       = 0.50   # genuine consensus gate
MIN_KOL_COUNT            = 25     # cohort sample-size floor
FUNDING_EXTREME_ABS      = 0.0008 # 8h funding skew threshold (Mode 3 / Tier 2)

# ── Market filters ───────────────────────────────────────────────────────────
MIN_VOLUME_USD           = 10_000_000    # daily HL volume floor — avoid illiquid coins
ASSET_BLOCKLIST          = []            # coins to never trade

# ── Signal feed ──────────────────────────────────────────────────────────────
SIGNAL_FEED_BASE = "https://signals.useotto.xyz"
SIGNAL_FEED_TIMEOUT_SEC = 4
SIGNAL_FEED_RETRIES = 1

# ── Logging ──────────────────────────────────────────────────────────────────
LOG_TRADES_TO_FILE = True
LOG_FILE = "otto_sniper_trades.jsonl"
