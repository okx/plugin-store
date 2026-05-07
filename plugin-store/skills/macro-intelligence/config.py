"""
Macro Intelligence Skill v1.0 — Configuration
Merges perception layers from RWA Alpha + TG Intel into a unified intelligence feed.
Edit this file to configure sources, filters, classification, and output.

DISCLAIMER: This skill is for educational and informational purposes only.
It provides macro intelligence signals — no trading logic is included.
Review all parameters before connecting to downstream trading skills.
"""
import os

# ═══════════════════════════════════════════════════════════════════════
#  6551.io OpenNews (WebSocket + REST fallback)
# ═══════════════════════════════════════════════════════════════════════
OPENNEWS_ENABLED = True
OPENNEWS_TOKEN = os.environ.get("OPENNEWS_TOKEN", "")
OPENNEWS_WSS_URL = "wss://ai.6551.io/open/news_wss"
OPENNEWS_API_BASE = "https://ai.6551.io"
OPENNEWS_MIN_SCORE = 40          # Only process articles with AI score >= 40
OPENNEWS_ENGINE_TYPES = ["news"]  # "news", "listing", "onchain", "meme", "market", "prediction"
OPENNEWS_POLL_SEC = 120          # REST fallback interval (if WebSocket disconnects)

# ═══════════════════════════════════════════════════════════════════════
#  Finnhub Market News
# ═══════════════════════════════════════════════════════════════════════
FINNHUB_ENABLED = True
FINNHUB_API_KEY = os.environ.get("FINNHUB_API_KEY", "")
FINNHUB_BASE = "https://finnhub.io/api/v1"
FINNHUB_POLL_SEC = 180           # Every 3 min (60 req/min limit, be conservative)
FINNHUB_CATEGORIES = ["general", "crypto"]  # "general", "forex", "crypto", "merger"
FINNHUB_PRICE_SYMBOLS = {
    "SPY": "SPY",
    "GLD": "GOLD",
    "SLV": "SILVER",
}
PRICE_TICKER_POLL_SEC = 60       # Refresh prices every 60s

# ═══════════════════════════════════════════════════════════════════════
#  FRED Macro Indicators
# ═══════════════════════════════════════════════════════════════════════
FRED_ENABLED = True
FRED_API_KEY = os.environ.get("FRED_API_KEY", "")
FRED_BASE = "https://api.stlouisfed.org/fred"
FRED_POLL_SEC = 3600             # Every 1 hour (data updates daily/monthly)
FRED_SERIES = {
    "FEDFUNDS":  "Fed Funds Rate",
    "CPIAUCSL":  "CPI (Inflation)",
    "GDP":       "Real GDP",
    "UNRATE":    "Unemployment Rate",
    "T10Y2Y":    "10Y-2Y Treasury Spread",
    "DGS10":     "10-Year Treasury Yield",
}

# ═══════════════════════════════════════════════════════════════════════
#  TELETHON AUTH (optional — skill runs without it)
#  Get credentials at https://my.telegram.org/apps
# ═══════════════════════════════════════════════════════════════════════
TELETHON_API_ID    = 0              # Integer from my.telegram.org
TELETHON_API_HASH  = ""             # String from my.telegram.org
SESSION_NAME       = "macro_news"   # Session file name

# Or set env vars: TG_API_ID, TG_API_HASH

# ═══════════════════════════════════════════════════════════════════════
#  TELEGRAM GROUPS TO MONITOR
#  Categories determine how signals are tagged in `affects`:
#    macro    → Fed/CPI/rates/gold       → affects: rwa, perps
#    whale    → Whale alerts, smart money → affects: spot_long, wallet_tracker
#    alpha    → Token calls, CT alpha     → affects: spot_long, meme
#    rwa      → RWA-specific (Ondo, etc.) → affects: rwa, perps
#    meme     → Meme / degen plays        → affects: meme
#    general  → Mixed crypto discussion   → affects: all
# ═══════════════════════════════════════════════════════════════════════
GROUPS = {
    "macro": [
        # "@MacroAlphaGroup",
        # -1001234567890,
    ],
    "whale": [],
    "alpha": [],
    "rwa": [],
    "meme": [],
    "general": [],
}

