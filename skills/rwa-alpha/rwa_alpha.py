#!/usr/bin/env python3
"""
rwa_alpha.py — RWA Alpha (Real World Asset Intelligence Trading Skill)
融合 NewsNow 宏观事件驱动 + Polymarket 概率确认 + 链上 DEX 执行的 RWA 交易策略。

用法:
  python3 rwa_alpha.py

依赖: Python 3.8+ 标准库 + onchainos CLI >= 2.1.0
"""

import subprocess, json, os, sys, time, threading, traceback, copy, re, math
from http.server import HTTPServer, SimpleHTTPRequestHandler
from socketserver import ThreadingMixIn
from pathlib import Path
from datetime import datetime
from collections import defaultdict
from urllib.request import urlopen, Request
from urllib.error import URLError

# ── 加载 Config ────────────────────────────────────────────────────────
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import config as C

# ── Constants ──────────────────────────────────────────────────────────
_ONCHAINOS = os.path.expanduser("~/.local/bin/onchainos")
_STATE_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "state")
os.makedirs(_STATE_DIR, exist_ok=True)

WALLET_ADDRESSES = {}  # chain -> address, set on startup

# ── State ──────────────────────────────────────────────────────────────
positions       = {}      # "SYM" -> position dict
signals_log     = []      # trade signals history
macro_events    = []      # detected macro events
yield_snapshots = {}      # SYM -> {apy, nav_premium, ...}
price_cache     = {}      # SYM -> {price, mc, volume, nav_premium, updated_ts}

pos_lock        = threading.RLock()
feed_lock       = threading.RLock()
cache_lock      = threading.RLock()
_buying         = set()
_selling        = set()
_cached_yield_ranking = []   # Updated each perception cycle, served to dashboard
_cached_api_json = '{"prices":{},"positions":{},"feed":[],"signals":[],"trades":[],"session":{},"yield_ranking":[],"portfolio":{"total_invested":0,"total_pnl":0,"portfolio_value":0,"categories":{}}}'

# Session stats
session = {
    "start_ts":      0,
    "buys":          0,
    "sells":         0,
    "wins":          0,
    "losses":        0,
    "net_pnl_usd":   0.0,
    "daily_trades":  0,
    "daily_reset":   0,
    "paused_until":  0,
    "total_invested": 0.0,
}

trades_log   = []     # completed trade records
live_feed    = []     # dashboard feed (max 200)
_bot_running = True


# ═══════════════════════════════════════════════════════════════════════
#  ONCHAINOS CLI WRAPPER
# ═══════════════════════════════════════════════════════════════════════

def _onchainos(*args, timeout: int = 25) -> dict:
    try:
        r = subprocess.run([_ONCHAINOS, *args],
                           capture_output=True, text=True, timeout=timeout)
        return json.loads(r.stdout)
    except Exception:
        return {"ok": False, "data": None}


def _cli_data(r: dict):
    d = r.get("data")
    if isinstance(d, list):
        return d[0] if d else {}
    return d or {}


def _cli_data_list(r: dict) -> list:
    d = r.get("data")
    return d if isinstance(d, list) else []


# ═══════════════════════════════════════════════════════════════════════
#  LOGGING / FEED
# ═══════════════════════════════════════════════════════════════════════

def log(msg: str):
    ts = time.strftime("%H:%M:%S")
    print(f"  [{ts}] {msg}")
    push_feed(msg)


def push_feed(msg: str, category: str = "info"):
    with feed_lock:
        entry = {"msg": str(msg), "t": time.strftime("%H:%M:%S"), "cat": category}
        live_feed.insert(0, entry)
        if len(live_feed) > 200:
            live_feed[:] = live_feed[:200]


# ═══════════════════════════════════════════════════════════════════════
#  PERSISTENCE
# ═══════════════════════════════════════════════════════════════════════

def _atomic_write(filepath: str, data):
    os.makedirs(os.path.dirname(filepath), exist_ok=True)
    tmp = filepath + ".tmp"
    with open(tmp, "w") as f:
        json.dump(data, f, indent=2, default=str)
    os.replace(tmp, filepath)


def save_positions():
    with pos_lock:
        _atomic_write(os.path.join(_STATE_DIR, "positions.json"), positions)

def load_positions():
    global positions
    fp = os.path.join(_STATE_DIR, "positions.json")
    if os.path.exists(fp):
        try:
            with open(fp) as f: positions = json.load(f)
            # Migrate: add has_nav field if missing (backward compat)
            for sym, pos in positions.items():
                if "has_nav" not in pos:
                    token = C.RWA_UNIVERSE.get(sym, {})
                    pos["has_nav"] = token.get("has_nav", pos.get("asset_backed", False))
            log(f"Loaded {len(positions)} positions from disk")
        except Exception: positions = {}

def save_trades():
    _atomic_write(os.path.join(_STATE_DIR, "trades.json"), trades_log)

def load_trades():
    global trades_log
    fp = os.path.join(_STATE_DIR, "trades.json")
    if os.path.exists(fp):
        try:
            with open(fp) as f: trades_log = json.load(f)
        except Exception: trades_log = []

def save_signals():
    _atomic_write(os.path.join(_STATE_DIR, "signals.json"), signals_log[-200:])

def save_macro_events():
    _atomic_write(os.path.join(_STATE_DIR, "macro_events.json"), macro_events[-100:])

def save_yield_snapshots():
    _atomic_write(os.path.join(_STATE_DIR, "yield_snapshots.json"), yield_snapshots)


# ═══════════════════════════════════════════════════════════════════════
#  ONCHAINOS DATA APIs
# ═══════════════════════════════════════════════════════════════════════

def price_info(token_addr: str, chain: str = "ethereum") -> dict:
    r = _onchainos("token", "price-info",
                   "--chain", chain, "--address", token_addr)
    items = _cli_data_list(r)
    if not items:
        items = [_cli_data(r)]
    return items[0] if items else {}


def advanced_info(token_addr: str, chain: str = "ethereum") -> dict:
    r = _onchainos("token", "advanced-info",
                   "--chain", chain, "--address", token_addr)
    return _cli_data(r)


def get_token_price(sym: str) -> dict:
    """Get price + market data for an RWA token. Returns {price, mc, volume, liquidity}."""
    token = C.RWA_UNIVERSE.get(sym)
    if not token:
        return {}

    # Try each chain the token is on (not limited to ENABLED_CHAINS — read-only)
    for chain in token.get("chains", []):
        addr = token.get("addresses", {}).get(chain, "")
        if not addr:
            continue
        try:
            pi = price_info(addr, chain)
            price = float(pi.get("price", pi.get("usdPrice", 0)) or 0)
            if price > 0:
                return {
                    "price":          price,
                    "mc":             float(pi.get("marketCap", pi.get("fdv", 0)) or 0),
                    "volume_24h":     float(pi.get("volume24H", pi.get("volume24h", pi.get("volume", 0))) or 0),
                    "liquidity":      float(pi.get("liquidity", 0) or 0),
                    "priceChange24H": float(pi.get("priceChange24H", 0) or 0),
                    "chain":          chain,
                    "address":        addr,
                }
        except Exception:
            continue
    return {}


def get_wallet_balance(chain: str) -> float:
    """Get native balance (ETH/SOL) on a chain."""
    chain_idx = C.CHAIN_CONFIG.get(chain, {}).get("chain_index", "1")
    r = _onchainos("wallet", "balance", "--chain", chain_idx)
    data = _cli_data(r)
    try:
        return float(data.get("balance", data.get("totalBalance", 0)) or 0)
    except (ValueError, TypeError):
        return 0.0


# ═══════════════════════════════════════════════════════════════════════
#  PERCEPTION LAYER — Read the World
# ═══════════════════════════════════════════════════════════════════════

# ── Token Price & NAV Monitor ──────────────────────────────────────────

_tier_assignments = {}  # sym -> tier (0, 1, 2), reassigned periodically
_tier_last_reassign = 0

def _assign_tiers():
    """Assign polling tiers: 0=held, 1=native+top20, 2=rest."""
    global _tier_assignments, _tier_last_reassign
    tiers = {}
    # Tier 0: held positions
    with pos_lock:
        held = set(positions.keys())
    for sym in held:
        tiers[sym] = 0
    # Tier 1: native RWA + top 20 by cached liquidity
    for sym in getattr(C, "NATIVE_RWA_SYMBOLS", []):
        if sym not in tiers:
            tiers[sym] = 1
    # Top 20 by liquidity from cache
    with cache_lock:
        liq_ranked = sorted(
            ((s, c.get("liquidity", 0)) for s, c in price_cache.items() if s not in tiers),
            key=lambda x: x[1], reverse=True
        )
    for sym, _ in liq_ranked[:20]:
        if sym not in tiers:
            tiers[sym] = 1
    # Tier 2: everything else
    for sym in C.RWA_UNIVERSE:
        if sym not in tiers:
            tiers[sym] = 2
    _tier_assignments = tiers
    _tier_last_reassign = time.time()


def _fetch_batch(symbols: list):
    """Fetch prices for a batch of symbols using threads."""
    from concurrent.futures import ThreadPoolExecutor
    batch_size = getattr(C, "POLL_BATCH_SIZE", 10)
    def _fetch_one(sym):
        token = C.RWA_UNIVERSE.get(sym)
        if not token:
            return
        if C.STRATEGY_MODE == "yield_optimizer" and not token.get("asset_backed"):
            return
        data = get_token_price(sym)
        if data and data.get("price", 0) > 0:
            with cache_lock:
                price_cache[sym] = {**data, "updated_ts": time.time()}
    with ThreadPoolExecutor(max_workers=batch_size) as pool:
        pool.map(_fetch_one, symbols)


_initial_fetch_done = False

