"""
Meme Trench Scanner v1.0 — Strategy Configuration
Modify this file to adjust strategy parameters without changing scan_live.py

⚠️ Disclaimer:
This script and all parameter configurations are for educational research and
technical reference only, and do not constitute any investment advice.
Meme Trench Scanner targets newly launched small-cap Meme tokens, which carry
extremely high risk, including but not limited to:
  - Tokens may go to zero within minutes of launch (Rug Pull, Dev Dump)
  - Extremely low liquidity; you may be unable to sell after buying (Honeypot, LP removal)
  - Fees and slippage from high-frequency trading may erode most profits
  - Smart contracts are unaudited and may contain unforeseen vulnerabilities
Users should adjust all parameters according to their own risk tolerance and bear
full responsibility for any losses resulting from the use of this strategy.
It is recommended to test thoroughly using Paper Mode first.
"""

# ── Operating Mode ─────────────────────────────────────────────────────
PAUSED         = True     # True=manually paused (no new positions, monitoring continues), False=normal trading
PAPER_TRADE    = True     # True=Paper Trading (recommended to test first), False=Live Trading

# ── Position ───────────────────────────────────────────────────────────
SOL_PER_TRADE  = {"SCALP": 0.25, "MINIMUM": 0.15, "STRONG": 0.25}
MAX_SOL        = 1.00     # Max total exposure (SOL)
MAX_POSITIONS  = 7        # Max concurrent positions
SLIPPAGE_BUY   = {"SCALP": 8, "MINIMUM": 10, "STRONG": 10}   # Buy slippage (integer percent, 8=8%)
SLIPPAGE_SELL  = 50       # Fixed high sell slippage (low liquidity small-cap tokens)
SOL_GAS        = 0.05     # Reserved for fees
COST_PER_LEG   = 0.003    # OKX DEX 0.3% per leg
MAX_TRADES     = 50       # Auto-stop (0=unlimited)

# ── Take Profit ────────────────────────────────────────────────────────
TP1_PCT        = 0.15     # +15% first take profit
TP1_SELL       = {"SCALP": 0.60, "hot": 0.50, "quiet": 0.40}  # TP1 partial sell ratio
TP2_PCT        = 0.25     # +25% second take profit
TP2_SELL       = {"SCALP": 1.00, "hot": 1.00, "quiet": 1.00}  # TP2 full exit

# ── Stop Loss ──────────────────────────────────────────────────────────
S1_PCT         = {"SCALP": -0.15, "hot": -0.20, "quiet": -0.20}
HE1_PCT        = -0.50    # -50% emergency exit
TRAILING_DROP  = 0.05     # 5% drawdown after TP1 → full exit
FAST_DUMP_PCT  = -0.15    # -15% within 10s → instant exit
FAST_DUMP_SEC  = 10       # Fast dump detection window (seconds)

# ── Time Stop ──────────────────────────────────────────────────────────
S3_MIN         = {"SCALP": 5, "hot": 8, "quiet": 15}  # minutes
MAX_HOLD_MIN   = 30       # Max position hold time (minutes)

# ── Session Risk Control ───────────────────────────────────────────────
MAX_CONSEC_LOSS  = 2      # N consecutive losses → pause
PAUSE_CONSEC_SEC = 900    # Consecutive loss pause duration (seconds, 15min)
PAUSE_LOSS_SOL   = 0.30   # Cumulative loss >= N SOL → pause 30min
STOP_LOSS_SOL    = 0.50   # Cumulative loss >= N SOL → stop trading

# ── Scanning ───────────────────────────────────────────────────────────
LOOP_SEC       = 10       # Scan interval (seconds)
MONITOR_SEC    = 1        # Position monitor interval (seconds)
CHAIN_INDEX    = "501"    # Solana
SOL_ADDR       = "11111111111111111111111111111111"
DASHBOARD_PORT = 3241

# ── Basic Filters ──────────────────────────────────────────────────────
AGE_HARD_MIN   = 240      # Min token age (seconds, 4min)
AGE_SOFT_MIN   = 300      # Early window threshold (seconds, 5min)
AGE_MAX        = 86_400   # Max token age (seconds, 24h)
MC_CAP         = 800_000  # MC upper limit ($)
MC_MIN         = 50_000   # MC lower limit ($)
LIQ_MIN        = 10_000   # Liquidity lower limit ($)
BS_MIN         = 1.0      # 1h B/S ratio pre-filter
DUMP_FLOOR     = -40      # Max single candle drop (%)