CHANNELS = {
    "macro": [
        "@WatcherGuru",
        "@MacroScope",
        "@zabordev",
    ],
    "crypto_news": [
        "@CoinDesk",
        "@TheBlock__",
    ],
    "whale": [
        "@whale_alert_io",
        "@lookonchain",
    ],
    "rwa": [
        "@OndoFinance",
    ],
}

# ═══════════════════════════════════════════════════════════════════════
#  NEWSNOW HTTP SOURCES
# ═══════════════════════════════════════════════════════════════════════
NEWSNOW_BASE       = "https://newsnow.busiyi.world/api/s"
NEWS_SOURCES       = ["wallstreetcn", "cls", "jin10"]
NEWS_POLL_SEC      = 120           # Poll interval (seconds)
POLYMARKET_POLL_SEC = 120          # Polymarket poll interval
POLYMARKET_BASE    = "https://gamma-api.polymarket.com/markets"
POLYMARKET_QUERIES = ["fed rate", "cpi inflation", "gold price"]

# ═══════════════════════════════════════════════════════════════════════
#  NOISE FILTER (Telegram only — kills ~85% of group messages)
# ═══════════════════════════════════════════════════════════════════════
NOISE_MIN_LENGTH       = 30
NOISE_MAX_EMOJI_RATIO  = 0.5
NOISE_SKIP_PATTERNS    = [
    # Greetings / reactions
    r"^(gm|gn|gm!|gn!|good morning|good night|wen|wagmi|ngmi|lol|lmao|haha|nice|based|ser|fren)\s*$",
    r"^(早上好|晚安|早|哈哈|牛|不错)\s*$",
    # Short reactions
    r"^\+1$", r"^(yes|no|ok|yep|nope|yea|nah)\s*$",
    r"^(this|this is the way|100%|fr|real|facts|true)\s*$",
    # Spam patterns
    r"(?i)(airdrop|free mint|whitelist|join now|click here|t\.me/\S+bot)",
    r"(?i)(DM me|send me|telegram\.me)",
]
NOISE_SKIP_BOT_FORWARDS = True
NOISE_SKIP_DEEP_REPLIES = True

# VIP senders — always bypass noise filter (username or user_id)
VIP_SENDERS = []

# ═══════════════════════════════════════════════════════════════════════
#  LLM CLASSIFICATION (Layer 2 & 3)
# ═══════════════════════════════════════════════════════════════════════
LLM_ENABLED        = True
LLM_MODEL          = "claude-haiku-4-5-20251001"
LLM_MAX_TOKENS     = 100
LLM_TIMEOUT_SEC    = 5
LLM_CONFIDENCE_BAND = (0.55, 0.80)   # Only call LLM in this conviction range
LLM_INSIGHT_ENABLED = True            # Generate AI insight for classified signals
LLM_INSIGHT_TIMEOUT_SEC = 8           # Slightly longer for richer output
LLM_INSIGHT_MAX_TOKENS = 250

# Pre-screen keywords — only messages matching these go to LLM (saves cost)
LLM_PRESCREEN_KEYWORDS = [
    # English
    "buy", "sell", "long", "short", "entry", "target", "stop",
    "bullish", "bearish", "breakout", "dump", "pump", "accumulate",
    "whale", "liquidat", "billion", "million", "fed", "cpi", "rate",
    "etf", "sec", "blackrock", "approval", "ondo", "paxg", "rwa",
    "funding rate", "open interest", "leverage", "margin",
    "chart", "support", "resistance", "volume", "divergence",
    "gold", "treasury", "yield", "inflation", "tariff", "trade war",
    "gdp", "employment", "payroll", "fomc", "ecb", "boj",
    # Chinese
    "买", "卖", "做多", "做空", "入场", "止盈", "止损",
    "牛", "熊", "突破", "暴跌", "暴涨", "鲸鱼", "清算",
    "降息", "加息", "通胀", "黄金", "监管", "利好", "利空",
    "关税", "贸易战", "就业", "非农",
    # Ticker pattern
    r"\$[A-Z]{2,10}",
]

