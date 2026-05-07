"""
Smart Money Signal Copy Trade v1.0 — Strategy Configuration
Modify this file to adjust strategy parameters without changing bot.py

⚠️ Disclaimer:
This script and all parameter configurations are provided solely for educational
research and technical reference purposes. They do not constitute any investment advice.
Cryptocurrency trading (especially meme coins) carries extremely high risk, including but not limited to:
  - Smart money signals do not guarantee profits; signal delays and market reversals can happen at any time
  - Copy trading strategies essentially follow others' decisions; signal source quality cannot be guaranteed
  - On-chain transactions are irreversible; once executed they cannot be undone
  - Low market cap tokens have poor liquidity and may not sell at the expected price
Users should adjust all parameters based on their own risk tolerance and assume
full responsibility for any losses incurred from using this strategy.
It is recommended to test thoroughly in Paper Mode first.
"""

# ── Run Mode ──────────────────────────────────────────────────────────────
PAUSED                 = True                   # True=Paused (no new positions), False=Normal trading
DRY_RUN                = True                   # True=Paper (recommended to test first), False=Live

# ── Chain ────────────────────────────────────────────────────────────────────
CHAIN_ID               = 501                    # Solana

# ── Signal filters ──────────────────────────────────────────────────────────
# Smart Money strategy core: multiple smart wallets co-riding the same buy = consensus signal.
# MIN_WALLET_COUNT is the most critical filter parameter; ≥3 is statistically meaningful.
SIGNAL_LABELS          = [1, 2, 3]              # 1=SmartMoney 2=KOL 3=Whale
MIN_WALLET_COUNT       = 3                      # Min co-rider wallet count (recommend ≥3)
PAGE_SIZE              = 20                     # Signals fetched per cycle (API max=20)
MAX_SELL_RATIO         = 0.80                   # Sell ratio >80% skip (most already sold = stale signal)

# ── Token safety thresholds ─────────────────────────────────────────────────
# Smart Money strategy covers a wide market cap range, but must ensure basic liquidity and community base.
# The following thresholds are aligned with the skill.md startup protocol.
MIN_MCAP               = 200_000                # USD — Minimum $200K market cap
MIN_LIQUIDITY          = 80_000                 # USD — Minimum $80K liquidity
MIN_HOLDERS            = 300                    # Sufficient holder dispersion
MIN_LIQ_MC_RATIO       = 0.05                   # liq/MC >= 5% — Liquidity depth
MAX_TOP10_HOLDER_PCT   = 50.0                   # Top 10 holdings ≤ 50% — Prevent concentrated control
MIN_LP_BURN            = 80                     # LP burn >= 80% — Prevent pool drain
MIN_HOLDER_DENSITY     = 50                     # Min 50 holders per million MC

# ── Dev/Bundler safety ──────────────────────────────────────────────────────
# Smart Money strategy emphasizes token fundamentals safety due to longer hold times.
DEV_MAX_LAUNCHED       = 20                     # dev launched >20 tokens = token farm
DEV_MAX_RUG_RATIO      = 0.0                    # Reserved for compatibility
DEV_MAX_RUG_COUNT      = 5                      # Dev rug count absolute cap (fallback beyond rate-based logic)
DEV_MAX_HOLD_PCT       = 15.0                   # Dev holding >15% skip
BUNDLE_MAX_ATH_PCT     = 25.0                   # Bundler ATH >25% skip
BUNDLE_MAX_COUNT       = 5                      # Bundler >5 skip

# ── Position sizing (tiered by signal strength) ────────────────────────────
# Position sized by co-rider wallet count: more consensus = higher confidence = larger position.
# Users can scale proportionally based on total budget.
POSITION_TIERS         = {
    "high":  {"min_addr": 8, "sol": 0.020},     # ≥8 wallets → 0.020 SOL
    "mid":   {"min_addr": 5, "sol": 0.015},     # ≥5 wallets → 0.015 SOL
    "low":   {"min_addr": 3, "sol": 0.010},     # ≥3 wallets → 0.010 SOL
}
SLIPPAGE_PCT           = 3                      # % — Recommend 3-5% for meme coins
MAX_PRICE_IMPACT       = 5                      # %
MAX_POSITIONS          = 6                      # Max concurrent positions — Diversify risk