def refresh_price_cache():
    """Tiered price polling: Tier 0 (60s) → Tier 1 (5min) → Tier 2 (30min)."""
    global _tier_last_reassign, _initial_fetch_done
    now = time.time()

    # Reassign tiers every 30 min or on first run
    if now - _tier_last_reassign > 1800 or not _tier_assignments:
        _assign_tiers()

    # On startup, bulk fetch ALL tokens (no tier limits)
    if not _initial_fetch_done:
        _initial_fetch_done = True
        all_syms = list(C.RWA_UNIVERSE.keys())
        log(f"🚀 Initial bulk fetch: {len(all_syms)} tokens...")
        _fetch_batch(all_syms)
        log(f"✅ Initial fetch done: {len(price_cache)} tokens with data")
        return

    tier_0_sec = getattr(C, "POLL_TIER_0_SEC", 60)
    tier_1_sec = getattr(C, "POLL_TIER_1_SEC", 300)
    tier_2_sec = getattr(C, "POLL_TIER_2_SEC", 1800)

    t0, t1, t2 = [], [], []
    for sym, tier in _tier_assignments.items():
        with cache_lock:
            last = price_cache.get(sym, {}).get("updated_ts", 0)
        age = now - last
        if tier == 0 and age >= tier_0_sec:
            t0.append(sym)
        elif tier == 1 and age >= tier_1_sec:
            t1.append(sym)
        elif tier == 2 and age >= tier_2_sec:
            t2.append(sym)

    # Always fetch tier 0 immediately
    if t0:
        _fetch_batch(t0)
    # Tier 1 next
    if t1:
        _fetch_batch(t1)
    # Tier 2: batched, cap at 50 per cycle to avoid API spam
    if t2:
        _fetch_batch(t2[:50])


def get_nav_premium(sym: str) -> float:
    """
    Estimate NAV premium/discount for asset-backed tokens.
    Positive = trading above NAV (overvalued), Negative = below NAV (undervalued).

    For gold tokens: compare DEX price vs gold spot (approximated by token MC / supply).
    For treasury tokens: compare DEX price vs $1 peg or vs reported NAV.
    Returns value in decimal (e.g., 0.003 = 0.3% premium).
    """
    token = C.RWA_UNIVERSE.get(sym, {})
    if not token.get("has_nav", False):
        return 0.0

    with cache_lock:
        cached = price_cache.get(sym, {})
    price = cached.get("price", 0)
    if price <= 0:
        return 0.0

    category = token.get("category", "")

    if category == "gold":
        # Gold tokens: NAV ~ gold spot. PAXG/XAUT should track gold closely.
        # We use the token's own MC / supply as fair-value anchor.
        # Premium vs 24h TWAP approximation
        mc = cached.get("mc", 0)
        if mc > 0 and price > 0:
            # Simple: if price deviates from (MC / circulating), there's a premium
            # For a properly priced token, price ≈ MC / supply, so premium ≈ 0
            # We track relative change from the cache as a proxy
            return 0.0  # Will be refined with external gold price feed

    elif category == "treasury":
        # Treasury tokens: USDY, sDAI trade near $1 but with yield accrual
        # OUSG tracks treasury ETF NAV
        if sym in ("USDY", "sDAI", "USDe"):
            # These accrue yield, so fair value > $1 and grows over time
            # Premium = how far current DEX price is from the yield-adjusted NAV
            # Simplified: any deviation > 0.5% from recent average is notable
            return 0.0  # Placeholder — needs NAV oracle integration

        if sym == "OUSG":
            # OUSG NAV is reported daily by Ondo
            return 0.0  # Placeholder — needs Ondo API

    return 0.0


# ── News & Sentiment APIs ─────────────────────────────────────────────

_POLYMARKET_BASE = "https://gamma-api.polymarket.com/markets"
# Google News RSS — reliable, no API key, 100 results per query
_GNEWS_RSS = "https://news.google.com/rss/search?hl=en-US&gl=US&ceid=US:en&q="
_NEWS_QUERIES = [
    "federal+reserve+OR+FOMC+OR+rate+cut+OR+rate+hike",
    "CPI+inflation+OR+treasury+yield",
    "gold+XAU+OR+gold+price",
    "SEC+tokenization+OR+RWA+crypto+OR+Ondo+OR+Pendle",
]
_news_cache = {"headlines": [], "polymarket": [], "ts": 0}
_news_lock = threading.Lock()

# Keyword patterns for macro event detection from headlines
_MACRO_KEYWORDS = {
    "fed_cut_expected":      [r"(?i)fed\s+cut", r"(?i)rate\s+cut\s+expect", r"(?i)降息\s*预期"],
    "fed_cut_surprise":      [r"(?i)surprise\s+cut", r"(?i)emergency\s+cut", r"(?i)意外降息"],
    "fed_hold_hawkish":      [r"(?i)fed\s+hold.*hawk", r"(?i)rates?\s+unchanged.*hawk", r"(?i)鹰派.*按兵不动"],
    "fed_hike":              [r"(?i)fed\s+hike", r"(?i)rate\s+hike", r"(?i)加息"],
    "cpi_hot":               [r"(?i)cpi\s+(hot|higher|above|surpass|beat)", r"(?i)cpi.*超预期", r"(?i)通胀.*高于"],
    "cpi_cool":              [r"(?i)cpi\s+(cool|lower|below|miss)", r"(?i)cpi.*低于预期", r"(?i)通胀.*降温"],
    "gold_breakout":         [r"(?i)gold\s+(ath|record|breakout|all.time)", r"(?i)黄金.*新高", r"(?i)gold.*surge"],
    "geopolitical_escalation":[r"(?i)(war|conflict|sanction|tension|missile|nuclear)", r"(?i)(战争|冲突|制裁|紧张)"],
    "ondo_yield_increase":   [r"(?i)ondo.*yield.*(?:increas|rais|up|hike)", r"(?i)usdy.*apy.*(?:up|increas)"],
    "maker_dsr_up":          [r"(?i)maker.*dsr.*(?:increas|rais|up)", r"(?i)dai.*savings.*rate.*up"],
    "sec_rwa_positive":      [r"(?i)sec.*(?:approv|clear|positive).*(?:rwa|tokeniz|asset)", r"(?i)rwa.*(?:合规|批准|利好)"],
    "sec_rwa_negative":      [r"(?i)sec.*(?:reject|block|sue|crack).*(?:rwa|tokeniz|asset)", r"(?i)rwa.*(?:监管|打压|利空)"],
    "credit_expansion":      [r"(?i)credit\s+(eas|expan|boom)", r"(?i)lending\s+(grow|expan|boom)", r"(?i)信贷.*扩张"],
    "credit_tightening":     [r"(?i)credit\s+(tight|crunch|default)", r"(?i)lending\s+(crisis|default|freeze)", r"(?i)信贷.*收紧"],
    "equity_market_rally":   [r"(?i)(stocks?|equit|nasdaq|s&p)\s+(rally|surge|breakout|boom|record)", r"(?i)(美股|纳斯达克|标普).*(?:暴涨|创新高|大涨)"],
    "equity_market_crash":   [r"(?i)(stocks?|equit|nasdaq|s&p)\s+(crash|plunge|selloff|collapse|tank)", r"(?i)(美股|纳斯达克|标普).*(?:暴跌|崩盘|大跌)"],
    "sp500_breakout":        [r"(?i)s&?p\s*500\s*(ath|record|break|all.time|high)", r"(?i)标普.*新高"],
}

# Simple sentiment keywords (FinBERT-lite)
_BULLISH_WORDS = {"rally", "surge", "breakout", "soar", "bullish", "moon", "pump",
                  "ath", "record", "all-time high", "涨", "暴涨", "突破", "牛市", "利好"}
_BEARISH_WORDS = {"crash", "dump", "plunge", "tank", "bearish", "collapse", "fear",
                  "sell-off", "selloff", "panic", "跌", "暴跌", "崩盘", "熊市", "利空"}


def _http_get_json(url: str, timeout: int = 10) -> dict:
    """Stdlib HTTP GET → JSON. Returns {} on any failure."""
    try:
        req = Request(url, headers={"User-Agent": "RWA-Alpha/1.1"})
        with urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read().decode())
    except Exception:
        return {}


def fetch_news_headlines() -> list:
    """
    Fetch latest financial headlines from Google News RSS.
    Returns list of {title, url, source, ts}.
    """
    import xml.etree.ElementTree as ET
    all_items = []
    seen_titles = set()
    for query in _NEWS_QUERIES:
        try:
            req = Request(f"{_GNEWS_RSS}{query}",
                          headers={"User-Agent": "RWA-Alpha/1.1"})
            with urlopen(req, timeout=10) as resp:
                xml_data = resp.read().decode()
            root = ET.fromstring(xml_data)
            for item in root.findall(".//item")[:20]:
                title_el = item.find("title")
                title = title_el.text.strip() if title_el is not None and title_el.text else ""
                if not title or title in seen_titles:
                    continue
                seen_titles.add(title)
                source_el = item.find("source")
                link_el = item.find("link")
                pub_el = item.find("pubDate")
                all_items.append({
                    "title":  title,
                    "url":    link_el.text.strip() if link_el is not None and link_el.text else "",
                    "source": source_el.text.strip() if source_el is not None and source_el.text else "Google News",
                    "ts":     pub_el.text.strip() if pub_el is not None and pub_el.text else "",
                })
        except Exception:
            continue
    return all_items


def fetch_polymarket_signals() -> list:
    """
    Fetch Polymarket prediction markets for macro signal confirmation.
    Returns list of {question, probability, category}.
    """
    results = []
    # Focus on Fed, CPI, gold-related markets
    for query in ["fed rate", "cpi inflation", "gold price"]:
        try:
            url = f"{_POLYMARKET_BASE}?active=true&closed=false&limit=5&tag=economics"
            data = _http_get_json(url, timeout=8)
            markets = data if isinstance(data, list) else data.get("data", data.get("markets", []))
            if isinstance(markets, list):
                for m in markets[:5]:
                    question = m.get("question", m.get("title", ""))
                    if not question:
                        continue
                    # Extract probability (outcomePrices is usually a JSON string)
                    prices = m.get("outcomePrices", "")
                    prob = 0.5
                    try:
                        if isinstance(prices, str):
                            prices = json.loads(prices)
                        if isinstance(prices, list) and prices:
                            prob = float(prices[0])
                    except Exception:
                        pass
                    results.append({
                        "question":    question,
                        "probability": prob,
                        "category":    m.get("groupSlug", m.get("category", "")),
                    })
        except Exception:
            continue
    return results


def _analyze_headline_sentiment(title: str) -> float:
    """
    Quick keyword-based sentiment score for a headline.
    Returns -1.0 (bearish) to +1.0 (bullish).
    """
    title_lower = title.lower()
    bull = sum(1 for w in _BULLISH_WORDS if w in title_lower)
    bear = sum(1 for w in _BEARISH_WORDS if w in title_lower)
    if bull + bear == 0:
        return 0.0
    return max(-1.0, min(1.0, (bull - bear) / max(bull + bear, 1)))