# ═══════════════════════════════════════════════════════════════════════
#  MACRO KEYWORDS — Layer 1 regex classification (EN + CN)
#  Merged from RWA Alpha (13 types) + TG Intel (11 rules) + extended
# ═══════════════════════════════════════════════════════════════════════
MACRO_KEYWORDS = {
    # ── Fed / Rates ──
    # NOTE: More specific patterns MUST come before general ones (surprise before generic cut)
    "fed_cut_surprise":       [r"(?i)surprise\s+cut", r"(?i)emergency\s+cut", r"(?i)(fed|rate).*surprise", r"(?i)surprise.*(fed|rate|cut|bps)", r"(?i)意外降息", r"(?i)紧急降息"],
    "fed_cut_expected":       [r"(?i)fed\s+cut", r"(?i)rate\s+cut", r"(?i)cut\s+rate", r"(?i)cuts?\s+rates?", r"(?i)降息\s*预期"],
    "fed_hold_hawkish":       [r"(?i)fed\s+hold.*hawk", r"(?i)rates?\s+unchanged.*hawk", r"(?i)利率不变.*鹰"],
    "fed_hike":               [r"(?i)fed\s+hike", r"(?i)rate\s+hike", r"(?i)加息"],
    "fed_dovish":             [r"(?i)fed\s+dovish", r"(?i)dovish\s+pivot", r"(?i)easing\s+cycle", r"(?i)鸽派"],
    # ── CPI / Inflation ──
    "cpi_hot":                [r"(?i)cpi\s+(hot|higher|above|surpass|beat)", r"(?i)inflation\s+(rose|higher|above|hot)", r"(?i)cpi\s*超预期"],
    "cpi_cool":               [r"(?i)cpi\s+(cool|lower|below|miss)", r"(?i)inflation\s+(fell|lower|below|cool)", r"(?i)cpi\s*低于", r"(?i)disinflation"],
    # ── Gold ──
    "gold_breakout":          [r"(?i)gold\s+(ath|record|breakout|all.time|surge|high)", r"(?i)gold.*(all.time|new)\s*high", r"(?i)xau\s+(breakout|ath|record)", r"(?i)黄金.*新高"],
    "gold_selloff":           [r"(?i)gold\s+(selloff|sell.off|crash|plunge|dump)", r"(?i)黄金.*暴跌"],
    # ── Whale / Smart Money ──  (before RWA so "whale bought ONDO" matches whale first)
    "whale_buy":              [r"(?i)whale\s+(bought|accumulated|buying|buy|transfer.*to\s+wallet)", r"(?i)large\s+buy", r"(?i)smart\s+money\s+buy", r"(?i)鲸鱼\s*买入"],
    "whale_sell":             [r"(?i)whale\s+(sold|dumped|selling|sell|transfer.*to\s+exchange)", r"(?i)large\s+sell", r"(?i)smart\s+money\s+sell", r"(?i)鲸鱼\s*卖出"],
    # ── Geopolitical ──
    "geopolitical_escalation":[r"(?i)(war|invasion|conflict|sanction|tension|missile|nuclear)", r"(?i)(战争|制裁|冲突|紧张)"],
    "geopolitical_deesc":     [r"(?i)(ceasefire|peace\s+deal|de.?escalat|truce)", r"(?i)(停火|和平|缓和)"],
    # ── Trade / Tariff ──
    "tariff_escalation":      [r"(?i)(tariff\s+(hike|increase|impose|new)|trade\s+war\s+escalat)", r"(?i)(关税.*上调|贸易战.*升级)"],
    "tariff_relief":          [r"(?i)(tariff\s+(cut|relief|exemption|pause|remove)|trade\s+deal)", r"(?i)(关税.*降低|贸易.*协议)"],
    # ── Liquidation ──
    "liquidation_cascade":    [r"(?i)(liquidated|liquidation|margin\s+call|cascade)", r"(?i)(清算|爆仓)"],
    # ── RWA Specific ──  (after whale so token mentions in whale context match whale first)
    "rwa_catalyst":           [r"(?i)\b(ondo|usdy|ousg)\b.*\b(launch|partner|tvl|yield|list)", r"(?i)(tokenized\s+treasury|rwa\s+tvl|blackrock\s+buidl)", r"(?i)(国债代币化|rwa).*利好"],
    "sec_rwa_positive":       [r"(?i)sec\s+(approv|clear|green.light)", r"(?i)etf\s+approv", r"(?i)regulatory\s+clarity"],
    "sec_rwa_negative":       [r"(?i)sec\s+(sued|reject|den|crackdown)", r"(?i)banned\s+crypto", r"(?i)监管.*打压"],
    # ── Employment / GDP ──
    "nfp_strong":             [r"(?i)(nonfarm|non.farm|payroll)\s*(beat|strong|above|surge)", r"(?i)非农.*超预期"],
    "nfp_weak":               [r"(?i)(nonfarm|non.farm|payroll)\s*(miss|weak|below|disappoint)", r"(?i)非农.*不及预期"],
    "gdp_strong":             [r"(?i)gdp\s*(beat|strong|above|surge|accelerat)", r"(?i)gdp.*超预期"],
    "gdp_weak":               [r"(?i)gdp\s*(miss|weak|below|contract|slow)", r"(?i)gdp.*不及预期"],
}

