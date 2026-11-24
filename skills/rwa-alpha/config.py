"""
RWA Alpha v1.0 — Real World Asset Intelligence Trading Skill 配置文件
修改此文件调整策略参数，无需改动 rwa_alpha.py
"""

# ── 运行模式 ────────────────────────────────────────────────────────────
MODE              = "paper"           # "paper" / "live"
PAUSED            = True              # True=暂停（不开新仓），False=正常交易 — safe default, user must explicitly unpause
STRATEGY_MODE     = "full_alpha"       # "yield_optimizer" / "macro_trader" / "full_alpha"

# ── 资金分配 ────────────────────────────────────────────────────────────
TOTAL_BUDGET_USD  = 1000              # 总 RWA 配置 (USDC 等值)
MAX_POSITIONS     = 6                 # 最多同时持仓数
MAX_SINGLE_PCT    = 25                # 单一代币最大占比 (%)
MAX_CATEGORY_PCT  = 50                # 单一类别最大占比 (%)
BUY_AMOUNT_USD    = 100               # 单笔买入默认金额 (USDC)

# ── 链配置 ──────────────────────────────────────────────────────────────
ENABLED_CHAINS    = ["ethereum"]      # 支持: "ethereum", "solana"
CHAIN_CONFIG      = {
    "ethereum": {"chain": "ethereum", "chain_index": "1",   "stable": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"},  # USDC
    "solana":   {"chain": "solana",   "chain_index": "501", "stable": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"},  # USDC
}
GAS_RESERVE       = {"ethereum": 0.01, "solana": 0.02}  # ETH / SOL

# ── 感知层 (Perception) ────────────────────────────────────────────────
NEWS_POLL_SEC     = 120               # 新闻/宏观事件检查周期 (秒)
CHAIN_POLL_SEC    = 60                # 链上状态刷新周期 (秒)
SENTIMENT_WINDOW  = 7                 # 情绪移动平均窗口 (天)

# ── LLM 辅助分类 (Headline Classification) ────────────────────────────
LLM_ENABLED       = True              # True=启用 LLM 辅助分类, False=仅关键词
LLM_MODEL         = "claude-haiku-4-5-20251001"  # 最便宜最快的模型
LLM_CONFIDENCE_BAND = (0.55, 0.80)   # 只对这个 conviction 区间调用 LLM
                                       # >0.80 = 关键词已够明确, <0.55 = 噪音

# ── 认知层 (Cognition) ─────────────────────────────────────────────────
MIN_CONVICTION    = 0.55              # 最低信号置信度才交易 (0.0~1.0)
NAV_ZSCORE_ENTRY  = 1.5              # NAV 套利入场 z-score 阈值
YIELD_ROTATION_BPS = 50               # 收益率轮换最小差值 (bps)
MACRO_OVERRIDE    = 0.80              # 宏观事件高于此值直接覆盖其他信号

# ── 执行层 (Execution) ─────────────────────────────────────────────────
SLIPPAGE_BUY      = 1.0               # 买入滑点 (%)
SLIPPAGE_SELL     = 2.0               # 卖出滑点 (%)

# ── 风控 ───────────────────────────────────────────────────────────────
MAX_DAILY_TRADES  = 10                # 每日最大交易次数
SESSION_STOP_USD  = 50                # 累计亏损停止交易 (USDC)
COOLDOWN_LOSS_SEC = 300               # 亏损后冷却 (秒)
MAX_DRAWDOWN_PCT  = 8                 # 投资组合级止损 (%)
MIN_LIQUIDITY_USD = 200_000           # 最小池流动性 (RWA 代币通常流动性更高)
MAX_NAV_PREMIUM_BPS = 50              # 不买 NAV 溢价 >50bps 的代币

# ── 止盈止损 (资产锚定型: USDY, OUSG, PAXG, sDAI) ────────────────────
TP_NAV_PREMIUM_BPS = 40              # NAV 溢价超 40bps 止盈
SL_NAV_DISCOUNT_BPS = 100            # NAV 折价超 100bps 止损

# ── 止盈止损 (治理代币型: ONDO, CFG, MPL, PENDLE, PLUME, OM, GFI, TRU) ──
TP_GOVERNANCE_PCT = 20                # +20% 止盈
SL_GOVERNANCE_PCT = -10               # -10% 止损
TRAILING_ACTIVATE = 10                # 追踪止损: 盈利超 10% 激活
TRAILING_DROP     = 8                 # 追踪止损: 峰值回撤 8% 触发

# ── 收益率轮换 ─────────────────────────────────────────────────────────
YIELD_CHECK_SEC   = 3600              # 收益率对比检查周期 (秒)
MIN_YIELD_ADV_PCT = 0.50              # 最小 APY 优势才轮换 (%)

# ── Dashboard ──────────────────────────────────────────────────────────
DASHBOARD_PORT    = 3249

# ── RWA 代币宇宙 ──────────────────────────────────────────────────────
# category: treasury / gold / defi_yield / rwa_gov / yield_protocol / rwa_infra / rwa_credit
# asset_backed: True = NAV锚定型, False = 治理代币型
RWA_UNIVERSE = {
    # ── Tokenized Treasury ─────────────────────────────────────
    "USDY": {
        "name":         "Ondo USDY",
        "category":     "treasury",
        "asset_backed": True,
        "chains":       ["ethereum", "solana"],
        "addresses": {
            "ethereum": "0x96F6eF951840721AdBF46Ac996b59E0235CB985C",
            "solana":   "A1KLoBrKBde8Ty9qtNQUtq3C2ortoC3u7twggz7sEto6",
        },
    },
    "OUSG": {
        "name":         "Ondo OUSG",
        "category":     "treasury",
        "asset_backed": True,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0x1B19C19393e2d034D8Ff31ff34c81252FcBbee92"},
    },
    "sDAI": {
        "name":         "Savings DAI",
        "category":     "treasury",
        "asset_backed": True,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0x83F20F44975D03b1b09e64809B757c47f942BEeA"},
    },

    # ── Tokenized Gold ─────────────────────────────────────────
    "PAXG": {
        "name":         "Pax Gold",
        "category":     "gold",
        "asset_backed": True,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0x45804880De22913dAFE09f4980848ECE6EcbAf78"},
    },
    "XAUT": {
        "name":         "Tether Gold",
        "category":     "gold",
        "asset_backed": True,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0x68749665FF8D2d112Fa859AA293F07A622782F38"},
    },

    # ── DeFi Yield ─────────────────────────────────────────────
    "USDe": {
        "name":         "Ethena USDe",
        "category":     "defi_yield",
        "asset_backed": True,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0x4c9EDD5852cd905f086C759E8383e09bff1E68B3"},
    },

    # ── RWA Governance ─────────────────────────────────────────
    "ONDO": {
        "name":         "Ondo Finance",
        "category":     "rwa_gov",
        "asset_backed": False,
        "chains":       ["ethereum", "solana"],
        "addresses": {
            "ethereum": "0xfAbA6f8e4a5E8Ab82F62fe7C39859FA577269BE3",
            "solana":   "",
        },
    },
    "CFG": {
        "name":         "Centrifuge",
        "category":     "rwa_gov",
        "asset_backed": False,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0xc221b7E65FfC80DE234bbB6667aBDd46593D34F0"},
    },
    "MPL": {
        "name":         "Maple Finance",
        "category":     "rwa_gov",
        "asset_backed": False,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0x33349B282065b0284d756F0577FB39c158F935e6"},
    },

    # ── Yield Protocol ────────────────────────────────────────────
    "PENDLE": {
        "name":         "Pendle Finance",
        "category":     "yield_protocol",
        "asset_backed": False,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0x808507121b80c02388fad14726482e061b8da827"},
    },

    # ── RWA Infrastructure ───────────────────────────────────────
    "PLUME": {
        "name":         "Plume Network",
        "category":     "rwa_infra",
        "asset_backed": False,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0x4c1746a800d224393fe2470c70a35717ed4ea5f1"},
    },
    "OM": {
        "name":         "MANTRA",
        "category":     "rwa_infra",
        "asset_backed": False,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0x3593d125a4f7849a1b059e64f4517a86dd60c95d"},
    },

    # ── RWA Credit ────────────────────────────────────────────────
    "GFI": {
        "name":         "Goldfinch",
        "category":     "rwa_credit",
        "asset_backed": False,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0xdab396ccf3d84cf2d07c4454e10c8a6f5b008d2b"},
    },
    "TRU": {
        "name":         "TrueFi",
        "category":     "rwa_credit",
        "asset_backed": False,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0x4c19596f5aaff459fa38b0f7ed92f11ae6543784"},
    },

    # ── Tokenized Treasury (additional) ──────────────────────────
    "bIB01": {
        "name":         "Backed IB01 Treasury Bond 0-1yr",
        "category":     "treasury",
        "asset_backed": True,
        "chains":       ["ethereum"],
        "addresses":    {"ethereum": "0xca30c93b02514f86d5c86a6e375e3a330b435fb5"},
    },
}

CATEGORY_NAMES = {
    "treasury":       "Tokenized Treasury",
    "gold":           "Tokenized Gold",
    "defi_yield":     "DeFi Yield",
    "rwa_gov":        "RWA Governance",
    "yield_protocol": "Yield Protocol",
    "rwa_infra":      "RWA Infrastructure",
    "rwa_credit":     "RWA Credit",
}

# ── 稳定币忽略列表 ────────────────────────────────────────────────────
_IGNORE_TOKENS = {
    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",  # USDC (ETH)
    "0xdAC17F958D2ee523a2206206994597C13D831ec7",  # USDT (ETH)
    "0x6B175474E89094C44Da98b954EedeAC495271d0F",  # DAI  (ETH)
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", # USDC (SOL)
}
