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
MIN_SCORE                = 0.65   # trending mode: skip below this signal strength
MIN_CONFIDENCE_KOL       = 0.70
MIN_KOL_COUNT            = 40     # min KOLs aggregated for valid KOL signal
FUNDING_EXTREME_ABS      = 0.0008 # 8h funding skew threshold for funding-fade mode

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