def _match_macro_event(title: str) -> list:
    """Match a headline against macro keyword patterns. Returns list of event_type strings."""
    matched = []
    for event_type, patterns in _MACRO_KEYWORDS.items():
        for pat in patterns:
            if re.search(pat, title):
                matched.append(event_type)
                break
    return matched


# ── LLM Headline Classification ──────────────────────────────────────

_LLM_EVENT_TYPES = ", ".join(sorted(set([
    "fed_cut_expected", "fed_cut_surprise", "fed_hold_hawkish", "fed_hike",
    "cpi_hot", "cpi_cool", "gold_breakout", "gold_selloff",
    "geopolitical_escalation", "ondo_yield_increase", "maker_dsr_up",
    "sec_rwa_positive", "sec_rwa_negative",
    "equity_market_rally", "equity_market_crash", "sp500_breakout",
    "credit_expansion", "credit_tightening", "none",
])))

_LLM_SYSTEM_PROMPT = f"""You classify financial headlines into macro event types for an RWA (Real World Asset) trading system.

Valid event types: {_LLM_EVENT_TYPES}

Rules:
- Return ONLY the event_type string, nothing else
- "none" if the headline is irrelevant to RWA/macro/rates/gold/regulation
- Read the FULL nuance: "Fed holds but hints at cuts" = fed_hold_hawkish (NOT fed_cut_expected)
- "SEC delays ruling" = none (delay is not positive or negative)
- "Gold drops on profit-taking" = none (temporary, not structural selloff)
- "Gold drops 3% on strong jobs data" = gold_selloff (fundamental driver)
- Be conservative: when unsure, return "none"
"""

_llm_cache = {}  # title_hash -> (event_type, ts)


def _llm_classify_headline(title: str) -> str:
    """
    Call Anthropic Haiku to classify a single headline.
    Returns event_type string or "none". Falls back to "none" on any error.
    Cost: ~$0.005 per call.
    """
    if not getattr(C, "LLM_ENABLED", False):
        return "none"

    api_key = os.environ.get("ANTHROPIC_API_KEY", "")
    if not api_key:
        return "none"

    # Check cache (dedupe identical headlines within 10 min)
    cache_key = hash(title)
    cached = _llm_cache.get(cache_key)
    if cached and time.time() - cached[1] < 600:
        return cached[0]

    try:
        payload = json.dumps({
            "model": getattr(C, "LLM_MODEL", "claude-haiku-4-5-20251001"),
            "max_tokens": 30,
            "system": _LLM_SYSTEM_PROMPT,
            "messages": [{"role": "user", "content": f"Classify: {title}"}],
        }).encode()

        req = Request(
            "https://api.anthropic.com/v1/messages",
            data=payload,
            headers={
                "Content-Type": "application/json",
                "x-api-key": api_key,
                "anthropic-version": "2023-06-01",
            },
            method="POST",
        )
        with urlopen(req, timeout=5) as resp:
            body = json.loads(resp.read().decode())

        # Extract text from response
        text = ""
        for block in body.get("content", []):
            if block.get("type") == "text":
                text = block["text"].strip().lower()
                break

        # Validate it's a known event type
        if text in MACRO_PLAYBOOK or text == "none":
            _llm_cache[cache_key] = (text, time.time())
            # Keep cache small
            if len(_llm_cache) > 200:
                oldest = sorted(_llm_cache, key=lambda k: _llm_cache[k][1])[:100]
                for k in oldest:
                    _llm_cache.pop(k, None)
            return text

    except Exception as e:
        log(f"LLM classify error: {e}")

    return "none"


# ── RWA-relevant headline filter (for LLM pass) ─────────────────────

_RWA_RELEVANCE = re.compile(
    r"(?i)(fed|fomc|rate|cpi|inflation|treasury|gold|xau|sec|rwa|tokeniz|"
    r"ondo|usdy|ousg|paxg|centrifuge|maple|dai savings|dsr|maker|"
    r"pendle|plume|mantra|goldfinch|truefi|credit|lending|"
    r"降息|加息|通胀|黄金|国债|监管|代币化|信贷)",
)


def refresh_news_cache():
    """Refresh the global news + polymarket cache (called from perception loop)."""
    headlines = fetch_news_headlines()
    polymarket = fetch_polymarket_signals()
    with _news_lock:
        _news_cache["headlines"] = headlines
        _news_cache["polymarket"] = polymarket
        _news_cache["ts"] = time.time()


# ── Macro Event Detection ─────────────────────────────────────────────

# Macro playbook: event_type -> action mapping
def _resolve_playbook_targets(categories: list, limit: int = 20) -> list:
    """Resolve category names → concrete symbols, capped by limit, sorted by cached liquidity."""
    syms = []
    for sym, token in C.RWA_UNIVERSE.items():
        if token.get("category") in categories:
            # Must have at least one enabled chain
            if any(c in C.ENABLED_CHAINS for c in token.get("chains", [])):
                with cache_lock:
                    liq = price_cache.get(sym, {}).get("liquidity", 0)
                syms.append((sym, liq))
    syms.sort(key=lambda x: x[1], reverse=True)
    return [s for s, _ in syms[:limit]]

def _symbols_by_category(*categories) -> list:
    """Get symbols from universe matching any of the given categories."""
    return [sym for sym, t in C.RWA_UNIVERSE.items() if t.get("category") in categories]

MACRO_PLAYBOOK = {
    "fed_cut_expected": {
        "action": "buy", "target_categories": ["treasury"],
        "targets": ["USDY", "OUSG", "bIB01"],  # fallback if category resolution empty
        "conviction": 0.60, "urgency": "session",
        "rationale": "Rate cut in line → higher demand for locked-in treasury yield",
    },
    "fed_cut_surprise": {
        "action": "strong_buy", "target_categories": ["treasury", "yield_protocol"],
        "targets": ["USDY", "OUSG", "ONDO", "bIB01", "PENDLE"],
        "conviction": 0.85, "urgency": "immediate",
        "rationale": "Surprise cut → bond rally → NAV appreciation + narrative pump + yield repricing",
    },
    "fed_hold_hawkish": {
        "action": "rotate",
        "sell_categories": ["rwa_gov", "rwa_infra", "yield_protocol"],
        "sell": ["ONDO", "CFG", "PLUME", "OM", "PENDLE"],
        "buy": ["USDY"],
        "conviction": 0.70, "urgency": "session",
        "rationale": "Hawkish hold → risk-off for governance, flight to yield safety",
    },
    "fed_hike": {
        "action": "sell_risk",
        "sell_categories": ["rwa_gov", "rwa_infra", "rwa_credit", "yield_protocol"],
        "sell": ["ONDO", "CFG", "MPL", "PLUME", "OM", "GFI", "TRU", "PENDLE"],
        "conviction": 0.80, "urgency": "immediate",
        "rationale": "Surprise hike → sell risk assets, rates higher for longer",
    },
    "cpi_hot": {
        "action": "buy", "target_categories": ["gold"],
        "targets": ["PAXG", "XAUT"],
        "conviction": 0.75, "urgency": "session",
        "rationale": "Hot CPI → inflation fear → gold bid",
    },
    "cpi_cool": {
        "action": "buy", "target_categories": ["treasury"],
        "targets": ["OUSG", "USDY", "bIB01"],
        "conviction": 0.70, "urgency": "session",
        "rationale": "Cool CPI → rate cut expectations → treasury rally",
    },
    "gold_breakout": {
        "action": "buy", "target_categories": ["gold"],
        "targets": ["PAXG", "XAUT"],
        "conviction": 0.80, "urgency": "immediate",
        "rationale": "Gold ATH breakout + potential PAXG NAV discount = alpha",
    },
    "geopolitical_escalation": {
        "action": "buy", "target_categories": ["gold"],
        "targets": ["PAXG"],
        "conviction": 0.65, "urgency": "session",
        "rationale": "Safe haven flow → gold + treasuries",
    },
    "ondo_yield_increase": {
        "action": "buy", "targets": ["USDY", "ONDO", "PENDLE"],
        "conviction": 0.70, "urgency": "next_cycle",
        "rationale": "Higher USDY yield → more TVL → ONDO governance premium + yield repricing",
    },
    "maker_dsr_up": {
        "action": "buy", "targets": ["sDAI"],
        "conviction": 0.65, "urgency": "next_cycle",
        "rationale": "Higher DSR → sDAI more attractive vs alternatives",
    },
    "sec_rwa_positive": {
        "action": "buy",
        "target_categories": ["rwa_gov", "rwa_infra", "rwa_credit"],
        "targets": ["ONDO", "CFG", "MPL", "PLUME", "OM", "GFI", "TRU"],
        "conviction": 0.60, "urgency": "session",
        "rationale": "Regulatory clarity → institutional inflow narrative for all RWA tokens",
    },
    "sec_rwa_negative": {
        "action": "sell_risk",
        "sell_categories": ["rwa_gov", "rwa_infra"],
        "sell": ["ONDO", "CFG", "PLUME", "OM"],
        "conviction": 0.75, "urgency": "immediate",
        "rationale": "Regulatory crackdown → governance/infra tokens hit, assets unaffected",
    },
    "gold_selloff": {
        "action": "sell_risk", "sell_categories": ["gold"],
        "sell": ["PAXG", "XAUT"],
        "conviction": 0.65, "urgency": "session",
        "rationale": "Gold dropping → risk of NAV discount on gold-backed tokens",
    },
    "credit_expansion": {
        "action": "buy", "target_categories": ["rwa_credit"],
        "targets": ["GFI", "TRU", "MPL"],
        "conviction": 0.60, "urgency": "session",
        "rationale": "Credit easing/expansion → lending protocols benefit",
    },
    "credit_tightening": {
        "action": "sell_risk", "sell_categories": ["rwa_credit"],
        "sell": ["GFI", "TRU", "MPL"],
        "conviction": 0.70, "urgency": "session",
        "rationale": "Credit tightening/defaults → lending protocol risk",
    },
    # ── Equity market events (NEW) ──
    "equity_market_rally": {
        "action": "buy",
        "target_categories": ["xstock", "ondo_tokenized", "stablestock"],
        "targets": [],
        "conviction": 0.60, "urgency": "session",
        "rationale": "Broad equity rally → tokenized stock demand spills over to on-chain",
    },
    "equity_market_crash": {
        "action": "sell_risk",
        "sell_categories": ["xstock", "ondo_tokenized", "stablestock", "leveraged"],
        "sell": [],
        "conviction": 0.80, "urgency": "immediate",
        "rationale": "Equity crash → tokenized stocks drop with underlying, leveraged ETFs amplify",
    },
    "sp500_breakout": {
        "action": "buy",
        "target_categories": ["ondo_tokenized", "xstock"],
        "targets": [],
        "conviction": 0.65, "urgency": "session",
        "rationale": "S&P 500 breakout → momentum into tokenized index/equity products",
    },
}


