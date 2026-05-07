"""
Top Rank Tokens Sniper v1.0 — Strategy Configuration
Modify this file to adjust strategy parameters without changing ranking_sniper.py

⚠️ Disclaimer:
This script and all parameter configurations are provided solely for educational
research and technical reference purposes. They do not constitute any investment advice.
Cryptocurrency trading (especially Meme coins) carries extremely high risk, including but not limited to:
  - Drastic price volatility, potentially going to zero within seconds
  - Sudden liquidity drain, unable to sell
  - Smart contract vulnerabilities, Rug Pulls, and other malicious activities
  - On-chain transactions are irreversible, cannot be undone once executed
Users should adjust all parameters according to their own risk tolerance and assume
full responsibility for any losses incurred from using this strategy.
It is recommended to test thoroughly in Paper mode first.
"""

# ── Run Mode ──────────────────────────────────────────────────────────────
MODE              = "paper"     # "paper" (recommended to test first) / "live" (Live Trading)
PAUSED            = True        # True=Paused, no new positions (safe default), False=Normal operation
TOTAL_BUDGET      = 0.5         # SOL total budget
DAILY_LOSS_LIMIT  = 0.15        # Daily Loss Limit (ratio of TOTAL_BUDGET)

# ── Session Risk Control ─────────────────────────────────────────────────
MAX_CONSEC_LOSS   = 3           # N consecutive losses → pause
PAUSE_CONSEC_SEC  = 900         # Consecutive loss pause duration (seconds, 15min)
SESSION_STOP_SOL  = 0.10        # Cumulative loss >= N SOL → stop trading

# ── Position ──────────────────────────────────────────────────────────────
# Ranking strategy characteristics: tokens already have market consensus (on the gainers leaderboard),
# relatively sufficient liquidity — suitable for medium positions with quick TP/SL.
# Per trade recommended <= 10% of total budget.
BUY_AMOUNT        = 0.05        # Per trade buy amount (SOL)
MAX_POSITIONS     = 5           # Max simultaneous positions
MAX_SINGLE_BUYS   = 1           # Max buy count for the same token
SLIPPAGE_BUY      = 5           # Buy slippage (%) — leaderboard tokens have decent liquidity, 5% is enough
SLIPPAGE_SELL     = 8           # Normal sell slippage (%) — TP / Time Stop
SLIPPAGE_SELL_URGENT = 15       # Urgent sell slippage (%) — Ranking Exit / Hard SL (liquidity may drain)
GAS_RESERVE       = 0.01        # Gas reserve (SOL)
MIN_WALLET_BAL    = 0.06        # Min wallet balance to open positions (SOL)

# ── Leaderboard Scanning ─────────────────────────────────────────────────
POLL_INTERVAL     = 10          # Polling interval (seconds)
TOP_N             = 20          # Leaderboard Top N
MIN_CHANGE_PCT    = 15          # Min price change (%) — raise threshold to avoid weak tokens
MAX_CHANGE_PCT    = 500         # Max price change (%) — tighten ceiling, overheated tokens risk pullback
MIN_LIQUIDITY     = 30_000      # Min liquidity ($) — leaderboard tokens should have sufficient liquidity
MIN_MCAP          = 50_000      # Min market cap ($) — too small market cap is easily manipulated
MAX_MCAP          = 10_000_000  # Max market cap ($) — very large market cap has limited upside
MIN_HOLDERS       = 100         # Min holders — ensure a real community exists
MIN_BUY_RATIO     = 0.55        # Min buy ratio — buy pressure should dominate
MIN_TRADERS       = 20          # Min unique traders — prevent wash trading
COOLDOWN_MIN      = 30          # Cooldown after sell (minutes) — avoid repeated entry/exit on same token
ENABLE_RANKING_EXIT = True      # Auto-exit when dropped off the leaderboard

# ── Safety Checks ─────────────────────────────────────────────────────────
# Ranking strategy faces tokens that already have some heat, but strict safety filtering is still needed.
# The thresholds below are recommended values based on common Meme coin risk patterns.
# Users can relax or tighten them as needed.
MAX_RISK_LEVEL    = 3           # Max risk level (1-5, 3=moderate risk acceptable)
BLOCK_HONEYPOT    = True        # Block honeypots (strongly recommended to keep True)
MAX_TOP10_HOLD    = 40          # Top 10 holding cap (%) — high concentration risks a dump
MAX_DEV_HOLD      = 15          # Dev holding cap (%) — high dev holding risks rug pull
MAX_BUNDLE_HOLD   = 15          # Bundler holding cap (%) — bundler control risk
MIN_LP_BURN       = 50          # LP burn floor (%) — ensure liquidity cannot be drained
MAX_DEV_RUG_COUNT = 2           # Dev rug count cap — stricter for devs with rug history
MAX_SNIPER_HOLD   = 15          # Sniper holding cap (%) — concentrated snipers create sell pressure
BLOCK_INTERNAL    = False       # Block internal tokens
MAX_SUSPICIOUS_HOLD  = 30       # Suspicious address holding cap (%)
MAX_SUSPICIOUS_COUNT = 10       # Suspicious address count cap
BLOCK_PHISHING    = True        # Block tokens with phishing addresses

# ── Take Profit ───────────────────────────────────────────────────────────
# Ranking strategy: tokens already have momentum, TP targets can be moderately aggressive,
# but first tier should recover cost quickly.
TP_TIERS = [
    (8,  0.30),   # +8%  sell 30% — quick cost recovery, cover fees
    (20, 0.35),   # +20% sell 35% — lock in profit
    (40, 0.35),   # +40% sell 35% — trend continuation reward
]

# ── Stop Loss ─────────────────────────────────────────────────────────────
# Ranking strategy: dropping off the leaderboard = momentum lost, exit quickly.
# Hard stop tightened to -15%.
STOP_LOSS_PCT     = -15         # Hard Stop Loss (%) — tighter than Signal Strategy, exit fast on momentum loss
QUICK_STOP_MIN    = 3           # Quick Stop: still losing after N minutes of holding
QUICK_STOP_PCT    = -8          # Quick Stop: loss exceeds N%
TRAILING_ACTIVATE = 10          # Trailing Stop: activates when profit exceeds N%
TRAILING_DROP     = 8           # Trailing Stop: triggers when drawdown N% from peak
MAX_HOLD_HOURS    = 2           # Time Stop: max holding hours — leaderboard heat fades fast

# ── Monitoring ────────────────────────────────────────────────────────────
MONITOR_INTERVAL  = 10          # Position check interval (seconds)
HEALTH_CHECK_SEC  = 300         # Wallet audit interval (seconds, 5min)

# ── Network ───────────────────────────────────────────────────────────────
DASHBOARD_PORT    = 3244        # Dashboard port

# ── Blacklist ─────────────────────────────────────────────────────────────
SKIP_TOKENS = [
    "11111111111111111111111111111111",                  # native SOL
    "So11111111111111111111111111111111111111112",        # WSOL
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",    # USDC
]
BLACKLIST = []