# ── Signal Thresholds ─────────────────────────────────────────────────
SIG_A_THRESHOLD     = 1.25   # TX acceleration ratio threshold
MIN_CONFIDENCE      = 25     # Minimum confidence
SIG_A_FLOOR_TXS_MIN = 45    # TX acceleration floor (txs/min)
HOT_MODE_RATIO      = 0.40  # Hot Mode trigger ratio

# ── Safety Detection ──────────────────────────────────────────────────
VOLMC_MIN_RATIO    = 0.02    # Vol/MC minimum ratio
TF_MIN_VOLUME      = 5_000   # 1h minimum volume ($)
TF_MAX_BUNDLERS    = 15      # Bundler holdings upper limit (%)
MIN_HOLDERS        = 50      # Minimum holders count
DEV_SELL_DROP_PCT  = 60      # Dev dump detection: ATH drawdown %
DEV_SELL_VOL_MULT  = 10      # Dev dump detection: volume multiplier
BUNDLE_ATH_PCT_MAX = 25      # Bundler ATH percentage upper limit (%)
RUG_RATE_MAX       = 0.50    # Dev rug rate upper limit
MAX_DEV_RUG_COUNT  = 5       # Dev rug count absolute upper limit (fallback beyond rate-based logic)
DEV_HOLD_DEEP_MAX  = 0.10    # Dev deep holdings upper limit (decimal, 0.10=10%)
DEV_MAX_LAUNCHED   = 800     # Dev historical token launch count upper limit
BUNDLE_MAX_COUNT   = 30      # Bundler wallet count upper limit

# ── Token List Filters ─────────────────────────────────────────────────
TOP10_HOLD_MAX     = 40      # Top 10 holdings upper limit (%)
INSIDERS_MAX       = 15      # Insider upper limit (%)
SNIPERS_MAX        = 20      # Sniper upper limit (%)
FRESH_WALLET_MAX   = 40      # Fresh wallet upper limit (%)
BOT_TRADERS_MAX    = 100
APED_WALLET_MAX    = 10
WASH_PRICE_CHG_MIN = 0.01    # Wash trading detection: min price change
BOND_NEAR_PCT      = 0.80    # Near migration threshold

# ── LP Lock ────────────────────────────────────────────────────────────
LP_LOCK_MIN_PCT    = 0.80
LP_LOCK_MIN_HOURS  = 0
LP_LOCK_STRICT     = False

# ── Protocol Support (11 Solana Launchpads) ───────────────────────────
PROTOCOL_PUMPFUN        = "120596"
PROTOCOL_LETSBONK       = "136266"
PROTOCOL_BELIEVE        = "134788"
PROTOCOL_BONKERS        = "139661"
PROTOCOL_JUPSTUDIO      = "137346"
PROTOCOL_BAGS           = "129813"
PROTOCOL_MOONSHOT_MONEY = "133933"
PROTOCOL_LAUNCHLAB      = "136137"
PROTOCOL_MOONSHOT       = "121201"
PROTOCOL_METEORADBC     = "136460"
PROTOCOL_MAYHEM         = "139048"
DISCOVERY_PROTOCOLS = [
    PROTOCOL_PUMPFUN, PROTOCOL_LETSBONK, PROTOCOL_BELIEVE,
    PROTOCOL_BONKERS, PROTOCOL_JUPSTUDIO, PROTOCOL_BAGS,
    PROTOCOL_MOONSHOT_MONEY, PROTOCOL_LAUNCHLAB, PROTOCOL_MOONSHOT,
    PROTOCOL_METEORADBC, PROTOCOL_MAYHEM,
]

# ── NEW Stage Discovery ───────────────────────────────────────────────
MC_MIN_NEW  = 50_000
MC_MAX_NEW  = 800_000
AGE_MAX_NEW = 86_400

# ── Pullback Watchlist ─────────────────────────────────────────────────
WATCHLIST_TIMEOUT_SEC   = 180    # 3 min
WATCHLIST_DUMP_DROP     = 0.15   # 15% drop = dump
WATCHLIST_PULLBACK_DROP = 0.05   # 5% drop = pullback
WATCHLIST_BS_MIN        = 1.5    # B/S ratio secondary confirmation

# ── Trade Blacklist ────────────────────────────────────────────────────
_WSOL_MINT_STR = "So11111111111111111111111111111111111111112"
_IGNORE_MINTS = {_WSOL_MINT_STR, "7JzLK1eq9MEq9mPNGMSr2PUoF2CCUG8corxKUbgxvJ3V"}
_NEVER_TRADE_MINTS = _IGNORE_MINTS | {
    "11111111111111111111111111111111",              # native SOL
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", # USDC
    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", # USDT
    "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So",  # mSOL
    "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj", # stSOL
    "bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1",  # bSOL
    "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn", # JitoSOL
}