def detect_macro_events() -> list:
    """
    Detect macro events from 3 sources:
      1. NewsNow headlines → keyword matching against MACRO_PLAYBOOK
      2. Polymarket → prediction market confirmation signals
      3. On-chain price action → gold breakout/selloff, volume spikes

    Returns list of {type, direction, magnitude, affected, urgency, ts, detail}
    """
    events = []
    now = time.time()
    _seen_types = set()  # Deduplicate within one cycle

    # ── Source 1: News headlines → keyword match + LLM for ambiguous ──
    with _news_lock:
        headlines = list(_news_cache.get("headlines", []))
    llm_band = getattr(C, "LLM_CONFIDENCE_BAND", (0.55, 0.80))

    for item in headlines[:30]:
        title = item.get("title", "")
        if not title:
            continue

        # Layer 1: Fast keyword match
        matched_types = _match_macro_event(title)

        if matched_types:
            # Keywords matched → use them (high confidence, no LLM needed)
            for etype in matched_types:
                if etype in _seen_types:
                    continue
                _seen_types.add(etype)
                playbook = MACRO_PLAYBOOK.get(etype, {})
                base_conv = playbook.get("conviction", 0.7)

                # Layer 2: LLM confirmation for ambiguous band
                if llm_band[0] <= base_conv < llm_band[1]:
                    llm_result = _llm_classify_headline(title)
                    if llm_result == "none":
                        # LLM says this is a false positive → skip
                        log(f"LLM override: '{title[:50]}' keyword={etype} → none (skipped)")
                        continue
                    elif llm_result != etype:
                        # LLM disagrees with keyword → trust LLM
                        log(f"LLM reclassify: '{title[:50]}' keyword={etype} → {llm_result}")
                        etype = llm_result
                        playbook = MACRO_PLAYBOOK.get(etype, playbook)

                events.append({
                    "type":     etype,
                    "direction": "news",
                    "magnitude": 0.7,
                    "affected":  playbook.get("targets", playbook.get("sell", [])),
                    "urgency":   playbook.get("urgency", "session"),
                    "ts":        now,
                    "detail":    f"[{item.get('source','')}] {title[:80]}",
                })
        else:
            # Layer 3: No keyword match — but is headline RWA-relevant?
            # If so, ask LLM to classify (catches nuanced headlines keywords miss)
            if _RWA_RELEVANCE.search(title):
                llm_result = _llm_classify_headline(title)
                if llm_result != "none" and llm_result not in _seen_types:
                    _seen_types.add(llm_result)
                    playbook = MACRO_PLAYBOOK.get(llm_result, {})
                    if playbook:
                        log(f"LLM discovered: '{title[:50]}' → {llm_result}")
                        events.append({
                            "type":     llm_result,
                            "direction": "news_llm",
                            "magnitude": 0.65,  # Slightly lower confidence for LLM-only
                            "affected":  playbook.get("targets", playbook.get("sell", [])),
                            "urgency":   playbook.get("urgency", "session"),
                            "ts":        now,
                            "detail":    f"[LLM|{item.get('source','')}] {title[:80]}",
                        })

    # ── Source 2: Polymarket confirmation ──
    with _news_lock:
        poly_markets = list(_news_cache.get("polymarket", []))
    for pm in poly_markets:
        q = pm.get("question", "").lower()
        prob = pm.get("probability", 0.5)
        # Boost macro events if Polymarket confirms high probability
        if "rate cut" in q and prob > 0.65 and "fed_cut_expected" not in _seen_types:
            _seen_types.add("fed_cut_expected")
            events.append({
                "type": "fed_cut_expected", "direction": "polymarket",
                "magnitude": prob, "affected": ["USDY", "OUSG"],
                "urgency": "session", "ts": now,
                "detail": f"Polymarket: {pm['question'][:60]} ({prob:.0%})",
            })
        if "rate hike" in q and prob > 0.60 and "fed_hike" not in _seen_types:
            _seen_types.add("fed_hike")
            events.append({
                "type": "fed_hike", "direction": "polymarket",
                "magnitude": prob, "affected": ["ONDO", "CFG", "MPL"],
                "urgency": "immediate", "ts": now,
                "detail": f"Polymarket: {pm['question'][:60]} ({prob:.0%})",
            })

    # ── Source 3: On-chain price action (gold + volume) ──
    gold_syms = _symbols_by_category("gold") or ["PAXG", "XAUT"]
    for gold_sym in gold_syms:
        with cache_lock:
            cached = price_cache.get(gold_sym, {})
        if not cached:
            continue
        price = cached.get("price", 0)

        prev_entry = None
        for evt in reversed(macro_events[-50:]):
            if evt.get("_price_ref") == gold_sym:
                prev_entry = evt
                break

        if prev_entry:
            prev_price = prev_entry.get("_price_val", price)
            if prev_price > 0:
                change_pct = (price - prev_price) / prev_price * 100
                if change_pct > 2.0 and "gold_breakout" not in _seen_types:
                    _seen_types.add("gold_breakout")
                    events.append({
                        "type": "gold_breakout", "direction": "gold_bull",
                        "magnitude": min(1.0, change_pct / 5.0),
                        "affected": ["PAXG", "XAUT"], "urgency": "immediate",
                        "ts": now, "detail": f"{gold_sym} +{change_pct:.1f}%",
                    })
                elif change_pct < -2.0 and "gold_selloff" not in _seen_types:
                    _seen_types.add("gold_selloff")
                    events.append({
                        "type": "gold_selloff", "direction": "gold_bear",
                        "magnitude": min(1.0, abs(change_pct) / 5.0),
                        "affected": ["PAXG", "XAUT"], "urgency": "session",
                        "ts": now, "detail": f"{gold_sym} {change_pct:.1f}%",
                    })

        macro_events.append({
            "_price_ref": gold_sym, "_price_val": price,
            "ts": now, "type": "_price_snapshot",
        })

    # Volume spikes on governance tokens
    gov_syms = _symbols_by_category("rwa_gov", "yield_protocol", "rwa_infra", "rwa_credit")
    if not gov_syms:
        gov_syms = ["ONDO", "CFG", "MPL", "PENDLE", "PLUME", "OM", "GFI", "TRU"]
    for gov_sym in gov_syms:
        if C.STRATEGY_MODE == "yield_optimizer":
            continue
        with cache_lock:
            cached = price_cache.get(gov_sym, {})
        if not cached:
            continue
        volume = cached.get("volume_24h", 0)
        mc = cached.get("mc", 0)
        if mc > 0 and volume > 0:
            vol_mc_ratio = volume / mc
            if vol_mc_ratio > 0.10:
                events.append({
                    "type": "volume_spike", "direction": "momentum",
                    "magnitude": min(1.0, vol_mc_ratio / 0.20),
                    "affected": [gov_sym], "urgency": "session",
                    "ts": now, "detail": f"{gov_sym} vol/MC {vol_mc_ratio:.1%}",
                })

    return events


# ── Sentiment Scoring (Simplified) ────────────────────────────────────

def get_sentiment_score(sym: str) -> float:
    """
    Composite sentiment score combining:
      1. News headline sentiment (keyword-based FinBERT-lite)
      2. On-chain volume/liquidity signals
    Range: -1.0 (extremely bearish) to +1.0 (extremely bullish).
    """
    token = C.RWA_UNIVERSE.get(sym, {})
    token_name = token.get("name", sym).lower()
    category = token.get("category", "")

    # ── Layer 1: News sentiment (weight 0.6) ──
    news_score = 0.0
    news_count = 0
    with _news_lock:
        headlines = list(_news_cache.get("headlines", []))
    # Search for headlines mentioning this token or its category
    search_terms = [sym.lower(), token_name]
    if category == "gold":
        search_terms.extend(["gold", "黄金", "paxg", "xaut"])
    elif category == "treasury":
        search_terms.extend(["treasury", "国债", "t-bill", "usdy", "yield"])
    elif category == "rwa_gov":
        search_terms.extend(["rwa", "tokeniz", "代币化"])

    for item in headlines[:40]:
        title = item.get("title", "").lower()
        if any(term in title for term in search_terms):
            news_score += _analyze_headline_sentiment(title)
            news_count += 1

    if news_count > 0:
        news_score = news_score / news_count  # Average sentiment
    else:
        news_score = 0.0  # No relevant news = neutral

    # ── Layer 2: On-chain signals (weight 0.4) ──
    chain_score = 0.0
    with cache_lock:
        cached = price_cache.get(sym, {})
    if cached:
        volume = cached.get("volume_24h", 0)
        mc = cached.get("mc", 0)
        liquidity = cached.get("liquidity", 0)
        if mc > 0 and volume > 0:
            vol_ratio = volume / mc
            if vol_ratio > 0.05:
                chain_score += 0.4
            elif vol_ratio > 0.02:
                chain_score += 0.15
        if liquidity > 1_000_000:
            chain_score += 0.1
        elif liquidity < 200_000:
            chain_score -= 0.2

    # Weighted composite
    composite = news_score * 0.6 + chain_score * 0.4
    return max(-1.0, min(1.0, composite))


# ═══════════════════════════════════════════════════════════════════════
#  COGNITION LAYER — Think Like a Quant
# ═══════════════════════════════════════════════════════════════════════

def rank_yield_opportunities() -> list:
    """
    Rank all yield-bearing RWA tokens by composite alpha score.
    Returns sorted list of {sym, price, liquidity, sentiment, alpha_score, chain}.
    """
    candidates = []
    for sym, token in C.RWA_UNIVERSE.items():
        if not token.get("has_nav", False):
            continue  # Skip non-NAV tokens for yield ranking

        with cache_lock:
            cached = price_cache.get(sym, {})
        if not cached or cached.get("price", 0) <= 0:
            continue

        nav_premium = get_nav_premium(sym)
        sentiment = get_sentiment_score(sym)
        liquidity = cached.get("liquidity", 0)

        # Liquidity score (0-1)
        liq_score = min(1.0, liquidity / 2_000_000) if liquidity > 0 else 0

        # Composite alpha score
        alpha = (
            max(0, -nav_premium * 100) * 0.30       # Discount = opportunity
            + (sentiment + 1) / 2 * 0.25             # Positive sentiment
            + liq_score * 0.25                        # Can we actually trade it
            + 0.20                                    # Base (equal weighting start)
        )

        candidates.append({
            "sym":         sym,
            "name":        token.get("name", sym),
            "category":    token["category"],
            "price":       cached["price"],
            "liquidity":   liquidity,
            "nav_premium": nav_premium,
            "sentiment":   sentiment,
            "alpha_score": round(alpha, 3),
            "chain":       cached.get("chain", "ethereum"),
            "address":     cached.get("address", ""),
        })

    return sorted(candidates, key=lambda x: x["alpha_score"], reverse=True)