# ═══════════════════════════════════════════════════════════════════════
#  CLASSIFIER RULES — TG Intel style (keyword_any + keyword_not)
#  Applied to Telegram messages as fast pre-LLM classification
# ═══════════════════════════════════════════════════════════════════════
CLASSIFIER_RULES = [
    # Fed / Rates
    {"keywords_any": ["rate cut", "fed cut", "dovish", "lower rates", "easing", "降息"],
     "keywords_not": ["no cut", "unchanged", "expected"],
     "event_type": "fed_dovish", "direction": "bullish", "magnitude": 0.85,
     "affects": ["rwa", "perps", "spot_long"]},
    {"keywords_any": ["rate hike", "hawkish", "higher rates", "tightening", "no cut", "加息"],
     "keywords_not": [],
     "event_type": "fed_hawkish", "direction": "bearish", "magnitude": 0.80,
     "affects": ["rwa", "perps", "spot_long"]},
    # CPI
    {"keywords_any": ["cpi below", "inflation fell", "inflation lower", "disinflation", "cpi低于"],
     "keywords_not": [],
     "event_type": "cpi_cool", "direction": "bullish", "magnitude": 0.70,
     "affects": ["rwa", "perps", "spot_long"]},
    {"keywords_any": ["cpi above", "inflation rose", "inflation higher", "hot cpi", "cpi超预期"],
     "keywords_not": [],
     "event_type": "cpi_hot", "direction": "bearish", "magnitude": 0.70,
     "affects": ["rwa", "perps"]},
    # Gold
    {"keywords_any": ["gold ath", "gold surges", "gold rallies", "gold record", "xau breakout", "黄金新高"],
     "keywords_not": [],
     "event_type": "gold_breakout", "direction": "bullish", "magnitude": 0.75,
     "affects": ["rwa", "perps"]},
    # Whale
    {"keywords_any": ["whale bought", "whale accumulated", "whale transferred", "large buy", "鲸鱼买入"],
     "keywords_not": ["sold", "dumped"],
     "event_type": "whale_buy", "direction": "bullish", "magnitude": 0.60,
     "affects": ["spot_long", "meme", "wallet_tracker"]},
    {"keywords_any": ["whale sold", "whale dumped", "large sell", "whale to exchange", "鲸鱼卖出"],
     "keywords_not": [],
     "event_type": "whale_sell", "direction": "bearish", "magnitude": 0.65,
     "affects": ["spot_long", "meme", "wallet_tracker"]},
    # Regulatory
    {"keywords_any": ["sec approved", "etf approved", "regulatory clarity", "legal victory", "批准", "利好"],
     "keywords_not": [],
     "event_type": "sec_rwa_positive", "direction": "bullish", "magnitude": 0.80,
     "affects": ["rwa", "perps", "spot_long"]},
    {"keywords_any": ["sec sued", "banned crypto", "regulatory crackdown", "exchange charged", "监管打压"],
     "keywords_not": [],
     "event_type": "sec_rwa_negative", "direction": "bearish", "magnitude": 0.75,
     "affects": ["rwa", "perps", "spot_long"]},
    # RWA
    {"keywords_any": ["ondo", "usdy", "ousg", "tokenized treasury", "rwa tvl", "blackrock buidl"],
     "keywords_not": ["hack", "exploit", "depeg"],
     "event_type": "rwa_catalyst", "direction": "bullish", "magnitude": 0.65,
     "affects": ["rwa", "perps"]},
    # Liquidation
    {"keywords_any": ["liquidated", "liquidation", "margin call", "cascade", "清算", "爆仓"],
     "keywords_not": [],
     "event_type": "liquidation_cascade", "direction": "bearish", "magnitude": 0.75,
     "affects": ["spot_long", "perps", "meme"]},
    # Geopolitical
    {"keywords_any": ["war", "invasion", "missile", "sanctions", "conflict", "战争", "制裁"],
     "keywords_not": ["ceasefire", "peace"],
     "event_type": "geopolitical_escalation", "direction": "bearish", "magnitude": 0.70,
     "affects": ["rwa", "perps", "spot_long"]},
    # Tariff / Trade War
    {"keywords_any": ["tariff hike", "new tariff", "trade war escalat", "关税上调", "贸易战升级"],
     "keywords_not": ["relief", "exemption", "pause"],
     "event_type": "tariff_escalation", "direction": "bearish", "magnitude": 0.70,
     "affects": ["rwa", "perps", "spot_long"]},
    {"keywords_any": ["tariff cut", "tariff relief", "trade deal", "关税降低", "贸易协议"],
     "keywords_not": [],
     "event_type": "tariff_relief", "direction": "bullish", "magnitude": 0.65,
     "affects": ["rwa", "perps", "spot_long"]},
]