# ── Cost model ──────────────────────────────────────────────────────────────
FIXED_COST_SOL         = 0.001                  # priority_fee×2 + rent
COST_PER_LEG_PCT       = 1.0                    # gas + slippage + DEX fee per leg

# ── Take-profit (cost-aware, NET targets) ──────────────────────────────────
# Smart Money strategy: following consensus signals, tokens have fundamental support, can let profits run moderately.
# NET = Actual profit rate after deducting fees, ensuring every TP is truly profitable.
TP_TIERS               = [
    {"pct": 0.05, "sell": 0.30},                # +5% net → sell 30% — Recover cost first
    {"pct": 0.15, "sell": 0.40},                # +15% net → sell 40% — Lock in profits
    {"pct": 0.30, "sell": 1.00},                # +30% net → sell remaining — Close position
]
TRAIL_ACTIVATE         = 0.12                   # activate trailing after +12%
TRAIL_DISTANCE         = 0.10                   # exit if price drops 10% from peak

# ── Stop-loss ──────────────────────────────────────────────────────────────
# Smart Money strategy holds positions longer; stop loss can be moderately relaxed to allow profit room to develop.
SL_MULTIPLIER          = 0.90                   # -10% hard stop
LIQ_EMERGENCY          = 5_000                  # emergency exit if liquidity < $5K

# ── Time-decay SL ──────────────────────────────────────────────────────────
# The longer a position is held without profit, the more likely the signal has expired; progressively tighten stop loss.
TIME_DECAY_SL          = [
    {"after_min": 60, "sl_pct": -0.05},         # After 60min, SL tightens to -5%
    {"after_min": 30, "sl_pct": -0.08},         # After 30min, SL tightens to -8%
    {"after_min": 15, "sl_pct": -0.10},         # After 15min, keep -10%
]

# ── Trend time stop ────────────────────────────────────────────────────────
TIME_STOP_MIN_HOLD_MIN = 30                     # Do not trigger before 30min
TIME_STOP_CANDLE_BAR   = "15m"                  # Candle timeframe
TIME_STOP_REVERSAL_VOL = 0.8                    # Trend reversal confirmation
TIME_STOP_MAX_HOLD_HRS = 4                      # Hard max hold time

# ── Session risk management ────────────────────────────────────────────────
MAX_CONSEC_LOSS        = 3                      # 3 consecutive losses → Pause
PAUSE_CONSEC_SEC       = 600                    # Pause for 10 minutes
SESSION_LOSS_LIMIT_SOL = 0.05                   # Cumulative loss 0.05 SOL → Pause 30min
SESSION_LOSS_PAUSE_SEC = 1800                   # 30 minutes
SESSION_STOP_SOL       = 0.10                   # Cumulative loss 0.10 SOL → Stop trading

# ── Entry safety ───────────────────────────────────────────────────────────
MAX_K1_PCT_ENTRY       = 15.0                   # 1m K1 >15% = Chasing pump
SAFE_PLATFORMS         = {"pump", "bonk"}       # Low market cap platform whitelist
PLATFORM_MCAP_THRESH   = 2_000_000              # Enable platform filter below $2M

# ── Timing ─────────────────────────────────────────────────────────────────
POLL_INTERVAL_SEC      = 20                     # Signal polling interval
API_DELAY_SEC          = 1.5                    # onchainos call interval
ORDER_TIMEOUT_SEC      = 120                    # Trade confirmation timeout

# ── Safety ─────────────────────────────────────────────────────────────────
SOL_GAS_RESERVE        = 0.05                   # SOL reserved for fees
MAX_SWAP_FAILS         = 3                      # Consecutive swap failure count
MIN_POSITION_VALUE_USD = 0.10                   # Dust cleanup threshold

# ── Dashboard ──────────────────────────────────────────────────────────────
DASHBOARD_PORT         = 3248