def compose_signal(events: list) -> list:
    """
    Signal composition: combine macro events, sentiment, and relative value
    into actionable TradeSignal dicts.

    Priority:
    1. Macro event override (high conviction events)
    2. Yield rotation (better risk-adjusted yield elsewhere)
    3. Governance token momentum (volume spikes)

    Returns list of signal dicts.
    """
    signals = []

    # ── Priority 1: Macro event signals ──
    for event in events:
        etype = event.get("type", "")
        if etype.startswith("_"):
            continue  # Skip internal markers

        playbook = MACRO_PLAYBOOK.get(etype)
        if not playbook:
            continue

        action = playbook["action"]
        conviction = playbook["conviction"] * event.get("magnitude", 0.5)

        if conviction < C.MIN_CONVICTION:
            continue

        if action in ("buy", "strong_buy"):
            # Resolve targets: category-based or explicit list
            targets = playbook.get("targets", [])
            if playbook.get("target_categories"):
                cat_targets = _resolve_playbook_targets(playbook["target_categories"])
                if cat_targets:
                    targets = cat_targets
            for target in targets:
                token = C.RWA_UNIVERSE.get(target)
                if not token:
                    continue
                if C.STRATEGY_MODE == "yield_optimizer" and not token.get("asset_backed"):
                    continue

                signals.append({
                    "action":     "buy",
                    "sym":        target,
                    "conviction": min(1.0, conviction),
                    "reason":     playbook["rationale"],
                    "source":     f"macro:{etype}",
                    "urgency":    playbook.get("urgency", "session"),
                })

        elif action == "sell_risk":
            with pos_lock:
                held = set(positions.keys())
            for target in playbook.get("sell", []):
                if target not in held:
                    continue  # Only sell what we actually hold
                signals.append({
                    "action":     "sell",
                    "sym":        target,
                    "conviction": min(1.0, conviction),
                    "reason":     playbook["rationale"],
                    "source":     f"macro:{etype}",
                    "urgency":    playbook.get("urgency", "immediate"),
                })

        elif action == "rotate":
            with pos_lock:
                held = set(positions.keys())
            for sell_sym in playbook.get("sell", []):
                if sell_sym not in held:
                    continue
                signals.append({
                    "action": "sell", "sym": sell_sym,
                    "conviction": conviction, "reason": playbook["rationale"],
                    "source": f"macro:{etype}", "urgency": "session",
                })
            for buy_sym in playbook.get("buy", []):
                signals.append({
                    "action": "buy", "sym": buy_sym,
                    "conviction": conviction, "reason": playbook["rationale"],
                    "source": f"macro:{etype}", "urgency": "session",
                })

    # ── Priority 2: Governance momentum ──
    if C.STRATEGY_MODE in ("macro_trader", "full_alpha"):
        for event in events:
            if event.get("type") == "volume_spike":
                for sym in event.get("affected", []):
                    signals.append({
                        "action":     "buy",
                        "sym":        sym,
                        "conviction": 0.55 * event.get("magnitude", 0.5),
                        "reason":     f"Volume spike: {event.get('detail', '')}",
                        "source":     "momentum:volume",
                        "urgency":    "next_cycle",
                    })

    # ── Priority 3: Alpha-score entry (full_alpha only) ──
    # When we have fewer positions than allowed, buy top-ranked yield opportunities
    if C.STRATEGY_MODE == "full_alpha" and not signals:
        with pos_lock:
            current_pos = set(positions.keys())
        if len(current_pos) < C.MAX_POSITIONS:
            top_yields = rank_yield_opportunities()
            log(f"🔍 Alpha entry scan: {len(top_yields)} candidates, {len(current_pos)} positions")
            for opp in top_yields[:3]:
                log(f"   → {opp.get('sym')} alpha={opp.get('alpha_score',0):.3f} liq={opp.get('liquidity',0):.0f}")
            for opp in top_yields[:2]:
                sym = opp.get("sym", "")
                if sym in current_pos or sym in _buying:
                    continue
                alpha = opp.get("alpha_score", 0)
                if alpha >= 0.30:
                    signals.append({
                        "action":     "buy",
                        "sym":        sym,
                        "conviction": min(0.70, 0.50 + alpha * 0.30),
                        "reason":     f"Alpha score {alpha:.3f} — top yield opportunity",
                        "source":     "alpha:yield_rank",
                        "urgency":    "next_cycle",
                    })

    # Filter by minimum conviction
    signals = [s for s in signals if s.get("conviction", 0) >= C.MIN_CONVICTION]

    return signals


# ═══════════════════════════════════════════════════════════════════════
#  EXECUTION LAYER — Trade On-Chain
# ═══════════════════════════════════════════════════════════════════════

def get_best_route(sym: str) -> dict:
    """Find the best chain + address to trade a token."""
    token = C.RWA_UNIVERSE.get(sym, {})
    best = None

    for chain in token.get("chains", []):
        if chain not in C.ENABLED_CHAINS:
            continue
        addr = token.get("addresses", {}).get(chain, "")
        if not addr:
            continue

        try:
            pi = price_info(addr, chain)
            liq = float(pi.get("liquidity", 0) or 0)
            price = float(pi.get("price", 0) or 0)
            if price <= 0:
                continue

            route = {
                "chain":     chain,
                "address":   addr,
                "price":     price,
                "liquidity": liq,
            }
            if best is None or liq > best.get("liquidity", 0):
                best = route
        except Exception:
            continue

    return best or {}


def risk_check_signal(signal: dict) -> tuple:
    """
    Risk gate check. Returns (approved: bool, reason: str).
    """
    sym = signal.get("sym", "")

    # Daily trade limit
    if session["daily_trades"] >= C.MAX_DAILY_TRADES:
        return False, f"DAILY_LIMIT {session['daily_trades']}/{C.MAX_DAILY_TRADES}"

    # Session stop
    if session["net_pnl_usd"] <= -C.SESSION_STOP_USD:
        return False, f"SESSION_STOP: lost ${abs(session['net_pnl_usd']):.0f}"

    # Cooldown
    now = time.time()
    if session["paused_until"] > now:
        remaining = int(session["paused_until"] - now)
        return False, f"COOLDOWN {remaining}s"

    if C.PAUSED:
        return False, "PAUSED"

    if signal["action"] == "buy":
        # Position concentration
        with pos_lock:
            if sym in positions:
                return False, f"ALREADY_HOLDING {sym}"
            if len(positions) >= C.MAX_POSITIONS:
                return False, f"MAX_POS {len(positions)}/{C.MAX_POSITIONS}"

            # Category concentration
            token = C.RWA_UNIVERSE.get(sym, {})
            cat = token.get("category", "")
            cat_total = sum(
                p.get("usd_in", 0) for s, p in positions.items()
                if C.RWA_UNIVERSE.get(s, {}).get("category") == cat
            )
            budget_pct = (cat_total + C.BUY_AMOUNT_USD) / max(C.TOTAL_BUDGET_USD, 1) * 100
            if budget_pct > C.MAX_CATEGORY_PCT:
                return False, f"CAT_LIMIT {cat} would be {budget_pct:.0f}% > {C.MAX_CATEGORY_PCT}%"

        # Budget check
        if session["total_invested"] + C.BUY_AMOUNT_USD > C.TOTAL_BUDGET_USD:
            return False, f"BUDGET ${session['total_invested']:.0f}/${C.TOTAL_BUDGET_USD:.0f}"

        # Liquidity check
        route = get_best_route(sym)
        if route.get("liquidity", 0) < C.MIN_LIQUIDITY_USD:
            return False, f"LOW_LIQ ${route.get('liquidity', 0):,.0f} < ${C.MIN_LIQUIDITY_USD:,.0f}"

        # NAV premium check for NAV-trackable tokens only
        token = C.RWA_UNIVERSE.get(sym, {})
        if token.get("has_nav", False):
            nav_p = get_nav_premium(sym)
            if nav_p * 10000 > C.MAX_NAV_PREMIUM_BPS:
                return False, f"NAV_PREMIUM {nav_p*100:.2f}% too high"

        # Conviction check
        if signal.get("conviction", 0) < C.MIN_CONVICTION:
            return False, f"LOW_CONVICTION {signal.get('conviction', 0):.2f}"

    return True, "approved"