# ═══════════════════════════════════════════════════════════════════════
#  MACRO PLAYBOOK — Intelligence only (no buy/sell actions)
#  Maps event_type → direction, magnitude, affects, urgency
# ═══════════════════════════════════════════════════════════════════════
MACRO_PLAYBOOK = {
    "fed_cut_expected":       {"direction": "bullish",  "magnitude": 0.60, "affects": ["rwa", "perps", "spot_long"],            "urgency": 0.5},
    "fed_cut_surprise":       {"direction": "bullish",  "magnitude": 0.85, "affects": ["rwa", "perps", "spot_long", "meme"],    "urgency": 0.95},
    "fed_hold_hawkish":       {"direction": "bearish",  "magnitude": 0.70, "affects": ["rwa", "perps"],                         "urgency": 0.6},
    "fed_hike":               {"direction": "bearish",  "magnitude": 0.80, "affects": ["rwa", "perps", "spot_long"],            "urgency": 0.8},
    "fed_dovish":             {"direction": "bullish",  "magnitude": 0.75, "affects": ["rwa", "perps", "spot_long"],            "urgency": 0.6},
    "cpi_hot":                {"direction": "bearish",  "magnitude": 0.70, "affects": ["rwa", "perps"],                         "urgency": 0.6},
    "cpi_cool":               {"direction": "bullish",  "magnitude": 0.70, "affects": ["rwa", "perps", "spot_long"],            "urgency": 0.6},
    "gold_breakout":          {"direction": "bullish",  "magnitude": 0.75, "affects": ["rwa", "perps"],                         "urgency": 0.5},
    "gold_selloff":           {"direction": "bearish",  "magnitude": 0.65, "affects": ["rwa"],                                  "urgency": 0.4},
    "geopolitical_escalation":{"direction": "bearish",  "magnitude": 0.70, "affects": ["rwa", "perps", "spot_long"],            "urgency": 0.7},
    "geopolitical_deesc":     {"direction": "bullish",  "magnitude": 0.55, "affects": ["rwa", "perps", "spot_long"],            "urgency": 0.4},
    "tariff_escalation":      {"direction": "bearish",  "magnitude": 0.70, "affects": ["rwa", "perps", "spot_long"],            "urgency": 0.7},
    "tariff_relief":          {"direction": "bullish",  "magnitude": 0.65, "affects": ["rwa", "perps", "spot_long"],            "urgency": 0.5},
    "rwa_catalyst":           {"direction": "bullish",  "magnitude": 0.65, "affects": ["rwa", "perps"],                         "urgency": 0.4},
    "sec_rwa_positive":       {"direction": "bullish",  "magnitude": 0.80, "affects": ["rwa", "perps", "spot_long"],            "urgency": 0.7},
    "sec_rwa_negative":       {"direction": "bearish",  "magnitude": 0.75, "affects": ["rwa", "perps", "spot_long"],            "urgency": 0.7},
    "whale_buy":              {"direction": "bullish",  "magnitude": 0.60, "affects": ["spot_long", "meme", "wallet_tracker"],  "urgency": 0.5},
    "whale_sell":             {"direction": "bearish",  "magnitude": 0.65, "affects": ["spot_long", "meme", "wallet_tracker"],  "urgency": 0.5},
    "liquidation_cascade":    {"direction": "bearish",  "magnitude": 0.75, "affects": ["spot_long", "perps", "meme"],           "urgency": 0.8},
    "nfp_strong":             {"direction": "bearish",  "magnitude": 0.60, "affects": ["rwa", "perps"],                         "urgency": 0.5},
    "nfp_weak":               {"direction": "bullish",  "magnitude": 0.60, "affects": ["rwa", "perps", "spot_long"],            "urgency": 0.5},
    "gdp_strong":             {"direction": "bearish",  "magnitude": 0.55, "affects": ["rwa", "perps"],                         "urgency": 0.4},
    "gdp_weak":               {"direction": "bullish",  "magnitude": 0.55, "affects": ["rwa", "perps"],                         "urgency": 0.4},
}

