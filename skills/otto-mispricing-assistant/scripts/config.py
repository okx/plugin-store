"""
Otto Mispricing Assistant v0.1 — Hot-reloadable configuration.

Every parameter below is tunable without changing bot.py or the SKILL protocol.
The AI agent reads these values when ranking candidates and when sizing a trade.

⚠️  Disclaimer: values are reference defaults sized for a generalist user on
Polymarket. They are NOT investment advice and may not fit your risk tolerance
or jurisdiction. Adjust before going live.
"""

# ── Run mode ─────────────────────────────────────────────────────────────────
DRY_RUN = True             # True = no live orders, even with --confirm
PAUSED  = False            # True = refuse even dry-run scans

# ── Trade sizing ─────────────────────────────────────────────────────────────
DEFAULT_TRADE_SIZE_USD = 10     # per-trade default in USDC.e
MIN_TRADE_SIZE_USD     = 2
MAX_TRADE_SIZE_USD     = 50     # hard cap per single trade
MAX_SESSION_BUDGET_USD = 200    # cumulative cap per session

# ── Edge + filter thresholds ─────────────────────────────────────────────────
# Edge gate stays tight (mispricing is the whole point); only loosened the
# signal-confidence floor in score_candidate to accommodate news-flash flashes
# whose severity-derived confidence sits at 0.6-0.8.
MIN_EDGE_PCT           = 0.05   # min |otto_estimate - implied_prob| to surface
RESOLUTION_WINDOW_DAYS = 14     # skip markets resolving later than this
MIN_LIQUIDITY_USD      = 5_000  # on-book liquidity floor
MIN_VOLUME_USD         = 1_000  # 24h volume floor
TOP_N_CANDIDATES       = 5      # max candidates presented to user

# ── Signal windows ───────────────────────────────────────────────────────────
WINDOW_MIN_NEWS       = 360     # 6h lookback for news-flash relevance
WINDOW_HOURS_KOL      = 24      # rolling KOL sentiment window
PRICE_STALENESS_SEC   = 60      # re-quote if older than this at Step 7

# ── Signal feed ──────────────────────────────────────────────────────────────
# Calibrated against live signals.useotto.xyz cadence 2026-04-30:
# - News-flash producer: top_ten_analysis pipeline runs every 6h (top-of-window).
# - KOL pipeline: hourly. Funding pipeline: not yet shipped.
# - 75 min covers the typical refresh latency without firing on stale data.
SIGNAL_FEED_BASE    = "https://signals.useotto.xyz"
SIGNAL_FEED_TIMEOUT = 4
SIGNAL_FEED_RETRIES = 1
MAX_SIGNAL_AGE_SEC  = 4500      # 75 min — covers full hourly KOL + 6h news cycles

# ── Polymarket categories to scan by default ─────────────────────────────────
DEFAULT_CATEGORIES = ["crypto", "macro", "elections"]

# ── Logging ──────────────────────────────────────────────────────────────────
LOG_SCANS_TO_FILE = True
LOG_FILE          = "otto_mispricing_scans.jsonl"