def execute_buy(sym: str, signal: dict):
    """Execute a buy order for an RWA token."""
    if sym in _buying:
        return
    _buying.add(sym)

    try:
        route = get_best_route(sym)
        if not route:
            log(f"⛔ {sym} — no route found on enabled chains")
            return

        chain = route["chain"]
        token_addr = route["address"]
        entry_price = route["price"]

        if entry_price <= 0:
            log(f"⛔ {sym} — no price data")
            return

        chain_cfg = C.CHAIN_CONFIG.get(chain, {})
        stable_addr = chain_cfg.get("stable", "")
        chain_name = chain_cfg.get("chain", chain)
        chain_idx = chain_cfg.get("chain_index", "1")

        # Calculate token amount from USD
        amount_usd = C.BUY_AMOUNT_USD
        # For stablecoin swap: amount in smallest unit
        # USDC has 6 decimals on ETH, 6 on SOL
        amount_raw = str(int(amount_usd * 1e6))

        if C.MODE == "paper":
            token_amount = amount_usd / entry_price if entry_price > 0 else 0
            tx_hash = f"PAPER_{int(time.time())}"
            status = "SUCCESS"
        else:
            # Quote
            try:
                r = _onchainos("swap", "quote", "--chain", chain_name,
                               "--from", stable_addr, "--to", token_addr,
                               "--amount", amount_raw)
                quote = _cli_data(r)
                if isinstance(quote, list) and quote:
                    quote = quote[0]
                token_amount = float(quote.get("toTokenAmount", 0))
                if token_amount <= 0:
                    log(f"⛔ {sym} bad quote — 0 output")
                    return
            except Exception as e:
                log(f"⛔ {sym} quote error: {e}")
                return

            # Swap
            try:
                wallet_addr = WALLET_ADDRESSES.get(chain, "")
                r = _onchainos("swap", "swap", "--chain", chain_name,
                               "--from", stable_addr, "--to", token_addr,
                               "--amount", amount_raw,
                               "--slippage", str(int(C.SLIPPAGE_BUY)),
                               "--wallet-address", wallet_addr,
                               timeout=30)
                swap_data = _cli_data(r)
                if isinstance(swap_data, list) and swap_data:
                    swap_data = swap_data[0]
                tx_obj = swap_data.get("tx", "")
                unsigned_tx = tx_obj.get("data", "") if isinstance(tx_obj, dict) else tx_obj
                if not unsigned_tx:
                    raise ValueError("Empty tx from swap")
                tx_to = tx_obj.get("to", token_addr) if isinstance(tx_obj, dict) else token_addr

                # Sign + Broadcast
                r2 = _onchainos("wallet", "contract-call",
                                "--chain", chain_idx,
                                "--to", tx_to,
                                "--unsigned-tx", unsigned_tx,
                                timeout=60)
                data2 = _cli_data(r2)
                if isinstance(data2, list) and data2:
                    data2 = data2[0]
                tx_hash = data2.get("txHash", "") if isinstance(data2, dict) else ""
                if not tx_hash:
                    raise ValueError("No txHash returned")
            except Exception as e:
                log(f"❌ {sym} tx error: {e}")
                return

            # Confirm
            status = _wait_tx(tx_hash, chain_idx)
            if status == "FAILED":
                log(f"❌ {sym} tx FAILED: {tx_hash}")
                return

        # Record position
        pos = {
            "symbol":        sym,
            "address":       token_addr,
            "chain":         chain,
            "entry_price":   entry_price,
            "entry_ts":      time.time(),
            "entry_human":   time.strftime("%m-%d %H:%M:%S"),
            "usd_in":        amount_usd,
            "token_amount":  token_amount,
            "current_price": entry_price,
            "pnl_pct":       0.0,
            "pnl_usd":       0.0,
            "peak_price":    entry_price,
            "peak_pnl_pct":  0.0,
            "trailing_active": False,
            "signal_source": signal.get("source", ""),
            "signal_reason": signal.get("reason", ""),
            "conviction":    signal.get("conviction", 0),
            "category":      C.RWA_UNIVERSE.get(sym, {}).get("category", ""),
            "asset_backed":  C.RWA_UNIVERSE.get(sym, {}).get("asset_backed", False),
            "has_nav":       C.RWA_UNIVERSE.get(sym, {}).get("has_nav", False),
            "tx_hash":       tx_hash,
        }

        with pos_lock:
            positions[sym] = pos
            save_positions()

        session["buys"] += 1
        session["daily_trades"] += 1
        session["total_invested"] += amount_usd

        mode_label = "PAPER" if C.MODE == "paper" else "LIVE"
        log(f"🛒 BUY [{mode_label}] {sym} | ${amount_usd} @ ${entry_price:.4f} on {chain} | "
            f"conviction={signal.get('conviction', 0):.0%} | {signal.get('source', '')}")
        push_feed(f"BUY {sym} ${amount_usd}", "trade")

        # Log signal
        signals_log.append({
            "ts":         time.strftime("%m-%d %H:%M:%S"),
            "action":     "BUY",
            "sym":        sym,
            "usd":        amount_usd,
            "price":      entry_price,
            "conviction": signal.get("conviction", 0),
            "source":     signal.get("source", ""),
            "reason":     signal.get("reason", ""),
        })
        save_signals()

    except Exception as e:
        log(f"🔴 BUY CRASH [{sym}]: {e}")
        traceback.print_exc()
    finally:
        _buying.discard(sym)


def execute_sell(sym: str, sell_pct: float, reason: str):
    """Sell a position (full or partial)."""
    with pos_lock:
        if sym not in positions:
            return
        if sym in _selling:
            return
        _selling.add(sym)
        pos = copy.deepcopy(positions[sym])

    try:
        chain = pos.get("chain", "ethereum")
        token_addr = pos["address"]
        chain_cfg = C.CHAIN_CONFIG.get(chain, {})
        chain_name = chain_cfg.get("chain", chain)
        chain_idx = chain_cfg.get("chain_index", "1")
        stable_addr = chain_cfg.get("stable", "")

        token_amount = pos.get("token_amount", 0)
        sell_qty = token_amount * min(sell_pct, 1.0)
        if sell_qty <= 0:
            return
        # Token decimals: most ERC-20 RWA tokens use 18 decimals, USDC=6
        # token_amount from quote is already in raw units; convert if stored as float
        if sell_qty < 1e6:
            # Likely a float amount (e.g. 50.0 tokens) — convert to raw with 18 decimals
            sell_amount = str(int(sell_qty * 1e18))
        else:
            # Already in raw units
            sell_amount = str(int(sell_qty))

        if C.MODE == "paper":
            status = "SUCCESS"
        else:
            try:
                wallet_addr = WALLET_ADDRESSES.get(chain, "")
                r = _onchainos("swap", "swap", "--chain", chain_name,
                               "--from", token_addr, "--to", stable_addr,
                               "--amount", str(sell_amount),
                               "--slippage", str(int(C.SLIPPAGE_SELL)),
                               "--wallet-address", wallet_addr,
                               timeout=30)
                swap_data = _cli_data(r)
                if isinstance(swap_data, list) and swap_data:
                    swap_data = swap_data[0]
                tx_obj = swap_data.get("tx", "")
                unsigned_tx = tx_obj.get("data", "") if isinstance(tx_obj, dict) else tx_obj
                if not unsigned_tx:
                    raise ValueError("Empty tx (sell)")
                tx_to = tx_obj.get("to", stable_addr) if isinstance(tx_obj, dict) else stable_addr
                r2 = _onchainos("wallet", "contract-call",
                                "--chain", chain_idx,
                                "--to", tx_to,
                                "--unsigned-tx", unsigned_tx,
                                timeout=60)
                data2 = _cli_data(r2)
                if isinstance(data2, list) and data2:
                    data2 = data2[0]
                tx_hash = data2.get("txHash", "") if isinstance(data2, dict) else ""
                if not tx_hash:
                    raise ValueError("No txHash (sell)")
                status = _wait_tx(tx_hash, chain_idx)
            except Exception as e:
                log(f"❌ SELL {sym}: {e}")
                return

            if status == "FAILED":
                log(f"❌ SELL {sym} tx FAILED")
                return

        # PnL calc
        exit_price = pos.get("current_price", pos["entry_price"])
        if pos["entry_price"] > 0:
            pnl_pct = (exit_price - pos["entry_price"]) / pos["entry_price"] * 100
        else:
            pnl_pct = 0.0
        pnl_usd = pos["usd_in"] * sell_pct * (pnl_pct / 100)

        is_full = sell_pct >= 0.99

        trade_record = {
            "t":       time.strftime("%m-%d %H:%M"),
            "sym":     sym,
            "pnl_pct": round(pnl_pct, 2),
            "pnl_usd": round(pnl_usd, 2),
            "usd_in":  round(pos["usd_in"] * sell_pct, 2),
            "reason":  reason,
            "partial": not is_full,
            "chain":   chain,
        }

        if is_full:
            with pos_lock:
                positions.pop(sym, None)
                save_positions()
        else:
            with pos_lock:
                if sym in positions:
                    positions[sym]["token_amount"] = token_amount - sell_amount
                    positions[sym]["usd_in"] *= (1 - sell_pct)
                save_positions()

        trades_log.insert(0, trade_record)
        save_trades()

        session["sells"] += 1
        session["daily_trades"] += 1
        session["net_pnl_usd"] += pnl_usd
        if pnl_pct > 0:
            session["wins"] += 1
        else:
            session["losses"] += 1

        icon = "✅" if pnl_pct > 0 else "❌"
        log(f"{icon} SELL {sym} | {reason} | {pnl_pct:+.1f}% (${pnl_usd:+.0f})")
        push_feed(f"SELL {sym} {pnl_pct:+.1f}%", "trade")

        signals_log.append({
            "ts":     time.strftime("%m-%d %H:%M:%S"),
            "action": "SELL",
            "sym":    sym,
            "pnl":    f"{pnl_pct:+.1f}%",
            "reason": reason,
        })
        save_signals()

    except Exception as e:
        log(f"🔴 SELL CRASH [{sym}]: {e}")
        traceback.print_exc()
    finally:
        _selling.discard(sym)


def _wait_tx(tx_hash: str, chain_idx: str) -> str:
    """Poll for tx confirmation."""
    for _ in range(20):
        time.sleep(3)
        try:
            r = _onchainos("wallet", "history",
                           "--tx-hash", tx_hash,
                           "--chain", chain_idx)
            data = _cli_data(r)
            item = data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})
            status = str(item.get("txStatus", "0"))
            if status in ("1", "2", "SUCCESS"):
                return "SUCCESS"
            if status in ("3", "FAILED"):
                return "FAILED"
        except Exception:
            pass
    return "TIMEOUT"


# ═══════════════════════════════════════════════════════════════════════
#  POSITION MONITOR
# ═══════════════════════════════════════════════════════════════════════

