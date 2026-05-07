"""
Market Structure Analyzer v3.0 — Configuration
Read-only analytics skill. No trading, no wallet access.
Primary: OKX CeFi CLI + OnchainOS CLI.  Secondary: Direct HTTP for options + macro.
"""

# ── Output ─────────────────────────────────────────────────────────────
OUTPUT_FORMAT = "json"               # "json" / "text"
DEFAULT_TOKENS = ["BTC"]             # Default tokens when none specified

# ── Data Sources (HTTP — used for options chain + external macro) ─────
OKX_BASE = "https://www.okx.com/api/v5"   # options chain only (gamma wall, skew)
COINMETRICS_BASE = "https://community-api.coinmetrics.io/v4"
COINGECKO_BASE = "https://api.coingecko.com/api/v3"
ALTERNATIVE_BASE = "https://api.alternative.me"
DEFILLAMA_BASE = "https://stablecoins.llama.fi"

# ── Rate Limits ────────────────────────────────────────────────────────
REQUEST_TIMEOUT = 10                 # seconds per HTTP request
MAX_RETRIES = 2                      # retry on transient failures

# ── Dashboard ─────────────────────────────────────────────────────────
DASHBOARD_PORT = 8420
STRUCTURE_POLL_SEC = 60              # structure indicator refresh
CANDLE_POLL_SEC = 30                 # candle + TA refresh
CANDLE_LIMIT = 300                   # max candles to fetch
SUPPORTED_BARS = ["5m", "15m", "1H", "4H", "1D"]

# ── TA Indicator Parameters ───────────────────────────────────────────
RSI_PERIOD = 14
BB_PERIOD = 20
BB_STD = 2.0
MACD_FAST = 12
MACD_SLOW = 26
MACD_SIGNAL = 9

# ── Risk Disclaimer ───────────────────────────────────────────────────
# This skill is READ-ONLY analytics. It does NOT execute trades,
# access wallets, or manage funds. All data is from public APIs.
