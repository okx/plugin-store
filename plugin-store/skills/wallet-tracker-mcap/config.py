"""
钱包跟单策略 v1.0 -- Wallet Copy-Trade Bot 配置文件
修改此文件调整策略参数，无需改动 wallet_tracker.py
"""

# ── 运行模式 ────────────────────────────────────────────────────────────
MODE              = "paper"       # "paper" / "live"
PAUSED            = True          # True=暂停（不开新仓），False=正常交易

# ── 目标钱包 ────────────────────────────────────────────────────────────
TARGET_WALLETS    = []            # 要跟踪的 Solana 钱包地址列表
                                  # 例: ["Abc123...", "Def456..."]

# ── 跟单模式 ────────────────────────────────────────────────────────────
FOLLOW_MODE       = "mc_target"   # "mc_target" / "instant"
MC_TARGET_USD     = 8_888         # MC_TARGET: 代币总市值门槛 ($) -- 低于此值不跟买
MC_GROWTH_PCT     = 0             # MC_TARGET: 钱包买入后代币总市值需涨 N% 才跟买 (0=不等涨幅)
MC_MAX_USD        = 50_000_000    # 市值上限 -- 超过此值不跟买 ($)

# ── 卖出跟踪 ────────────────────────────────────────────────────────────
MIRROR_SELL       = True          # 目标钱包卖出时是否同步卖出
MIRROR_SELL_PCT   = 1.00          # 跟卖比例 (1.00=全卖, 0.50=卖一半)

# ── 仓位 ────────────────────────────────────────────────────────────────
BUY_AMOUNT        = 0.03          # 单笔买入 (SOL)
MAX_POSITIONS     = 5             # 最多同时持仓数
TOTAL_BUDGET      = 0.50          # SOL 总预算
SLIPPAGE_BUY      = 5             # 买入滑点 (%)
SLIPPAGE_SELL     = 15            # 卖出滑点 (%)
GAS_RESERVE       = 0.01          # 保留 gas (SOL)
MIN_WALLET_BAL    = 0.05          # 最低钱包余额才开仓 (SOL)
SOL_ADDR          = "11111111111111111111111111111111"
CHAIN             = "solana"
CHAIN_INDEX       = "501"

# ── 安全过滤（跟单仍需安全检查，不能盲跟）──────────────────────────────
MIN_LIQUIDITY     = 10_000        # 最小流动性 ($)
MIN_HOLDERS       = 30            # 最少持有者
MAX_TOP10_HOLD    = 60            # Top10 持仓上限 (%)
MAX_DEV_HOLD      = 30            # Dev 持仓上限 (%)
MAX_BUNDLE_HOLD   = 20            # Bundler 持仓上限 (%)
MAX_DEV_RUG_COUNT = 3             # Dev rug 次数上限
BLOCK_HONEYPOT    = True          # 拦截蜜罐
RISK_CHECK_GATE   = 3             # risk_check severity >= 此值则拒绝 (G3/G4 block)

# ── 止盈（梯度）────────────────────────────────────────────────────────
TP_TIERS = [
    (15, 0.30),   # +15% 卖 30%
    (30, 0.40),   # +30% 卖 40%
    (50, 1.00),   # +50% 卖剩余全部
]

# ── 止损 ────────────────────────────────────────────────────────────────
STOP_LOSS_PCT     = -20           # 硬止损 (%)
TRAILING_ACTIVATE = 10            # 追踪止损: 盈利超过 N% 激活
TRAILING_DROP     = 15            # 追踪止损: 从峰值回撤 N% 触发
MAX_HOLD_HOURS    = 6             # 时间止损: 最大持仓小时数

# ── Session 风控 ────────────────────────────────────────────────────────
MAX_CONSEC_LOSS   = 3             # 连续亏损 N 次 → 暂停
PAUSE_CONSEC_SEC  = 600           # 暂停时长 (秒)
SESSION_STOP_SOL  = 0.10          # 累计亏损 → 停止交易

# ── 轮询 ────────────────────────────────────────────────────────────────
POLL_INTERVAL     = 30            # 钱包监控轮询周期 (秒)
MONITOR_INTERVAL  = 15            # 持仓 + MC 检查周期 (秒)
HEALTH_CHECK_SEC  = 300           # 钱包审计周期 (秒)
ZERO_CONFIRM_COUNT = 3            # 连续 N 次 balance=0 才认为已卖

# ── Dashboard ──────────────────────────────────────────────────────────
DASHBOARD_PORT    = 3248

# ── 交易黑名单 ──────────────────────────────────────────────────────────
_IGNORE_MINTS = {
    "11111111111111111111111111111111",               # native SOL
    "So11111111111111111111111111111111111111112",     # wSOL
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", # USDC
    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", # USDT
    "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So",  # mSOL
    "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj", # stSOL
    "bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1",  # bSOL
    "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn", # JitoSOL
}