def check_positions():
    """Check all open positions for exit conditions."""
    with pos_lock:
        syms = list(positions.keys())

    for sym in syms:
        with pos_lock:
            if sym not in positions:
                continue
            pos = copy.deepcopy(positions[sym])

        # Get current price
        data = get_token_price(sym)
        if not data or data.get("price", 0) <= 0:
            continue
        price = data["price"]

        # Update position state
        entry = pos["entry_price"]
        if entry <= 0:
            continue
        pnl_pct = (price - entry) / entry * 100
        pnl_usd = pos["usd_in"] * (pnl_pct / 100)
        peak_price = max(pos.get("peak_price", entry), price)
        peak_pnl = (peak_price - entry) / entry * 100

        with pos_lock:
            if sym in positions:
                positions[sym]["current_price"] = price
                positions[sym]["pnl_pct"] = round(pnl_pct, 2)
                positions[sym]["pnl_usd"] = round(pnl_usd, 2)
                positions[sym]["peak_price"] = peak_price
                positions[sym]["peak_pnl_pct"] = round(peak_pnl, 2)

        has_nav = pos.get("has_nav", pos.get("asset_backed", False))
        is_asset = pos.get("asset_backed", False)

        # ── Exit logic for NAV-trackable tokens (treasury, gold, defi_yield) ──
        if has_nav:
            # NAV premium take-profit
            nav_p = get_nav_premium(sym)
            if nav_p * 10000 > C.TP_NAV_PREMIUM_BPS and C.TP_NAV_PREMIUM_BPS > 0:
                execute_sell(sym, 1.0, f"TP_NAV_PREMIUM({nav_p*100:.2f}%)")
                continue

            # Hard stop loss (NAV discount)
            if pnl_pct <= -(C.SL_NAV_DISCOUNT_BPS / 100):
                execute_sell(sym, 1.0, f"SL_NAV({pnl_pct:+.1f}%)")
                continue

            # Yield rotation check (periodic, using interval tracking)
            last_yield_check = session.get("_last_yield_check", 0)
            now_t = time.time()
            if now_t - last_yield_check >= C.YIELD_CHECK_SEC:
                session["_last_yield_check"] = now_t
                top_yields = rank_yield_opportunities()
                if top_yields and top_yields[0]["sym"] != sym:
                    best = top_yields[0]
                    current_score = next(
                        (y["alpha_score"] for y in top_yields if y["sym"] == sym), 0
                    )
                    if best["alpha_score"] - current_score > 0.15:
                        execute_sell(sym, 1.0, f"YIELD_ROTATE→{best['sym']}")
                        rotate_signal = {
                            "action": "buy", "sym": best["sym"],
                            "conviction": 0.65,
                            "reason": f"Yield rotation from {sym} (alpha +{best['alpha_score']-current_score:.2f})",
                            "source": "yield_rotation",
                        }
                        threading.Thread(
                            target=execute_buy,
                            args=(best["sym"], rotate_signal),
                            daemon=True
                        ).start()
                        continue

        # ── Exit logic for TOKENIZED EQUITIES (asset_backed + no NAV) ──
        elif is_asset and not has_nav:
            tp = getattr(C, "TP_EQUITY_PCT", 15)
            sl = getattr(C, "SL_EQUITY_PCT", -8)
            if pnl_pct >= tp:
                execute_sell(sym, 1.0, f"TP_EQUITY({pnl_pct:+.1f}%)")
                continue
            if pnl_pct <= sl:
                execute_sell(sym, 1.0, f"SL_EQUITY({pnl_pct:+.1f}%)")
                continue
            # Trailing stop for equities
            if peak_pnl >= C.TRAILING_ACTIVATE:
                drop = peak_pnl - pnl_pct
                with pos_lock:
                    if sym in positions:
                        positions[sym]["trailing_active"] = True
                if drop >= C.TRAILING_DROP:
                    execute_sell(sym, 1.0, f"TRAIL_EQ({peak_pnl:+.1f}%→{pnl_pct:+.1f}%)")
                    continue

        # ── Exit logic for GOVERNANCE / utility tokens ──
        else:
            # Take profit
            if pnl_pct >= C.TP_GOVERNANCE_PCT:
                execute_sell(sym, 1.0, f"TP({pnl_pct:+.1f}%)")
                continue

            # Stop loss
            if pnl_pct <= C.SL_GOVERNANCE_PCT:
                execute_sell(sym, 1.0, f"SL({pnl_pct:+.1f}%)")
                continue

            # Trailing stop
            if peak_pnl >= C.TRAILING_ACTIVATE:
                drop = peak_pnl - pnl_pct
                with pos_lock:
                    if sym in positions:
                        positions[sym]["trailing_active"] = True
                if drop >= C.TRAILING_DROP:
                    execute_sell(sym, 1.0, f"TRAIL({peak_pnl:+.1f}%→{pnl_pct:+.1f}%)")
                    continue

    # Drawdown check (portfolio level)
    with pos_lock:
        total_pnl = sum(p.get("pnl_usd", 0) for p in positions.values())
        total_invested = sum(p.get("usd_in", 0) for p in positions.values())

    if total_invested > 0:
        drawdown_pct = (total_pnl / total_invested) * 100
        if drawdown_pct <= -C.MAX_DRAWDOWN_PCT:
            log(f"🚨 PORTFOLIO DRAWDOWN {drawdown_pct:.1f}% → closing all positions")
            with pos_lock:
                all_syms = list(positions.keys())
            for s in all_syms:
                execute_sell(s, 1.0, f"MAX_DRAWDOWN({drawdown_pct:.1f}%)")

    save_positions()


# ═══════════════════════════════════════════════════════════════════════
#  MAIN LOOPS
# ═══════════════════════════════════════════════════════════════════════

def perception_loop():
    """Perception layer: refresh prices, detect macro events."""
    log(f"👁️ Perception loop started | news_poll={C.NEWS_POLL_SEC}s | chain_poll={C.CHAIN_POLL_SEC}s")

    last_news = 0
    while _bot_running:
        try:
            # Refresh price cache
            refresh_price_cache()

            # Update cached data for dashboard (non-blocking)
            global _cached_yield_ranking, _cached_api_json
            try:
                _cached_yield_ranking = rank_yield_opportunities()
            except Exception:
                pass
            try:
                _cached_api_json = json.dumps(_dashboard_api_data(), default=str)
            except Exception:
                pass

            # Macro event detection (less frequent)
            now = time.time()
            if now - last_news >= C.NEWS_POLL_SEC:
                # Refresh news headlines + polymarket before detecting events
                try:
                    refresh_news_cache()
                except Exception:
                    pass
                events = detect_macro_events()
                real_events = [e for e in events if not e.get("type", "").startswith("_")]
                if real_events:
                    log(f"📰 Detected {len(real_events)} macro event(s)")
                    for e in real_events:
                        log(f"   → {e['type']}: {e.get('detail', '')}")
                        push_feed(f"MACRO: {e['type']} — {e.get('detail', '')}", "macro")

                # Compose signals (always run — alpha-score entry needs this even without macro events)
                signals = compose_signal(events)
                if signals:
                    for signal in signals:
                        approved, reason = risk_check_signal(signal)
                        if approved:
                            if signal["action"] == "buy":
                                threading.Thread(
                                    target=execute_buy,
                                    args=(signal["sym"], signal),
                                    daemon=True
                                ).start()
                            elif signal["action"] == "sell":
                                threading.Thread(
                                    target=execute_sell,
                                    args=(signal["sym"], 1.0, signal.get("reason", "signal")),
                                    daemon=True
                                ).start()
                        else:
                            log(f"🚫 Signal blocked: {signal['action']} {signal['sym']} — {reason}")

                    save_macro_events()

                    # Refresh dashboard cache after signal execution (wait briefly for buy threads)
                    time.sleep(3)
                    try:
                        _cached_api_json = json.dumps(_dashboard_api_data(), default=str)
                    except Exception:
                        pass

                last_news = now

        except Exception as e:
            log(f"🔴 Perception error: {e}")
            traceback.print_exc()

        time.sleep(C.CHAIN_POLL_SEC)


def monitor_loop():
    """Monitor open positions for exit conditions."""
    log(f"📊 Monitor loop started | interval={C.CHAIN_POLL_SEC}s")

    while _bot_running:
        try:
            # Reset daily trade counter
            today = time.strftime("%Y-%m-%d")
            if session.get("daily_reset") != today:
                session["daily_trades"] = 0
                session["daily_reset"] = today

            check_positions()

            # Refresh dashboard cache
            global _cached_api_json
            try:
                _cached_api_json = json.dumps(_dashboard_api_data(), default=str)
            except Exception:
                pass
        except Exception as e:
            log(f"🔴 Monitor error: {e}")
            traceback.print_exc()

        time.sleep(C.CHAIN_POLL_SEC)


# ═══════════════════════════════════════════════════════════════════════
#  DASHBOARD
# ═══════════════════════════════════════════════════════════════════════

def _dashboard_api_data() -> dict:
    with pos_lock:
        pos_copy = copy.deepcopy(positions)
    with feed_lock:
        feed_copy = list(live_feed[:100])
    with cache_lock:
        prices = copy.deepcopy(price_cache)

    # Portfolio summary
    total_invested = sum(p.get("usd_in", 0) for p in pos_copy.values())
    total_pnl = sum(p.get("pnl_usd", 0) for p in pos_copy.values())
    portfolio_value = total_invested + total_pnl

    # Category breakdown
    categories = defaultdict(lambda: {"invested": 0, "pnl": 0, "count": 0})
    for sym, pos in pos_copy.items():
        cat = pos.get("category", "unknown")
        categories[cat]["invested"] += pos.get("usd_in", 0)
        categories[cat]["pnl"] += pos.get("pnl_usd", 0)
        categories[cat]["count"] += 1

    return {
        "mode":            C.MODE,
        "paused":          C.PAUSED,
        "strategy_mode":   C.STRATEGY_MODE,
        "chains":          C.ENABLED_CHAINS,
        "positions":       pos_copy,
        "trades":          trades_log[:50],
        "signals":         signals_log[-30:],
        "feed":            feed_copy,
        "session":         session,
        "prices":          prices,
        "yield_ranking":   _cached_yield_ranking,
        "portfolio": {
            "total_invested": total_invested,
            "total_pnl":      total_pnl,
            "portfolio_value": portfolio_value,
            "categories":     dict(categories),
        },
        "ts": time.strftime("%H:%M:%S"),
    }


_cached_universe_json = ""

def _build_universe_json():
    """Build /api/universe response — token list with categories + enabled chains."""
    tokens = {}
    for sym, token in C.RWA_UNIVERSE.items():
        addr = ""
        for chain in token.get("chains", []):
            a = token.get("addresses", {}).get(chain, "")
            if a:
                addr = a
                break
        tokens[sym] = {
            "name":         token.get("name", sym),
            "cat":          token.get("category", ""),
            "backed":       token.get("asset_backed", False),
            "has_nav":      token.get("has_nav", False),
            "source":       token.get("source", "csv"),
            "chains":       token.get("chains", []),
            "addr":         addr,
            "logo":         token.get("logo", ""),
        }
    return json.dumps({
        "tokens":     tokens,
        "categories": C.CATEGORY_NAMES,
        "chains":     list(C.CHAIN_CONFIG.keys()),
        "enabled":    C.ENABLED_CHAINS,
        "count":      len(tokens),
    })