# ═══════════════════════════════════════════════════════════════════════
#  SENTIMENT LEXICON (domain-tuned, weighted)
# ═══════════════════════════════════════════════════════════════════════
POSITIVE_WORDS = {
    "bullish": 0.7, "surge": 0.6, "rally": 0.6, "pump": 0.5,
    "breakout": 0.6, "ath": 0.7, "approved": 0.7, "adoption": 0.5,
    "partnership": 0.4, "launch": 0.4, "upgrade": 0.4, "growth": 0.5,
    "accumulation": 0.5, "inflow": 0.5, "record": 0.5, "milestone": 0.4,
    "dovish": 0.6, "easing": 0.5, "recovery": 0.5, "bought": 0.4,
    "涨": 0.5, "暴涨": 0.7, "突破": 0.6, "牛": 0.6, "利好": 0.7,
}
NEGATIVE_WORDS = {
    "bearish": -0.7, "crash": -0.8, "dump": -0.7, "rug": -0.9,
    "hack": -0.8, "exploit": -0.8, "depeg": -0.7, "banned": -0.6,
    "sued": -0.6, "liquidated": -0.6, "sell-off": -0.6, "fear": -0.5,
    "hawkish": -0.6, "tightening": -0.5, "crackdown": -0.6,
    "tariff": -0.4, "sanctions": -0.5, "war": -0.6,
    "跌": -0.5, "暴跌": -0.7, "崩盘": -0.8, "熊": -0.6, "利空": -0.7,
}
BULLISH_WORDS = {"rally", "surge", "breakout", "bullish", "moon", "pump",
                 "accumulate", "buy", "long", "ath", "inflow", "dovish"}
BEARISH_WORDS = {"crash", "dump", "plunge", "bearish", "sell", "short",
                 "liquidat", "rug", "hack", "hawkish", "crackdown"}

# ═══════════════════════════════════════════════════════════════════════
#  DEDUP
# ═══════════════════════════════════════════════════════════════════════
DEDUP_WINDOW_HOURS     = 4
DEDUP_SIMILARITY_CHARS = 100

# ═══════════════════════════════════════════════════════════════════════
#  REPUTATION SYSTEM
# ═══════════════════════════════════════════════════════════════════════
REPUTATION_ENABLED     = True
REPUTATION_DECAY_DAYS  = 30
REPUTATION_BOOST_ALPHA = 0.3
REPUTATION_BOOST_NEWS  = 0.1
REPUTATION_PENALTY_NOISE = -0.05
REPUTATION_MIN_SCORE   = -1.0
REPUTATION_MAX_SCORE   = 5.0
REPUTATION_HIGH_SIGNAL = 1.5       # Senders above this get 1.3x magnitude boost

# ═══════════════════════════════════════════════════════════════════════
#  OUTPUT
# ═══════════════════════════════════════════════════════════════════════
STATE_DIR              = "state"
MAX_SIGNALS_KEPT       = 500
DASHBOARD_PORT         = 3252

# Token extraction — noise words to exclude from ALL-CAPS ticker matching
TICKER_NOISE_WORDS = {
    "THE", "FOR", "AND", "NOT", "BUT", "ARE", "WAS", "HAS", "HAD",
    "WITH", "FROM", "THIS", "THAT", "WILL", "CAN", "ALL", "NOW",
    "NEW", "GET", "GOT", "OUT", "WHO", "HOW", "WHY", "ITS", "ANY",
    "MAY", "SAY", "SET", "RUN", "USE", "BIG", "OLD", "LOW", "TOP",
    "USD", "EUR", "GBP", "JPY", "CNY",
}