class DashboardHandler(SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/api/state":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(_cached_api_json.encode())
        elif self.path == "/api/universe":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            global _cached_universe_json
            if not _cached_universe_json:
                _cached_universe_json = _build_universe_json()
            self.wfile.write(_cached_universe_json.encode())
        elif self.path == "/" or self.path == "/index.html":
            html_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "dashboard.html")
            if os.path.exists(html_path):
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.end_headers()
                with open(html_path, "rb") as f:
                    self.wfile.write(f.read())
            else:
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.end_headers()
                self.wfile.write(b"<html><body><h1>RWA Alpha</h1>"
                                 b"<p>dashboard.html not found</p></body></html>")
        else:
            super().do_GET()

    def log_message(self, format, *args):
        pass


def start_dashboard():
    try:
        class ThreadedHTTPServer(ThreadingMixIn, HTTPServer):
            daemon_threads = True
        server = ThreadedHTTPServer(("127.0.0.1", C.DASHBOARD_PORT), DashboardHandler)
        log(f"🌐 Dashboard: http://localhost:{C.DASHBOARD_PORT}")
        server.serve_forever()
    except Exception as e:
        log(f"⚠️ Dashboard failed: {e}")


# ═══════════════════════════════════════════════════════════════════════
#  STARTUP / INTERACTIVE SETUP
# ═══════════════════════════════════════════════════════════════════════

def _wallet_preflight() -> dict:
    """Check wallet login and return addresses per chain."""
    addresses = {}

    if C.MODE == "paper":
        log("📝 PAPER MODE — no wallet needed")
        for chain in C.ENABLED_CHAINS:
            addresses[chain] = "PAPER_MODE"
        return addresses

    # Check wallet status
    try:
        r = _onchainos("wallet", "status")
        data = _cli_data(r)
    except Exception as e:
        print("=" * 60)
        print("  FATAL: 无法检查 Agentic Wallet 状态")
        print(f"  错误: {e}")
        print()
        print("  请确保:")
        print("  1. onchainos CLI 已安装: onchainos --version")
        print("  2. 已登录钱包: onchainos wallet login <email>")
        print("=" * 60)
        sys.exit(1)

    if not data.get("loggedIn"):
        print("=" * 60)
        print("  FATAL: Agentic Wallet 未登录")
        print("  请先登录: onchainos wallet login <your-email>")
        print("=" * 60)
        sys.exit(1)

    # Get addresses per chain
    for chain in C.ENABLED_CHAINS:
        chain_idx = C.CHAIN_CONFIG.get(chain, {}).get("chain_index", "1")
        try:
            r2 = _onchainos("wallet", "addresses", "--chain", chain_idx)
            data2 = _cli_data(r2)
            addr = ""
            if isinstance(data2, dict):
                if chain == "ethereum":
                    eth_list = data2.get("ethereum", data2.get("evm", []))
                    if eth_list and isinstance(eth_list[0], dict):
                        addr = eth_list[0].get("address", "")
                    if not addr:
                        addr = data2.get("ethAddress", data2.get("address", ""))
                elif chain == "solana":
                    sol_list = data2.get("solana", [])
                    if sol_list and isinstance(sol_list[0], dict):
                        addr = sol_list[0].get("address", "")
                    if not addr:
                        addr = data2.get("solAddress", data2.get("address", ""))
            if isinstance(data2, list) and data2:
                addr = data2[0].get("address", "") if isinstance(data2[0], dict) else str(data2[0])
            if addr:
                addresses[chain] = addr
                log(f"  ✅ {chain} wallet: {addr[:8]}…{addr[-6:]}")
            else:
                log(f"  ⚠️ No {chain} address found — disabling {chain}")
                C.ENABLED_CHAINS = [c for c in C.ENABLED_CHAINS if c != chain]
        except Exception as e:
            log(f"  ⚠️ {chain} address error: {e}")

    if not addresses:
        print("  FATAL: 无法获取任何链的钱包地址")
        sys.exit(1)

    return addresses


def _save_config_to_disk():
    """Write current runtime config back to config.py."""
    config_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "config.py")
    try:
        with open(config_path, "r") as f:
            lines = f.read()

        import re
        replacements = {
            "MODE":           f'MODE              = "{C.MODE}"',
            "PAUSED":         f'PAUSED            = {C.PAUSED}',
            "STRATEGY_MODE":  f'STRATEGY_MODE     = "{C.STRATEGY_MODE}"',
            "TOTAL_BUDGET_USD": f'TOTAL_BUDGET_USD  = {C.TOTAL_BUDGET_USD}',
            "BUY_AMOUNT_USD": f'BUY_AMOUNT_USD    = {C.BUY_AMOUNT_USD}',
            "ENABLED_CHAINS": f'ENABLED_CHAINS    = {json.dumps(C.ENABLED_CHAINS)}',
        }
        for key, val in replacements.items():
            lines = re.sub(rf'^{key}\s*=.*$', val, lines, flags=re.MULTILINE)

        with open(config_path, "w") as f:
            f.write(lines)
    except Exception as e:
        print(f"  ⚠️ Could not save config: {e}")


def interactive_setup():
    """
    Setup using config.py defaults. When run inside a Claude Code skill,
    input() would block, so we use env vars or config defaults instead.
    Set env vars to override: RWA_STRATEGY_MODE, RWA_BUDGET, RWA_MODE, RWA_CHAINS
    """
    print()
    print("=" * 60)
    print("  🏛️  RWA Alpha — Real World Asset Intelligence")
    print("  ── Setup from config / env ──")
    print("=" * 60)
    print()

    # Strategy mode: env or config default
    mode_env = os.environ.get("RWA_STRATEGY_MODE", "").strip()
    if mode_env in ("yield_optimizer", "macro_trader", "full_alpha"):
        C.STRATEGY_MODE = mode_env
    # else keep config.py default

    # Budget
    budget_env = os.environ.get("RWA_BUDGET", "").strip()
    if budget_env:
        try:
            b = float(budget_env)
            if b >= 10:
                C.TOTAL_BUDGET_USD = b
        except ValueError:
            pass

    # Chains
    chains_env = os.environ.get("RWA_CHAINS", "").strip()
    if chains_env:
        C.ENABLED_CHAINS = [c.strip() for c in chains_env.split(",") if c.strip()]

    # Mode
    mode_run = os.environ.get("RWA_MODE", "").strip().lower()
    if mode_run in ("paper", "live"):
        C.MODE = mode_run

    # Buy amount
    buy_env = os.environ.get("RWA_BUY_AMOUNT", "").strip()
    if buy_env:
        try:
            a = float(buy_env)
            if 1 <= a <= C.TOTAL_BUDGET_USD:
                C.BUY_AMOUNT_USD = a
        except ValueError:
            pass

    C.PAUSED = False

    mode_map = {"yield_optimizer": "Yield Optimizer", "macro_trader": "Macro Trader", "full_alpha": "Full Alpha"}
    print(f"  Strategy: {mode_map.get(C.STRATEGY_MODE, C.STRATEGY_MODE)}")
    print(f"  Mode:     {C.MODE.upper()}")
    print(f"  Budget:   ${C.TOTAL_BUDGET_USD:,.0f} USDC")
    print(f"  Buy size: ${C.BUY_AMOUNT_USD:,.0f}")
    print(f"  Chains:   {', '.join(C.ENABLED_CHAINS)}")
    print()

    _save_config_to_disk()
    print("  Config saved to config.py")
    print()


def main():
    global WALLET_ADDRESSES, _bot_running

    print()
    print("=" * 60)
    print("  🏛️  RWA Alpha — Real World Asset Intelligence")
    print("=" * 60)
    print()

    # Check if first run or if env override requested
    first_run = C.PAUSED and C.MODE == "paper" and C.TOTAL_BUDGET_USD == 1000
    force_setup = os.environ.get("RWA_SETUP", "").strip().lower() == "1"
    if first_run or force_setup:
        interactive_setup()
    else:
        print(f"  Config: {C.STRATEGY_MODE} | {C.MODE} | chains={C.ENABLED_CHAINS}")
        print()

    # Wallet preflight
    WALLET_ADDRESSES = _wallet_preflight()

    # Load state
    load_positions()
    load_trades()

    session["start_ts"] = time.time()

    # Print config summary
    mode_map = {"yield_optimizer": "🛡️ Yield Optimizer", "macro_trader": "📊 Macro Trader", "full_alpha": "🚀 Full Alpha"}
    print()
    print("─" * 60)
    print("  📊 启动配置:")
    print("─" * 60)
    print(f"  策略模式:   {mode_map.get(C.STRATEGY_MODE, C.STRATEGY_MODE)}")
    print(f"  运行模式:   {C.MODE.upper()}")
    print(f"  总预算:     ${C.TOTAL_BUDGET_USD:,.0f} USDC")
    print(f"  单笔买入:   ${C.BUY_AMOUNT_USD:,.0f} USDC")
    print(f"  链:         {', '.join(C.ENABLED_CHAINS)}")
    print(f"  最大持仓:   {C.MAX_POSITIONS}")
    print(f"  最低置信度: {C.MIN_CONVICTION}")
    print(f"  最大回撤:   {C.MAX_DRAWDOWN_PCT}%")

    active_tokens = [
        sym for sym, t in C.RWA_UNIVERSE.items()
        if any(c in C.ENABLED_CHAINS for c in t.get("chains", []))
        and (C.STRATEGY_MODE != "yield_optimizer" or t.get("asset_backed"))
    ]
    from collections import Counter as _Counter
    _cat_counts = _Counter(C.RWA_UNIVERSE[s].get("category", "") for s in active_tokens)
    print(f"  代币池:     {len(active_tokens)} tokens loaded ({len(C.RWA_UNIVERSE)} universe)")
    print(f"  类别:       {', '.join(f'{c}({n})' for c, n in _cat_counts.most_common())}")
    print(f"  Dashboard:  http://localhost:{C.DASHBOARD_PORT}")
    print()
    print("─" * 60)
    print("  🚀 启动中… Ctrl+C 停止")
    print("─" * 60)
    print()

    # Start threads
    threads = [
        threading.Thread(target=perception_loop, daemon=True, name="perception"),
        threading.Thread(target=monitor_loop, daemon=True, name="monitor"),
        threading.Thread(target=start_dashboard, daemon=True, name="dashboard"),
    ]
    for t in threads:
        t.start()

    # Main thread — keep alive
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        print("\n  👋 Shutting down…")
        _bot_running = False
        save_positions()
        save_trades()
        save_signals()
        save_macro_events()
        save_yield_snapshots()
        print("  ✅ State saved. Goodbye!")


if __name__ == "__main__":
    main()
