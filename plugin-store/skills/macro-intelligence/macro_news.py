#!/usr/bin/env python3
"""
Macro Intelligence Skill v2.0 — Unified Macro Intelligence Feed
Merges perception layers from RWA Alpha + TG Intel.
Reads news from 9+ sources, classifies macro events, scores sentiment,
generates AI insights, exposes signals via HTTP API + WebSocket push.
No trading logic — intelligence only.
"""
from __future__ import annotations

import hashlib
import json
import os
import queue
import re
import sys
import time
import threading
import traceback
import xml.etree.ElementTree as ET
from collections import defaultdict
from datetime import datetime, timezone
from http.server import HTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
from urllib.parse import parse_qs, urlparse
from urllib.request import Request, urlopen

import config as C

_VERSION = "2.0.0"

# ═══════════════════════════════════════════════════════════════════════
#  GLOBAL STATE
# ═══════════════════════════════════════════════════════════════════════
_state_lock = threading.Lock()

_signals: list[dict] = []               # Unified signal list
_dedup_hashes: dict[str, float] = {}    # hash -> timestamp
_reputation: dict[str, dict] = {}       # sender_id -> {score, last_ts, hits}
_polymarket: list[dict] = []            # Latest Polymarket data
_source_status: dict[str, float] = {}   # source -> last_success_ts
_fear_greed: dict = {}                  # Latest Fear & Greed Index
_fred_indicators: dict = {}             # Latest FRED macro indicators
_opennews_ws_alive = False              # True while WebSocket is connected
_price_tickers: dict = {}               # Latest price tickers {symbol: {price, change_pct, label}}
_stats = {
    "messages_processed": 0,
    "signals_produced": 0,
    "start_ts": 0,
    "news_fetches": 0,
    "tg_messages": 0,
    "llm_calls": 0,
}

# ── WebSocket server state ──
_ws_clients: set = set()
_ws_client_filters: dict = {}          # websocket -> {direction, min_mag, affects}
_ws_broadcast_queue: queue.Queue = queue.Queue(maxsize=1000)

# ── Signal accuracy tracking ──
_accuracy_pending: list[dict] = []     # [{signal_ts, event_type, direction, btc_price, eth_price, check_at: [ts,...]}]
_accuracy_results: dict = {}           # event_type -> {hits, misses, checks}

# ── Trend detection ──
_recent_event_types: list[tuple[float, str]] = []  # [(ts, event_type)]

# ── Fuzzy dedup ──
_recent_texts: list[tuple[float, str]] = []  # [(ts, text)] last 100

# ── RSS state ──
_rss_seen_guids: dict[str, set] = {}   # feed_url -> set of seen guids
_rss_last_poll: dict[str, float] = {}  # feed_url -> last poll ts

# ── CryptoPanic state ──
_cryptopanic_last_ts: float = 0

# ── Signal votes ──
_signal_votes: dict[str, int] = {}     # signal_key -> net votes

# Compiled regex caches
_macro_regex: dict[str, list[re.Pattern]] = {}
_noise_regex: list[re.Pattern] = []

_BASE_DIR = Path(__file__).parent
_STATE_DIR = _BASE_DIR / C.STATE_DIR

# ── OpenNews article buffer (dual-store: raw articles + signals coexist) ──
_opennews_articles: list[dict] = []
_opennews_dedup: set[str] = set()
_opennews_article_index: dict[str, int] = {}
_opennews_source_status: dict[str, float] = {}

# ═══════════════════════════════════════════════════════════════════════
#  LOGGING & PERSISTENCE
# ═══════════════════════════════════════════════════════════════════════
def _log(msg: str, level: str = "INFO"):
    ts = datetime.now().strftime("%H:%M:%S")
    print(f"[{ts}] [{level}] {msg}", flush=True)

def _save_state():
    _STATE_DIR.mkdir(exist_ok=True)
    try:
        with _state_lock:
            data = {
                "signals": _signals[-C.MAX_SIGNALS_KEPT:],
                "dedup_hashes": _dedup_hashes,
                "reputation": _reputation,
                "polymarket": _polymarket,
                "stats": _stats,
                "source_status": _source_status,
                "finnhub_last_id": _finnhub_last_id,
                "accuracy_results": _accuracy_results,
                "accuracy_pending": _accuracy_pending[-200:],
                "signal_votes": _signal_votes,
                "opennews_articles": _opennews_articles[-C.OPENNEWS_MAX_ARTICLES:],
                "opennews_source_status": _opennews_source_status,
            }
        with open(_STATE_DIR / "state.json", "w") as f:
            json.dump(data, f, default=str)
    except Exception as e:
        _log(f"save_state error: {e}", "WARN")

def _load_state():
    global _signals, _dedup_hashes, _reputation, _polymarket, _stats, _source_status, _finnhub_last_id
    global _accuracy_results, _accuracy_pending, _signal_votes
    global _opennews_articles, _opennews_dedup, _opennews_source_status
    p = _STATE_DIR / "state.json"
    if not p.exists():
        return
    try:
        with open(p) as f:
            data = json.load(f)
        with _state_lock:
            _signals = data.get("signals", [])
            _dedup_hashes = data.get("dedup_hashes", {})
            _reputation = data.get("reputation", {})
            _polymarket = data.get("polymarket", [])
            saved_stats = data.get("stats", {})
            for k in _stats:
                if k in saved_stats:
                    _stats[k] = saved_stats[k]
            _source_status = data.get("source_status", {})
            _finnhub_last_id = data.get("finnhub_last_id", 0)
            _accuracy_results.update(data.get("accuracy_results", {}))
            _accuracy_pending.extend(data.get("accuracy_pending", []))
            _signal_votes.update(data.get("signal_votes", {}))
            # Restore OpenNews article buffer
            _opennews_articles = data.get("opennews_articles", [])
            _opennews_source_status = data.get("opennews_source_status", {})
            _opennews_dedup.clear()
            _opennews_dedup.update(a["id"] for a in _opennews_articles if "id" in a)
            _rebuild_opennews_index()
        # Backfill token_impacts for legacy signals
        backfilled = 0
        for sig in _signals:
            if "token_impacts" not in sig:
                sig["token_impacts"] = _compute_token_impacts(
                    sig.get("event_type", ""), sig.get("direction", "neutral"),
                    sig.get("magnitude", 0.5), sig.get("tokens", []),
                )
                backfilled += 1
        _log(f"Loaded state: {len(_signals)} signals, {len(_reputation)} senders, {len(_opennews_articles)} opennews articles"
             f"{f' (backfilled {backfilled} token impacts)' if backfilled else ''}")
    except Exception as e:
        _log(f"load_state error: {e}", "WARN")

# ═══════════════════════════════════════════════════════════════════════
#  HTTP HELPER
# ═══════════════════════════════════════════════════════════════════════
def _http_get_json(url: str, timeout: int = 10) -> dict | list:
    try:
        req = Request(url, headers={
            "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                          "AppleWebKit/537.36 (KHTML, like Gecko) "
                          "Chrome/125.0.0.0 Safari/537.36",
            "Accept": "application/json, text/html, */*",
            "Accept-Language": "en-US,en;q=0.9",
        })
        with urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read().decode())
    except Exception:
        return {}

# ═══════════════════════════════════════════════════════════════════════
#  OPENNEWS ARTICLE BUFFER — Raw article storage for dashboard tab
# ═══════════════════════════════════════════════════════════════════════
_HTML_TAG_RE = re.compile(r"<[^>]+>")

def _strip_html(s: str) -> str:
    return _HTML_TAG_RE.sub("", s).strip()

def _clean_ai_rating(ai: dict | None) -> dict | None:
    if not isinstance(ai, dict):
        return ai
    for key in ("summary", "enSummary"):
        if key in ai and isinstance(ai[key], str):
            ai[key] = _strip_html(ai[key])
    return ai

def _normalize_opennews_article(params: dict) -> dict | None:
    """Standardize a raw WS or REST article into our schema."""
    news_id = str(params.get("id", params.get("newsId", "")))
    text = _strip_html(params.get("text", params.get("title", "")))
    if not news_id or not text:
        return None

    ai_rating = params.get("aiRating")
    if ai_rating and not isinstance(ai_rating, dict):
        ai_rating = None

    coins = params.get("coins", [])
    if not isinstance(coins, list):
        coins = []

    return {
        "id": news_id,
        "text": text,
        "newsType": params.get("newsType", "unknown"),
        "engineType": params.get("engineType", "news"),
        "link": params.get("link", ""),
        "coins": coins,
        "aiRating": _clean_ai_rating(ai_rating),
        "ts": params.get("ts", int(time.time() * 1000)),
        "_received_ts": time.time(),
    }


def _ingest_opennews_article(article: dict) -> bool:
    """Dedup, append to buffer, update source status, queue for broadcast.
    Returns True if article was new."""
    aid = article["id"]
    with _state_lock:
        if aid in _opennews_dedup:
            return False
        _opennews_dedup.add(aid)
        _opennews_articles.append(article)

        # Update source status
        src = article.get("newsType", "unknown")
        _opennews_source_status[src] = time.time()

        # Trim buffer
        if len(_opennews_articles) > C.OPENNEWS_MAX_ARTICLES:
            removed = _opennews_articles[:len(_opennews_articles) - C.OPENNEWS_MAX_ARTICLES]
            del _opennews_articles[:len(_opennews_articles) - C.OPENNEWS_MAX_ARTICLES]
            for r in removed:
                _opennews_dedup.discard(r.get("id", ""))
            _rebuild_opennews_index()
        else:
            _opennews_article_index[aid] = len(_opennews_articles) - 1

    # Queue for WS broadcast (tagged so broadcast worker can distinguish)
    try:
        _ws_broadcast_queue.put_nowait({
            "_opennews_article": True,
            "type": "opennews_article",
            "data": article,
        })
    except queue.Full:
        pass
    return True


def _update_opennews_ai(news_id: str, ai_rating: dict):
    """Update an existing article's AI rating in-place and broadcast."""
    ai_rating = _clean_ai_rating(ai_rating)
    with _state_lock:
        idx = _opennews_article_index.get(news_id)
        if idx is not None and idx < len(_opennews_articles):
            _opennews_articles[idx]["aiRating"] = ai_rating
        else:
            for i in range(len(_opennews_articles) - 1, -1, -1):
                if _opennews_articles[i].get("id") == news_id:
                    _opennews_articles[i]["aiRating"] = ai_rating
                    break

    try:
        _ws_broadcast_queue.put_nowait({
            "_opennews_article": True,
            "type": "opennews_ai_update",
            "data": {"id": news_id, "aiRating": ai_rating},
        })
    except queue.Full:
        pass


def _rebuild_opennews_index():
    """Rebuild the id->index lookup. Call under _state_lock."""
    global _opennews_article_index
    _opennews_article_index = {a["id"]: i for i, a in enumerate(_opennews_articles) if "id" in a}


# ═══════════════════════════════════════════════════════════════════════
#  OPENNEWS METRICS ENGINE — Analytics for the OpenNews dashboard tab
# ═══════════════════════════════════════════════════════════════════════
def _opennews_windowed_articles(window_sec: int = 3600) -> list[dict]:
    cutoff = time.time() - window_sec
    with _state_lock:
        return [a for a in _opennews_articles if a.get("_received_ts", 0) >= cutoff]

def _opennews_score_distribution(articles: list[dict]) -> dict[str, int]:
    buckets = {"0-20": 0, "20-40": 0, "40-60": 0, "60-80": 0, "80-100": 0}
    for a in articles:
        ai = a.get("aiRating")
        if not isinstance(ai, dict):
            continue
        score = ai.get("score", 0)
        if score < 20:    buckets["0-20"] += 1
        elif score < 40:  buckets["20-40"] += 1
        elif score < 60:  buckets["40-60"] += 1
        elif score < 80:  buckets["60-80"] += 1
        else:             buckets["80-100"] += 1
    return buckets

def _opennews_signal_ratios(articles: list[dict]) -> dict:
    counts = {"long": 0, "short": 0, "neutral": 0}
    for a in articles:
        ai = a.get("aiRating")
        if not isinstance(ai, dict):
            continue
        sig = ai.get("signal", "neutral")
        if sig in counts:
            counts[sig] += 1
    total = sum(counts.values())
    if total == 0:
        return {"long": 0, "short": 0, "neutral": 0, "total": 0}
    return {
        "long": round(counts["long"] / total * 100, 1),
        "short": round(counts["short"] / total * 100, 1),
        "neutral": round(counts["neutral"] / total * 100, 1),
        "total": total,
    }

def _opennews_top_coins(articles: list[dict], limit: int = 15) -> list[dict]:
    coin_data: dict[str, dict] = {}
    for a in articles:
        coins = a.get("coins", [])
        ai = a.get("aiRating")
        score = ai.get("score", 0) if isinstance(ai, dict) else 0
        signal = ai.get("signal", "neutral") if isinstance(ai, dict) else "neutral"
        for c in coins:
            sym = c.get("symbol", "") if isinstance(c, dict) else str(c)
            if not sym:
                continue
            sym = sym.upper()
            if sym not in coin_data:
                coin_data[sym] = {"count": 0, "total_score": 0, "signals": defaultdict(int)}
            coin_data[sym]["count"] += 1
            coin_data[sym]["total_score"] += score
            coin_data[sym]["signals"][signal] += 1
    result = []
    for sym, d in sorted(coin_data.items(), key=lambda x: x[1]["count"], reverse=True)[:limit]:
        dominant = max(d["signals"], key=d["signals"].get) if d["signals"] else "neutral"
        result.append({
            "symbol": sym, "count": d["count"],
            "avg_score": round(d["total_score"] / d["count"], 1) if d["count"] else 0,
            "dominant_signal": dominant,
        })
    return result

def _opennews_engine_breakdown(articles: list[dict]) -> dict[str, int]:
    counts = {et: 0 for et in C.OPENNEWS_ENGINE_TYPES}
    for a in articles:
        et = a.get("engineType", "news")
        counts[et] = counts.get(et, 0) + 1
    return counts

def _opennews_velocity(articles: list[dict], window_sec: int = 3600) -> dict:
    if not articles:
        return {"overall": 0, "by_engine": {et: 0 for et in C.OPENNEWS_ENGINE_TYPES}}
    hours = max(window_sec / 3600, 0.01)
    by_engine: dict[str, int] = defaultdict(int)
    for a in articles:
        by_engine[a.get("engineType", "news")] += 1
    return {
        "overall": round(len(articles) / hours, 1),
        "by_engine": {et: round(by_engine.get(et, 0) / hours, 1) for et in C.OPENNEWS_ENGINE_TYPES},
    }

def _opennews_high_score_rate(articles: list[dict]) -> float:
    scored = [a for a in articles if isinstance(a.get("aiRating"), dict)]
    if not scored:
        return 0
    high = sum(1 for a in scored if a["aiRating"].get("score", 0) >= C.OPENNEWS_HIGH_SCORE_THRESHOLD)
    return round(high / len(scored) * 100, 1)

def _opennews_avg_score(articles: list[dict]) -> float:
    scored = [a for a in articles if isinstance(a.get("aiRating"), dict) and a["aiRating"].get("score")]
    if not scored:
        return 0
    return round(sum(a["aiRating"]["score"] for a in scored) / len(scored), 1)

def _opennews_predictions(articles: list[dict]) -> list[dict]:
    preds = [a for a in articles if a.get("engineType") == "prediction"]
    preds.sort(key=lambda x: x.get("_received_ts", 0), reverse=True)
    return preds[:10]

def _opennews_listings(articles: list[dict]) -> list[dict]:
    items = [a for a in articles if a.get("engineType") == "listing"]
    items.sort(key=lambda x: x.get("_received_ts", 0), reverse=True)
    return items[:10]

def _opennews_onchain(articles: list[dict]) -> list[dict]:
    items = [a for a in articles if a.get("engineType") == "onchain"]
    items.sort(key=lambda x: x.get("_received_ts", 0), reverse=True)
    return items[:10]

def _opennews_market_anomalies(articles: list[dict]) -> list[dict]:
    items = [a for a in articles if a.get("engineType") == "market"]
    items.sort(key=lambda x: x.get("_received_ts", 0), reverse=True)
    return items[:10]

def _opennews_meme(articles: list[dict]) -> list[dict]:
    items = [a for a in articles if a.get("engineType") == "meme"]
    items.sort(key=lambda x: x.get("_received_ts", 0), reverse=True)
    return items[:10]

def _opennews_source_health() -> list[dict]:
    now = time.time()
    with _state_lock:
        sources = dict(_opennews_source_status)
    result = []
    for name, last_ts in sorted(sources.items()):
        ago_sec = now - last_ts
        if ago_sec < C.OPENNEWS_SOURCE_STALE:
            status = "active"
        elif ago_sec < C.OPENNEWS_SOURCE_DEAD:
            status = "stale"
        else:
            status = "dead"
        result.append({"name": name, "status": status, "last_seen_ago": round(ago_sec)})
    return result

def compute_opennews_metrics(window_sec: int = 3600) -> dict:
    articles = _opennews_windowed_articles(window_sec)
    return {
        "window_sec": window_sec,
        "article_count": len(articles),
        "avg_score": _opennews_avg_score(articles),
        "score_distribution": _opennews_score_distribution(articles),
        "signal_ratios": _opennews_signal_ratios(articles),
        "top_coins": _opennews_top_coins(articles),
        "engine_breakdown": _opennews_engine_breakdown(articles),
        "velocity": _opennews_velocity(articles, window_sec),
        "high_score_rate": _opennews_high_score_rate(articles),
    }

def compute_opennews_intelligence(window_sec: int = 3600) -> dict:
    articles = _opennews_windowed_articles(window_sec)
    return {
        "predictions": _opennews_predictions(articles),
        "listings": _opennews_listings(articles),
        "onchain": _opennews_onchain(articles),
        "market_anomalies": _opennews_market_anomalies(articles),
        "meme": _opennews_meme(articles),
    }

def _filter_opennews_articles(
    engine: str = "", signal: str = "", min_score: int = 0,
    coin: str = "", q: str = "", limit: int = 100,
) -> list[dict]:
    with _state_lock:
        pool = list(_opennews_articles)
    result = []
    q_lower = q.lower() if q else ""
    coin_upper = coin.upper() if coin else ""
    for a in reversed(pool):
        if engine and a.get("engineType") != engine:
            continue
        ai = a.get("aiRating")
        if signal and (not isinstance(ai, dict) or ai.get("signal") != signal):
            continue
        if min_score > 0:
            if not isinstance(ai, dict) or (ai.get("score", 0) or 0) < min_score:
                continue
        if coin_upper:
            coins = a.get("coins", [])
            coin_syms = [(c.get("symbol", "") if isinstance(c, dict) else str(c)).upper() for c in coins]
            if coin_upper not in coin_syms:
                continue
        if q_lower:
            text = (a.get("text", "") or "").lower()
            en_summary = ""
            if isinstance(ai, dict):
                en_summary = (ai.get("enSummary", "") or "").lower()
            if q_lower not in text and q_lower not in en_summary:
                continue
        result.append(a)
        if len(result) >= limit:
            break
    return result


# ═══════════════════════════════════════════════════════════════════════
#  NEWS FETCHERS
# ═══════════════════════════════════════════════════════════════════════
def fetch_news_headlines() -> list[dict]:
    """Fetch latest headlines from NewsNow sources."""
    all_items = []
    for source in C.NEWS_SOURCES:
        data = _http_get_json(f"{C.NEWSNOW_BASE}?id={source}", timeout=8)
        items = data.get("items", data.get("data", [])) if isinstance(data, dict) else []
        for item in items[:15]:
            title = item.get("title", item.get("name", ""))
            if title:
                all_items.append({
                    "title": title,
                    "url": item.get("url", item.get("link", "")),
                    "source": source,
                    "ts": item.get("pubDate", item.get("time", "")),
                })
    return all_items

def fetch_polymarket_signals() -> list[dict]:
    """Fetch Polymarket prediction market data via events endpoint with keyword filtering."""
    _MACRO_KEYWORDS_PM = [
        "fed", "rate cut", "interest rate", "cpi", "inflation", "gdp",
        "tariff", "recession", "gold", "oil", "treasury", "fomc",
        "employment", "job", "bitcoin", "crypto", "economy", "china",
    ]
    results = []
    seen_questions = set()
    # Paginate through events to find macro-relevant ones
    for offset in (0, 100, 200):
        url = f"{C.POLYMARKET_BASE.replace('/markets', '/events')}?active=true&closed=false&limit=100&offset={offset}"
        data = _http_get_json(url, timeout=8)
        if not isinstance(data, list):
            continue
        for event in data:
            title = (event.get("title", "") or "").lower()
            if not any(kw in title for kw in _MACRO_KEYWORDS_PM):
                continue
            for m in event.get("markets", []):
                if not isinstance(m, dict):
                    continue
                question = m.get("question", m.get("title", ""))
                if not question or question in seen_questions:
                    continue
                seen_questions.add(question)
                prices = m.get("outcomePrices", "")
                prob = 0.5
                if isinstance(prices, str):
                    try:
                        prices = json.loads(prices)
                    except Exception:
                        prices = []
                if isinstance(prices, list) and prices:
                    try:
                        prob = float(prices[0])
                    except (ValueError, IndexError):
                        pass
                results.append({
                    "question": question,
                    "probability": prob,
                    "category": event.get("slug", ""),
                    "volume": m.get("volume", 0),
                })
    return results

def fetch_fear_greed() -> dict:
    """Fetch Crypto Fear & Greed Index from alternative.me."""
    data = _http_get_json("https://api.alternative.me/fng/?limit=7", timeout=8)
    if not isinstance(data, dict) or "data" not in data:
        return {}
    entries = data["data"]
    if not entries:
        return {}
    current = entries[0]
    history = [{"value": int(e.get("value", 0)),
                "label": e.get("value_classification", ""),
                "ts": int(e.get("timestamp", 0))} for e in entries]
    return {
        "value": int(current.get("value", 0)),
        "label": current.get("value_classification", ""),
        "ts": int(current.get("timestamp", 0)),
        "history": history,
    }

# ═══════════════════════════════════════════════════════════════════════
#  FINNHUB NEWS FETCHER
# ═══════════════════════════════════════════════════════════════════════
_finnhub_last_id: int = 0  # Track last seen article ID to avoid re-processing

def fetch_finnhub_news() -> list[dict]:
    """Fetch market news from Finnhub. Uses minId for incremental fetching."""
    global _finnhub_last_id
    if not C.FINNHUB_API_KEY:
        return []
    all_articles = []
    for cat in C.FINNHUB_CATEGORIES:
        url = f"{C.FINNHUB_BASE}/news?category={cat}&token={C.FINNHUB_API_KEY}"
        if _finnhub_last_id:
            url += f"&minId={_finnhub_last_id}"
        data = _http_get_json(url, timeout=10)
        if not isinstance(data, list):
            continue
        for item in data:
            article_id = item.get("id", 0)
            if article_id and article_id > _finnhub_last_id:
                _finnhub_last_id = article_id
            headline = item.get("headline", "")
            if headline:
                all_articles.append({
                    "title": headline,
                    "source": item.get("source", "finnhub"),
                    "url": item.get("url", ""),
                    "ts": item.get("datetime", 0),
                })
    return all_articles

# ═══════════════════════════════════════════════════════════════════════
#  FRED MACRO INDICATORS FETCHER
# ═══════════════════════════════════════════════════════════════════════
# Thresholds for significant change detection (emit signals)
_FRED_CHANGE_THRESHOLDS = {
    "FEDFUNDS": 0.10,   # 10 bps change in Fed Funds Rate
    "CPIAUCSL": 0.3,    # 0.3% CPI change
    "GDP":      0.5,    # 0.5% GDP change
    "UNRATE":   0.2,    # 0.2% unemployment change
    "T10Y2Y":   0.15,   # 15 bps spread change
    "DGS10":    0.15,   # 15 bps yield change
}

def fetch_fred_indicators() -> dict:
    """Fetch latest macro indicators from FRED. Returns dict of series data with change detection."""
    if not C.FRED_API_KEY:
        return {}
    results = {}
    for series_id, label in C.FRED_SERIES.items():
        url = (f"{C.FRED_BASE}/series/observations"
               f"?series_id={series_id}&api_key={C.FRED_API_KEY}"
               f"&file_type=json&limit=2&sort_order=desc")
        data = _http_get_json(url, timeout=10)
        if not isinstance(data, dict):
            continue
        obs = data.get("observations", [])
        if not obs:
            continue
        try:
            current_val = float(obs[0].get("value", "0"))
        except (ValueError, TypeError):
            continue
        current_date = obs[0].get("date", "")
        prev_val = None
        change = None
        if len(obs) > 1:
            try:
                prev_val = float(obs[1].get("value", "0"))
                change = round(current_val - prev_val, 4)
            except (ValueError, TypeError):
                pass
        results[series_id] = {
            "value": current_val,
            "date": current_date,
            "label": label,
            "prev_value": prev_val,
            "change": change,
        }
    return results

# ═══════════════════════════════════════════════════════════════════════
#  6551.io OPENNEWS REST FALLBACK
# ═══════════════════════════════════════════════════════════════════════
def fetch_opennews_rest() -> list[dict]:
    """Fetch articles via 6551.io REST /open/news_search POST.
    Dual-store: raw articles go to _opennews_articles buffer,
    high-score articles returned as signal candidates."""
    if not C.OPENNEWS_TOKEN:
        return []
    import urllib.error
    engine_filter = {et: [] for et in C.OPENNEWS_ENGINE_TYPES}
    body = json.dumps({"engineTypes": engine_filter, "limit": 50, "hasCoin": False}).encode()
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {C.OPENNEWS_TOKEN}",
    }
    req = Request(f"{C.OPENNEWS_API_BASE}/open/news_search",
                  data=body, headers=headers, method="POST")
    try:
        with urlopen(req, timeout=15) as resp:
            data = json.loads(resp.read())
    except Exception as e:
        _log(f"OpenNews REST error: {e}", "WARN")
        return []
    items = data.get("data", [])
    if not isinstance(items, list):
        return []
    if items:
        with _state_lock:
            _source_status["opennews_rest"] = time.time()
    results = []
    for item in items:
        # Dual-store: ingest every article into raw buffer
        article = _normalize_opennews_article(item)
        if article:
            _ingest_opennews_article(article)
        # High-score articles also become signal candidates
        score = item.get("aiRating", {}).get("score", 0) if isinstance(item.get("aiRating"), dict) else 0
        if score < C.OPENNEWS_MIN_SCORE:
            continue
        text = (item.get("enSummary") or item.get("summary") or
                item.get("title") or item.get("text", ""))
        if not text:
            continue
        results.append({
            "text": _strip_html(text),
            "source": item.get("newsType", "opennews"),
            "score": score,
            "signal": item.get("aiRating", {}).get("signal", "neutral") if isinstance(item.get("aiRating"), dict) else "neutral",
        })
    return results

# ═══════════════════════════════════════════════════════════════════════
#  PRICE TICKER FETCHER
# ═══════════════════════════════════════════════════════════════════════
def fetch_price_tickers() -> dict:
    """Fetch live prices for dashboard ticker bar.
    Finnhub for stocks/ETFs (SPY, GLD, SLV), CoinGecko for crypto (BTC, ETH).
    """
    results = {}

    # Finnhub stock/ETF quotes
    if C.FINNHUB_API_KEY:
        for symbol, label in C.FINNHUB_PRICE_SYMBOLS.items():
            data = _http_get_json(
                f"{C.FINNHUB_BASE}/quote?symbol={symbol}&token={C.FINNHUB_API_KEY}",
                timeout=5,
            )
            if isinstance(data, dict) and data.get("c"):
                price = data["c"]           # Current price
                prev_close = data.get("pc", price)  # Previous close
                change_pct = ((price - prev_close) / prev_close * 100) if prev_close else 0
                results[symbol] = {
                    "price": price,
                    "change_pct": round(change_pct, 2),
                    "label": label,
                }

    # CoinGecko crypto quotes (free, no key)
    cg_data = _http_get_json(
        "https://api.coingecko.com/api/v3/simple/price"
        "?ids=bitcoin,ethereum&vs_currencies=usd&include_24hr_change=true",
        timeout=5,
    )
    if isinstance(cg_data, dict):
        if "bitcoin" in cg_data:
            results["BTC"] = {
                "price": cg_data["bitcoin"].get("usd", 0),
                "change_pct": round(cg_data["bitcoin"].get("usd_24h_change", 0), 2),
                "label": "BTC",
            }
        if "ethereum" in cg_data:
            results["ETH"] = {
                "price": cg_data["ethereum"].get("usd", 0),
                "change_pct": round(cg_data["ethereum"].get("usd_24h_change", 0), 2),
                "label": "ETH",
            }

    return results

# ═══════════════════════════════════════════════════════════════════════
#  CRYPTOPANIC NEWS FETCHER
# ═══════════════════════════════════════════════════════════════════════
def fetch_cryptopanic_news() -> list[dict]:
    """Fetch news from CryptoPanic API with community vote data."""
    if not C.CRYPTOPANIC_TOKEN:
        return []
    filt = C.CRYPTOPANIC_FILTER
    url = f"{C.CRYPTOPANIC_BASE}?auth_token={C.CRYPTOPANIC_TOKEN}&public=true"
    if filt and filt != "all":
        url += f"&filter={filt}"
    data = _http_get_json(url, timeout=10)
    if not isinstance(data, dict):
        return []
    results_list = data.get("results", [])
    if not isinstance(results_list, list):
        return []
    articles = []
    for item in results_list[:20]:
        title = item.get("title", "")
        if not title:
            continue
        # Parse source timestamp
        created_at = item.get("created_at", "")
        src_ts = 0.0
        if created_at:
            try:
                # ISO 8601: "2025-04-17T10:30:00Z"
                from datetime import datetime as _dt
                dt = _dt.fromisoformat(created_at.replace("Z", "+00:00"))
                src_ts = dt.timestamp()
            except Exception:
                pass
        # Extract community votes
        votes = item.get("votes", {})
        positive = votes.get("positive", 0) if isinstance(votes, dict) else 0
        negative = votes.get("negative", 0) if isinstance(votes, dict) else 0
        important = votes.get("important", 0) if isinstance(votes, dict) else 0
        source_info = item.get("source", {})
        source_name = source_info.get("title", "cryptopanic") if isinstance(source_info, dict) else "cryptopanic"
        articles.append({
            "title": title,
            "source": source_name,
            "source_ts": src_ts,
            "votes_positive": positive,
            "votes_negative": negative,
            "votes_important": important,
        })
    return articles


# ═══════════════════════════════════════════════════════════════════════
#  RSS / ATOM FEED FETCHER
# ═══════════════════════════════════════════════════════════════════════
def _parse_rss_date(date_str: str) -> float:
    """Try common RSS/Atom date formats. Returns unix timestamp or 0."""
    formats = [
        "%a, %d %b %Y %H:%M:%S %z",      # RSS 2.0: "Thu, 17 Apr 2025 10:30:00 +0000"
        "%a, %d %b %Y %H:%M:%S %Z",       # RSS 2.0 with TZ name
        "%Y-%m-%dT%H:%M:%S%z",             # Atom / ISO 8601
        "%Y-%m-%dT%H:%M:%SZ",              # Atom without offset
        "%Y-%m-%d %H:%M:%S",               # Fallback
    ]
    for fmt in formats:
        try:
            dt = datetime.strptime(date_str.strip(), fmt)
            if dt.tzinfo is None:
                dt = dt.replace(tzinfo=timezone.utc)
            return dt.timestamp()
        except (ValueError, TypeError):
            continue
    return 0.0


def fetch_rss_feed(feed_config: dict) -> list[dict]:
    """Fetch and parse an RSS 2.0 or Atom feed. Returns list of articles."""
    url = feed_config.get("url", "")
    label = feed_config.get("label", url[:40])
    if not url:
        return []

    try:
        req = Request(url, headers={
            "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                          "AppleWebKit/537.36 (KHTML, like Gecko) "
                          "Chrome/125.0.0.0 Safari/537.36",
        })
        with urlopen(req, timeout=10) as resp:
            raw = resp.read()
    except Exception as e:
        _log(f"RSS fetch error ({label}): {e}", "WARN")
        return []

    try:
        root = ET.fromstring(raw)
    except ET.ParseError as e:
        _log(f"RSS parse error ({label}): {e}", "WARN")
        return []

    # Initialize seen guids for this feed
    if url not in _rss_seen_guids:
        _rss_seen_guids[url] = set()
    seen = _rss_seen_guids[url]

    articles = []
    ns = {"atom": "http://www.w3.org/2005/Atom"}

    # Try RSS 2.0 format first
    items = root.findall(".//item")
    if items:
        for item in items[:20]:
            title_el = item.find("title")
            title = title_el.text.strip() if title_el is not None and title_el.text else ""
            guid_el = item.find("guid")
            link_el = item.find("link")
            guid = (guid_el.text if guid_el is not None and guid_el.text else
                    link_el.text if link_el is not None and link_el.text else title)
            if not title or guid in seen:
                continue
            seen.add(guid)
            pub_el = item.find("pubDate")
            src_ts = _parse_rss_date(pub_el.text) if pub_el is not None and pub_el.text else 0.0
            articles.append({"title": title, "source": label, "source_ts": src_ts})
    else:
        # Try Atom format
        entries = root.findall("atom:entry", ns) or root.findall("entry")
        for entry in (entries or [])[:20]:
            title_el = entry.find("atom:title", ns) or entry.find("title")
            title = title_el.text.strip() if title_el is not None and title_el.text else ""
            id_el = entry.find("atom:id", ns) or entry.find("id")
            guid = id_el.text if id_el is not None and id_el.text else title
            if not title or guid in seen:
                continue
            seen.add(guid)
            pub_el = (entry.find("atom:published", ns) or entry.find("published") or
                      entry.find("atom:updated", ns) or entry.find("updated"))
            src_ts = _parse_rss_date(pub_el.text) if pub_el is not None and pub_el.text else 0.0
            articles.append({"title": title, "source": label, "source_ts": src_ts})

    # Cap seen guids per feed
    if len(seen) > 500:
        _rss_seen_guids[url] = set(list(seen)[-500:])

    return articles


# ═══════════════════════════════════════════════════════════════════════
#  NOISE FILTER (Telegram only)
# ═══════════════════════════════════════════════════════════════════════
def _count_emoji(text: str) -> int:
    # Count chars in common emoji ranges
    count = 0
    for ch in text:
        cp = ord(ch)
        if (0x1F600 <= cp <= 0x1F64F or 0x1F300 <= cp <= 0x1F5FF or
            0x1F680 <= cp <= 0x1F6FF or 0x1F900 <= cp <= 0x1F9FF or
            0x2600 <= cp <= 0x26FF or 0x2700 <= cp <= 0x27BF or
            0xFE00 <= cp <= 0xFE0F or 0x200D == cp):
            count += 1
    return count

def _is_noise(text: str, sender_id: str = "", sender_name: str = "",
              is_reply: bool = False, is_forward_from_bot: bool = False,
              is_deep_reply: bool = False) -> bool:
    """Return True if message should be dropped as noise."""
    # VIP bypass
    if sender_name in C.VIP_SENDERS or sender_id in C.VIP_SENDERS:
        return False

    stripped = text.strip()

    # Min length
    if len(stripped) < C.NOISE_MIN_LENGTH:
        return True

    # Bot forward
    if C.NOISE_SKIP_BOT_FORWARDS and is_forward_from_bot:
        return True

    # Deep reply
    if C.NOISE_SKIP_DEEP_REPLIES and is_deep_reply:
        return True

    # Emoji ratio
    emoji_count = _count_emoji(stripped)
    if len(stripped) > 0 and emoji_count / len(stripped) > C.NOISE_MAX_EMOJI_RATIO:
        return True

    # Pattern match
    for pat in _noise_regex:
        if pat.search(stripped):
            return True

    return False

# ═══════════════════════════════════════════════════════════════════════
#  DEDUP (cross-source, MD5 hash, time window)
# ═══════════════════════════════════════════════════════════════════════
def _dedup_hash(text: str) -> str:
    n = C.DEDUP_SIMILARITY_CHARS
    snippet = re.sub(r'\s+', ' ', text[:n].lower().strip())
    return hashlib.md5(snippet.encode()).hexdigest()[:12]

def _is_duplicate(text: str) -> bool:
    h = _dedup_hash(text)
    now = time.time()
    window = C.DEDUP_WINDOW_HOURS * 3600
    with _state_lock:
        # Clean old hashes
        expired = [k for k, ts in _dedup_hashes.items() if now - ts > window]
        for k in expired:
            del _dedup_hashes[k]
        if h in _dedup_hashes:
            return True
        _dedup_hashes[h] = now
    return False

# ═══════════════════════════════════════════════════════════════════════
#  3-LAYER CLASSIFIER
# ═══════════════════════════════════════════════════════════════════════
def _match_macro_keywords(text: str) -> tuple[str, float] | None:
    """Layer 1: Regex keyword match. Returns (event_type, confidence) or None."""
    text_lower = text.lower()
    for event_type, patterns in _macro_regex.items():
        for pat in patterns:
            if pat.search(text_lower) or pat.search(text):
                return (event_type, 0.85)
    return None

def _match_classifier_rules(text: str) -> dict | None:
    """Layer 1b: TG-style classifier rules. Returns matched rule dict or None."""
    text_lower = text.lower()
    for rule in C.CLASSIFIER_RULES:
        # Check keywords_any (at least one must match)
        any_match = False
        for kw in rule["keywords_any"]:
            if kw.lower() in text_lower:
                any_match = True
                break
        if not any_match:
            continue
        # Check keywords_not (none must match)
        not_match = False
        for kw in rule.get("keywords_not", []):
            if kw.lower() in text_lower:
                not_match = True
                break
        if not_match:
            continue
        return rule
    return None

def _llm_classify(text: str, source_type: str) -> dict | None:
    """Layer 2/3: LLM classification using Haiku. Returns signal dict or None."""
    if not C.LLM_ENABLED:
        return None

    api_key = os.environ.get("ANTHROPIC_API_KEY", "")
    if not api_key:
        return None

    # Pre-screen: check if text has any relevant keywords
    text_lower = text.lower()
    has_keyword = False
    for kw in C.LLM_PRESCREEN_KEYWORDS:
        if kw.startswith(r"\$"):
            if re.search(kw, text):
                has_keyword = True
                break
        elif kw.lower() in text_lower:
            has_keyword = True
            break
    if not has_keyword:
        return None

    event_types = list(C.MACRO_PLAYBOOK.keys())
    prompt = f"""Classify this {source_type} message into a macro event type.

Event types: {', '.join(event_types)}

If the message matches an event type, respond with JSON:
{{"event_type": "...", "direction": "bullish|bearish|neutral", "confidence": 0.0-1.0}}

If not relevant to macro/crypto, respond: {{"event_type": "none"}}

Message: {text[:500]}"""

    try:
        import urllib.request
        body = json.dumps({
            "model": C.LLM_MODEL,
            "max_tokens": C.LLM_MAX_TOKENS,
            "messages": [{"role": "user", "content": prompt}],
        }).encode()
        req = urllib.request.Request(
            "https://api.anthropic.com/v1/messages",
            data=body,
            headers={
                "Content-Type": "application/json",
                "x-api-key": api_key,
                "anthropic-version": "2023-06-01",
            },
        )
        with _state_lock:
            _stats["llm_calls"] += 1
        with urllib.request.urlopen(req, timeout=C.LLM_TIMEOUT_SEC) as resp:
            result = json.loads(resp.read().decode())
        content = result.get("content", [{}])[0].get("text", "")
        # Extract JSON from response
        match = re.search(r'\{[^}]+\}', content)
        if not match:
            return None
        data = json.loads(match.group())
        if data.get("event_type", "none") == "none":
            return None
        confidence = float(data.get("confidence", 0.5))
        if confidence < C.LLM_CONFIDENCE_BAND[0]:
            return None
        return {
            "event_type": data["event_type"],
            "direction": data.get("direction", "neutral"),
            "confidence": confidence,
            "classify_method": "llm_confirm" if confidence >= C.LLM_CONFIDENCE_BAND[1] else "llm_discover",
        }
    except Exception as e:
        _log(f"LLM classify error: {e}", "WARN")
        return None

def classify_text(text: str, source_type: str) -> dict:
    """Unified 3-layer classification. Returns classification result."""
    # Layer 1a: Macro keyword regex
    kw_match = _match_macro_keywords(text)
    if kw_match:
        event_type, confidence = kw_match
        playbook = C.MACRO_PLAYBOOK.get(event_type, {})
        return {
            "event_type": event_type,
            "direction": playbook.get("direction", "neutral"),
            "magnitude": playbook.get("magnitude", 0.5),
            "urgency": playbook.get("urgency", 0.5),
            "affects": playbook.get("affects", []),
            "classify_method": "keyword",
        }

    # Layer 1b: Classifier rules (TG-style)
    rule = _match_classifier_rules(text)
    if rule:
        return {
            "event_type": rule["event_type"],
            "direction": rule["direction"],
            "magnitude": rule["magnitude"],
            "urgency": C.MACRO_PLAYBOOK.get(rule["event_type"], {}).get("urgency", 0.5),
            "affects": rule["affects"],
            "classify_method": "keyword",
        }

    # Layer 2/3: LLM classification
    llm_result = _llm_classify(text, source_type)
    if llm_result:
        event_type = llm_result["event_type"]
        playbook = C.MACRO_PLAYBOOK.get(event_type, {})
        return {
            "event_type": event_type,
            "direction": llm_result.get("direction", playbook.get("direction", "neutral")),
            "magnitude": playbook.get("magnitude", 0.5),
            "urgency": playbook.get("urgency", 0.5),
            "affects": playbook.get("affects", []),
            "classify_method": llm_result["classify_method"],
        }

    return {
        "event_type": "unclassified",
        "direction": "neutral",
        "magnitude": 0.0,
        "urgency": 0.0,
        "affects": [],
        "classify_method": "none",
    }

# ═══════════════════════════════════════════════════════════════════════
#  LLM INSIGHT GENERATOR
# ═══════════════════════════════════════════════════════════════════════
def _generate_insight(headline: str, event_type: str, direction: str,
                      affects: list[str]) -> str:
    """Call Haiku to produce a 2-3 sentence insight explaining what the headline
    means for specific asset classes. Returns insight text or empty string."""
    if not C.LLM_INSIGHT_ENABLED:
        return ""
    api_key = os.environ.get("ANTHROPIC_API_KEY", "")
    if not api_key:
        return ""

    affects_str = ", ".join(affects) if affects else "broad crypto market"
    prompt = (
        f"You are a macro analyst. Given this headline and its classification, "
        f"write 2-3 concise sentences explaining:\n"
        f"1) The key takeaway from this event\n"
        f"2) How it is likely to affect specific assets or sectors "
        f"({affects_str})\n\n"
        f"Headline: {headline[:400]}\n"
        f"Event type: {event_type}\n"
        f"Direction: {direction}\n\n"
        f"Be specific about which assets benefit or suffer and why. "
        f"No preamble — start directly with the analysis."
    )

    try:
        import urllib.request
        body = json.dumps({
            "model": C.LLM_MODEL,
            "max_tokens": C.LLM_INSIGHT_MAX_TOKENS,
            "messages": [{"role": "user", "content": prompt}],
        }).encode()
        req = urllib.request.Request(
            "https://api.anthropic.com/v1/messages",
            data=body,
            headers={
                "Content-Type": "application/json",
                "x-api-key": api_key,
                "anthropic-version": "2023-06-01",
            },
        )
        with _state_lock:
            _stats["llm_calls"] += 1
        with urllib.request.urlopen(req, timeout=C.LLM_INSIGHT_TIMEOUT_SEC) as resp:
            result = json.loads(resp.read().decode())
        content = result.get("content", [{}])[0].get("text", "")
        return content.strip()
    except Exception as e:
        _log(f"Insight generation error: {e}", "WARN")
        return ""

# ═══════════════════════════════════════════════════════════════════════
#  SENTIMENT SCORING
# ═══════════════════════════════════════════════════════════════════════
def _score_sentiment(text: str) -> float:
    """Score sentiment from -1.0 to +1.0 using weighted lexicon."""
    words = re.findall(r'[\w\u4e00-\u9fff]+', text.lower())
    total_weight = 0.0
    word_count = 0
    for w in words:
        if w in C.POSITIVE_WORDS:
            total_weight += C.POSITIVE_WORDS[w]
            word_count += 1
        elif w in C.NEGATIVE_WORDS:
            total_weight += C.NEGATIVE_WORDS[w]
            word_count += 1
    if word_count == 0:
        return 0.0
    return max(-1.0, min(1.0, total_weight / word_count))

# ═══════════════════════════════════════════════════════════════════════
#  TOKEN EXTRACTION
# ═══════════════════════════════════════════════════════════════════════
def _extract_tokens(text: str) -> list[str]:
    """Extract ticker symbols from text."""
    dollar_tickers = re.findall(r'\$([A-Za-z]{2,10})', text)
    caps_tickers = re.findall(r'\b([A-Z]{3,5})\b', text)
    all_tickers = set(t.upper() for t in dollar_tickers)
    all_tickers.update(t for t in caps_tickers if t not in C.TICKER_NOISE_WORDS)
    return sorted(all_tickers)

# ═══════════════════════════════════════════════════════════════════════
#  TOKEN IMPACT ENGINE
# ═══════════════════════════════════════════════════════════════════════
def _compute_token_impacts(event_type: str, direction: str, magnitude: float,
                           extracted_tokens: list[str]) -> list[dict]:
    """Compute per-token impact scores from event type and extracted tokens.

    Returns list of {symbol, impact, direction} sorted by abs(impact) desc.
    """
    impacts = {}  # symbol → impact score

    # 1. Map from event type (high confidence — curated correlations)
    base_map = C.TOKEN_IMPACT_MAP.get(event_type, [])
    if not base_map and direction != "neutral":
        # Unknown event type but has direction → use generic crypto correlation
        base_map = C.TOKEN_IMPACT_GENERIC
        if direction == "bearish":
            base_map = [(sym, -abs(score)) for sym, score in base_map]

    for sym, base_score in base_map:
        impacts[sym] = round(base_score * magnitude, 3)

    # 2. Extracted tokens not already covered get directional score
    for tok in extracted_tokens:
        tok_up = tok.upper()
        if tok_up in impacts or tok_up in C.TICKER_NOISE_WORDS:
            continue
        # Assign based on direction with lower confidence
        if direction == "bullish":
            impacts[tok_up] = round(magnitude * 0.45, 3)
        elif direction == "bearish":
            impacts[tok_up] = round(-magnitude * 0.45, 3)
        else:
            impacts[tok_up] = 0.0

    # 3. Convert to sorted list
    result = []
    for sym, score in sorted(impacts.items(), key=lambda x: abs(x[1]), reverse=True):
        result.append({
            "symbol": sym,
            "impact": round(score, 2),
            "direction": "bullish" if score > 0.01 else "bearish" if score < -0.01 else "neutral",
        })
    return result[:8]  # Cap at 8 tokens max


# ═══════════════════════════════════════════════════════════════════════
#  REPUTATION SYSTEM
# ═══════════════════════════════════════════════════════════════════════
def _update_sender_rep(sender_id: str, event_type: str):
    """Update sender reputation based on signal quality."""
    if not C.REPUTATION_ENABLED or not sender_id:
        return
    with _state_lock:
        if sender_id not in _reputation:
            _reputation[sender_id] = {"score": 0.0, "last_ts": time.time(), "hits": 0}

        rep = _reputation[sender_id]
        now = time.time()

        # Time decay
        days_elapsed = (now - rep["last_ts"]) / 86400
        if days_elapsed > 0 and C.REPUTATION_DECAY_DAYS > 0:
            decay = 1.0 - (min(days_elapsed, C.REPUTATION_DECAY_DAYS) / C.REPUTATION_DECAY_DAYS) * 0.1
            rep["score"] *= max(0.0, decay)

        # Boost/penalty
        if event_type in ("alpha_call", "whale_buy", "whale_sell"):
            rep["score"] += C.REPUTATION_BOOST_ALPHA
        elif event_type == "unclassified":
            rep["score"] += C.REPUTATION_PENALTY_NOISE
        else:
            rep["score"] += C.REPUTATION_BOOST_NEWS

        rep["score"] = max(C.REPUTATION_MIN_SCORE, min(C.REPUTATION_MAX_SCORE, rep["score"]))
        rep["last_ts"] = now
        rep["hits"] = rep.get("hits", 0) + 1

def _get_sender_rep(sender_id: str) -> float:
    with _state_lock:
        return _reputation.get(sender_id, {}).get("score", 0.0)

def _decay_reputations():
    """Periodic decay of all reputations."""
    if not C.REPUTATION_ENABLED:
        return
    now = time.time()
    with _state_lock:
        for sid, rep in _reputation.items():
            days = (now - rep["last_ts"]) / 86400
            if days > C.REPUTATION_DECAY_DAYS:
                rep["score"] *= 0.5

# ═══════════════════════════════════════════════════════════════════════
#  UNIFIED PIPELINE — single entry point for all sources
# ═══════════════════════════════════════════════════════════════════════
def process_signal(text: str, source_type: str, source_name: str,
                   sender: str = "", group_category: str = "",
                   is_reply: bool = False, is_forward_from_bot: bool = False,
                   is_deep_reply: bool = False,
                   source_ts: float = 0.0) -> dict | None:
    """
    Unified signal processing pipeline.
    source_type: "newsnow" | "polymarket" | "telegram" | "opennews" | "finnhub" | "fred" | "cryptopanic" | "rss"
    source_ts: original publication timestamp (for latency tracking)
    Returns UnifiedSignal dict or None if filtered.
    """
    with _state_lock:
        _stats["messages_processed"] += 1

    # 1. Noise filter (TG only)
    if source_type == "telegram":
        if _is_noise(text, sender, sender, is_reply, is_forward_from_bot, is_deep_reply):
            _update_sender_rep(sender, "unclassified")
            return None

    # 2. Dedup — exact MD5 match
    if _is_duplicate(text):
        return None

    # 2b. Fuzzy dedup — Jaccard similarity
    if _is_fuzzy_duplicate(text):
        return None

    # 3. Classify
    classification = classify_text(text, source_type)
    if classification["event_type"] == "unclassified" and classification["magnitude"] == 0.0:
        if source_type != "telegram":
            return None
        _update_sender_rep(sender, "unclassified")
        return None

    # 4. Sentiment
    sentiment = _score_sentiment(text)

    # 5. Token extraction
    tokens = _extract_tokens(text)

    # 6. Reputation
    sender_rep = 0.0
    if sender:
        _update_sender_rep(sender, classification["event_type"])
        sender_rep = _get_sender_rep(sender)
        if sender_rep >= C.REPUTATION_HIGH_SIGNAL:
            classification["magnitude"] = min(1.0, classification["magnitude"] * 1.3)

    # 6b. Trend detection — boost urgency/magnitude if trending
    urgency = classification.get("urgency", 0.5)
    magnitude = classification["magnitude"]
    trend_meta = None
    if classification["event_type"] != "unclassified":
        ub, mb, trend_meta = _detect_trend(classification["event_type"], urgency, magnitude)
        urgency = min(1.0, urgency + ub)
        magnitude = min(1.0, magnitude + mb)
        classification["urgency"] = urgency
        classification["magnitude"] = magnitude

    # 7. Generate AI insight (for classified signals only)
    insight = ""
    if classification["event_type"] != "unclassified":
        insight = _generate_insight(
            text, classification["event_type"],
            classification["direction"],
            classification.get("affects", []),
        )

    # 8. Build signal
    now = time.time()
    latency_ms = int((now - source_ts) * 1000) if source_ts > 0 else 0
    signal = {
        "ts": int(now),
        "ts_human": datetime.now().strftime("%m-%d %H:%M:%S"),
        "source_type": source_type,
        "source_name": source_name,
        "event_type": classification["event_type"],
        "direction": classification["direction"],
        "magnitude": round(classification["magnitude"], 2),
        "urgency": round(classification.get("urgency", 0.5), 2),
        "affects": classification.get("affects", []),
        "tokens": tokens,
        "sentiment": round(sentiment, 3),
        "text": text[:400],
        "insight": insight,
        "sender": sender or source_name,
        "sender_rep": round(sender_rep, 2),
        "classify_method": classification["classify_method"],
        "group_category": group_category or ("http_news" if source_type == "newsnow" else source_type),
        "source_ts": source_ts if source_ts > 0 else 0,
        "latency_ms": latency_ms,
    }

    # 8b. Token impact analysis — map event to specific crypto tokens
    signal["token_impacts"] = _compute_token_impacts(
        signal["event_type"], signal["direction"],
        signal["magnitude"], tokens,
    )

    # 9. Store
    with _state_lock:
        _signals.append(signal)
        if len(_signals) > C.MAX_SIGNALS_KEPT:
            _signals[:] = _signals[-C.MAX_SIGNALS_KEPT:]
        _stats["signals_produced"] += 1
        _source_status[source_name] = now

    _log(f"SIGNAL [{source_type}] {classification['event_type']} "
         f"{classification['direction']} mag={classification['magnitude']:.2f} "
         f"from={source_name} method={classification['classify_method']}"
         f"{f' latency={latency_ms}ms' if latency_ms > 0 else ''}")

    # 10. WebSocket broadcast
    try:
        _ws_broadcast_queue.put_nowait(signal)
    except queue.Full:
        pass

    # 11. Webhook push
    _webhook_push(signal)

    # 12. Accuracy tracking
    _record_accuracy_checkpoint(signal)

    # 13. Emit trend meta-signal (if 3rd occurrence detected in step 6b)
    if trend_meta:
        now_ts = time.time()
        meta_signal = {
            "ts": int(now_ts),
            "ts_human": datetime.now().strftime("%m-%d %H:%M:%S"),
            "source_type": source_type,
            "source_name": "trend_detector",
            "event_type": trend_meta["event_type"],
            "direction": trend_meta["direction"],
            "magnitude": round(trend_meta["magnitude"], 2),
            "urgency": round(trend_meta["urgency"], 2),
            "affects": trend_meta["affects"],
            "tokens": [],
            "sentiment": 0.0,
            "text": trend_meta["text"],
            "insight": "",
            "sender": "trend_detector",
            "sender_rep": 0.0,
            "classify_method": "trend",
            "group_category": "macro",
            "source_ts": 0,
            "latency_ms": 0,
        }
        with _state_lock:
            _signals.append(meta_signal)
            _stats["signals_produced"] += 1
        try:
            _ws_broadcast_queue.put_nowait(meta_signal)
        except queue.Full:
            pass
        _webhook_push(meta_signal)
        _log(f"TREND [{trend_meta['event_type']}] {trend_meta['text']}")

    return signal

# ═══════════════════════════════════════════════════════════════════════
#  NEWS COLLECTOR THREAD
# ═══════════════════════════════════════════════════════════════════════
def _news_collector_loop():
    """Background thread: polls NewsNow + Polymarket + Finnhub + FRED + OpenNews REST on intervals."""
    global _polymarket, _fear_greed, _fred_indicators, _price_tickers
    _log("NewsNow + Polymarket + Finnhub + FRED + CryptoPanic + RSS collector started")
    news_last = 0
    poly_last = 0
    fng_last = 0
    finnhub_last = 0
    fred_last = 0
    opennews_rest_last = 0
    prices_last = 0
    cryptopanic_last = 0

    while True:
        try:
            now = time.time()

            # NewsNow headlines
            if now - news_last >= C.NEWS_POLL_SEC:
                news_last = now
                headlines = fetch_news_headlines()
                with _state_lock:
                    _stats["news_fetches"] += 1
                for h in headlines:
                    # Parse pubDate for latency tracking
                    src_ts = 0.0
                    raw_ts = h.get("ts", "")
                    if isinstance(raw_ts, (int, float)) and raw_ts > 1e9:
                        src_ts = float(raw_ts)
                    process_signal(
                        text=h["title"],
                        source_type="newsnow",
                        source_name=h["source"],
                        group_category="http_news",
                        source_ts=src_ts,
                    )
                if headlines:
                    _log(f"NewsNow: fetched {len(headlines)} headlines")

            # Polymarket
            if now - poly_last >= C.POLYMARKET_POLL_SEC:
                poly_last = now
                markets = fetch_polymarket_signals()
                if markets:
                    with _state_lock:
                        _polymarket = markets
                        _source_status["polymarket"] = now
                    # Group markets by event category — emit ONE signal per group
                    by_cat: dict[str, list] = {}
                    for m in markets:
                        cat = m.get("category", "") or "other"
                        by_cat.setdefault(cat, []).append(m)
                    for cat, cat_markets in by_cat.items():
                        # Pick the most notable market (highest prob divergence from 50%)
                        best = max(cat_markets, key=lambda x: abs(x.get("probability", 0.5) - 0.5))
                        prob = best.get("probability", 0.5)
                        q = best.get("question", "")
                        if abs(prob - 0.5) < 0.15:
                            continue  # Skip near-50/50 markets
                        n_related = len(cat_markets)
                        summary = f"{q} — currently at {prob:.0%} probability"
                        if n_related > 1:
                            summary += f". {n_related} related prediction markets tracking this event."
                        process_signal(
                            text=summary,
                            source_type="polymarket",
                            source_name="polymarket",
                            group_category="polymarket",
                        )
                    _log(f"Polymarket: fetched {len(markets)} markets in {len(by_cat)} groups")

            # Fear & Greed Index (every 5 min — updates daily but cheap to poll)
            if now - fng_last >= 300:
                fng_last = now
                fng = fetch_fear_greed()
                if fng:
                    with _state_lock:
                        _fear_greed = fng
                    _log(f"Fear & Greed: {fng['value']} ({fng['label']})")

            # Price tickers (every 60s)
            if now - prices_last >= C.PRICE_TICKER_POLL_SEC:
                prices_last = now
                tickers = fetch_price_tickers()
                if tickers:
                    with _state_lock:
                        _price_tickers = tickers
                    parts = [f"{v['label']}=${v['price']:,.1f}" for v in tickers.values()]
                    _log(f"Prices: {', '.join(parts)}")

            # Finnhub market news
            if C.FINNHUB_ENABLED and C.FINNHUB_API_KEY and now - finnhub_last >= C.FINNHUB_POLL_SEC:
                finnhub_last = now
                articles = fetch_finnhub_news()
                for a in articles:
                    src_ts = float(a.get("ts", 0)) if a.get("ts") else 0.0
                    process_signal(
                        text=a["title"],
                        source_type="finnhub",
                        source_name=a.get("source", "finnhub"),
                        group_category="http_news",
                        source_ts=src_ts,
                    )
                if articles:
                    _log(f"Finnhub: fetched {len(articles)} articles")

            # FRED macro indicators
            if C.FRED_ENABLED and C.FRED_API_KEY and now - fred_last >= C.FRED_POLL_SEC:
                fred_last = now
                indicators = fetch_fred_indicators()
                if indicators:
                    with _state_lock:
                        _fred_indicators = indicators
                        _source_status["fred"] = now
                    _log(f"FRED: updated {len(indicators)} indicators")
                    # Significant change detection — emit signals
                    for series_id, data in indicators.items():
                        if data["change"] is not None:
                            threshold = _FRED_CHANGE_THRESHOLDS.get(series_id, 0.2)
                            if abs(data["change"]) >= threshold:
                                process_signal(
                                    text=f"FRED {data['label']}: {data['value']} (prev: {data['prev_value']}, change: {data['change']:+.2f})",
                                    source_type="fred",
                                    source_name="fred",
                                    group_category="macro_data",
                                )

            # 6551.io OpenNews REST (always poll — WS returns 403 on free tier)
            if (C.OPENNEWS_ENABLED and C.OPENNEWS_TOKEN
                    and now - opennews_rest_last >= C.OPENNEWS_POLL_SEC):
                opennews_rest_last = now
                articles = fetch_opennews_rest()
                for a in articles:
                    process_signal(
                        text=a["text"],
                        source_type="opennews",
                        source_name=a.get("source", "opennews"),
                        group_category="opennews",
                    )
                if articles:
                    _log(f"OpenNews REST: fetched {len(articles)} signal articles ({len(_opennews_articles)} in buffer)")

            # CryptoPanic
            if (C.CRYPTOPANIC_ENABLED and C.CRYPTOPANIC_TOKEN
                    and now - cryptopanic_last >= C.CRYPTOPANIC_POLL_SEC):
                cryptopanic_last = now
                cp_articles = fetch_cryptopanic_news()
                for a in cp_articles:
                    # Use community votes to boost magnitude
                    vote_hint = ""
                    pos = a.get("votes_positive", 0)
                    neg = a.get("votes_negative", 0)
                    imp = a.get("votes_important", 0)
                    if pos + neg + imp > 0:
                        vote_hint = f" [votes: +{pos}/-{neg} imp:{imp}]"
                    process_signal(
                        text=a["title"] + vote_hint,
                        source_type="cryptopanic",
                        source_name=a.get("source", "cryptopanic"),
                        group_category="crypto_news",
                        source_ts=a.get("source_ts", 0),
                    )
                if cp_articles:
                    with _state_lock:
                        _source_status["cryptopanic"] = now
                    _log(f"CryptoPanic: fetched {len(cp_articles)} articles")

            # RSS feeds
            if C.RSS_ENABLED and C.RSS_FEEDS:
                for feed_cfg in C.RSS_FEEDS:
                    feed_url = feed_cfg.get("url", "")
                    poll_sec = feed_cfg.get("poll_sec", C.RSS_DEFAULT_POLL_SEC)
                    last_poll = _rss_last_poll.get(feed_url, 0)
                    if now - last_poll >= poll_sec:
                        _rss_last_poll[feed_url] = now
                        rss_articles = fetch_rss_feed(feed_cfg)
                        category = feed_cfg.get("category", "crypto_news")
                        for a in rss_articles:
                            process_signal(
                                text=a["title"],
                                source_type="rss",
                                source_name=a.get("source", "rss"),
                                group_category=category,
                                source_ts=a.get("source_ts", 0),
                            )
                        if rss_articles:
                            label = feed_cfg.get("label", feed_url[:30])
                            with _state_lock:
                                _source_status[f"rss:{label}"] = now
                            _log(f"RSS ({label}): fetched {len(rss_articles)} articles")

            # Periodic save + reputation decay + accuracy check
            _save_state()
            _decay_reputations()
            _check_accuracy()

        except Exception as e:
            _log(f"News collector error: {e}", "ERROR")
            traceback.print_exc()

        time.sleep(10)

# ═══════════════════════════════════════════════════════════════════════
#  TELEGRAM COLLECTOR (Telethon)
# ═══════════════════════════════════════════════════════════════════════
_telethon_available = False
try:
    from telethon import TelegramClient, events
    from telethon.tl.types import User, Channel, Chat
    _telethon_available = True
except ImportError:
    pass

def _build_group_map() -> dict:
    """Build identifier -> category mapping from config."""
    gmap = {}
    for category, identifiers in C.GROUPS.items():
        for ident in identifiers:
            gmap[ident] = category
    for category, identifiers in C.CHANNELS.items():
        for ident in identifiers:
            gmap[ident] = category
    return gmap

async def _telethon_monitor():
    """Async Telethon event loop — runs in a dedicated thread."""
    api_id = C.TELETHON_API_ID or int(os.environ.get("TG_API_ID", "0"))
    api_hash = C.TELETHON_API_HASH or os.environ.get("TG_API_HASH", "")
    if not api_id or not api_hash:
        _log("Telethon: no API credentials — Telegram monitoring disabled", "WARN")
        return

    session_path = str(_BASE_DIR / C.SESSION_NAME)
    client = TelegramClient(session_path, api_id, api_hash)
    await client.start()
    _log("Telethon: connected")

    group_map = _build_group_map()
    resolved_chats = []
    chat_categories = {}

    for identifier, category in group_map.items():
        try:
            entity = await client.get_entity(identifier)
            eid = entity.id
            resolved_chats.append(eid)
            chat_name = getattr(entity, 'title', getattr(entity, 'username', str(eid)))
            chat_categories[eid] = (category, chat_name)
            _log(f"Telethon: resolved {identifier} → {chat_name} ({category})")
        except Exception as e:
            _log(f"Telethon: failed to resolve {identifier}: {e}", "WARN")

    if not resolved_chats:
        _log("Telethon: no chats resolved — monitoring disabled", "WARN")
        await client.disconnect()
        return

    @client.on(events.NewMessage(chats=resolved_chats))
    async def _on_message(event):
        text = event.text or ""
        if not text.strip():
            return

        with _state_lock:
            _stats["tg_messages"] += 1

        # Extract sender info
        sender = await event.get_sender()
        sender_id = str(getattr(sender, 'id', ''))
        sender_name = ""
        is_bot = False
        if isinstance(sender, User):
            sender_name = sender.username or f"{sender.first_name or ''} {sender.last_name or ''}".strip()
            is_bot = sender.bot or False
        elif hasattr(sender, 'title'):
            sender_name = sender.title

        # Reply/forward info
        is_reply = event.is_reply
        is_forward_from_bot = False
        is_deep_reply = False
        if event.forward and hasattr(event.forward, 'sender') and event.forward.sender:
            is_forward_from_bot = getattr(event.forward.sender, 'bot', False)
        if is_reply:
            try:
                reply_msg = await event.get_reply_message()
                if reply_msg and reply_msg.is_reply:
                    is_deep_reply = True
            except Exception:
                pass

        # Chat category
        chat_id = event.chat_id
        category, chat_name = chat_categories.get(chat_id, ("general", "unknown"))

        tg_source_ts = event.date.timestamp() if event.date else 0.0
        process_signal(
            text=text,
            source_type="telegram",
            source_name=chat_name,
            sender=sender_name or sender_id,
            group_category=category,
            is_reply=is_reply,
            is_forward_from_bot=is_forward_from_bot,
            is_deep_reply=is_deep_reply,
            source_ts=tg_source_ts,
        )

    _log(f"Telethon: monitoring {len(resolved_chats)} chats")
    await client.run_until_disconnected()

def _start_telethon_thread():
    """Start Telethon in a dedicated thread with its own event loop."""
    import asyncio
    def _run():
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            loop.run_until_complete(_telethon_monitor())
        except Exception as e:
            _log(f"Telethon thread error: {e}", "ERROR")
            traceback.print_exc()
    t = threading.Thread(target=_run, daemon=True, name="telethon")
    t.start()
    return t

# ═══════════════════════════════════════════════════════════════════════
#  6551.io OPENNEWS WEBSOCKET COLLECTOR
# ═══════════════════════════════════════════════════════════════════════
async def _opennews_monitor():
    """WebSocket listener for 6551.io OpenNews — runs in dedicated thread."""
    global _opennews_ws_alive
    import asyncio
    try:
        import websockets
    except ImportError:
        _log("OpenNews: websockets not installed — run: pip install websockets", "WARN")
        return

    backoff_secs = [5, 10, 30, 60]
    attempt = 0

    while True:
        ws_url = f"{C.OPENNEWS_WSS_URL}?token={C.OPENNEWS_TOKEN}"
        try:
            async with websockets.connect(ws_url, ping_interval=30, ping_timeout=10) as ws:
                _opennews_ws_alive = True
                attempt = 0
                _log("OpenNews: WebSocket connected")

                # Subscribe to news updates
                engine_filter = {et: [] for et in C.OPENNEWS_ENGINE_TYPES}
                subscribe_msg = json.dumps({
                    "method": "news.subscribe",
                    "params": {"engineTypes": engine_filter, "hasCoin": False},
                })
                await ws.send(subscribe_msg)
                _log(f"OpenNews: subscribed to {C.OPENNEWS_ENGINE_TYPES}")

                # Pending articles waiting for AI rating
                pending: dict[str, dict] = {}  # news_id -> article data

                async for raw in ws:
                    try:
                        msg = json.loads(raw)
                    except (json.JSONDecodeError, TypeError):
                        continue

                    method = msg.get("method", "")

                    if method == "news.update":
                        # New article arrived
                        params = msg.get("params", {})
                        news_id = str(params.get("id", params.get("newsId", "")))
                        text = params.get("text", params.get("title", ""))
                        news_type = params.get("newsType", "opennews")
                        engine_type = params.get("engineType", "")
                        link = params.get("link", "")

                        # Dual-store: raw article buffer for OpenNews tab
                        article = _normalize_opennews_article(params)
                        if article:
                            _ingest_opennews_article(article)

                        if text and news_id:
                            pending[news_id] = {
                                "text": text,
                                "newsType": news_type,
                                "engineType": engine_type,
                                "link": link,
                                "coins": params.get("coins", []),
                                "arrival_ts": time.time(),
                            }
                            # Evict old pending entries (keep last 200)
                            if len(pending) > 200:
                                oldest = list(pending.keys())[:100]
                                for k in oldest:
                                    del pending[k]

                    elif method == "news.ai_update":
                        # AI rating for a previously received article
                        params = msg.get("params", {})
                        news_id = str(params.get("id", params.get("newsId", "")))
                        ai_rating = params.get("aiRating", {})
                        score = ai_rating.get("score", 0)
                        signal = ai_rating.get("signal", "neutral")
                        en_summary = ai_rating.get("enSummary", "")

                        # Dual-store: update raw article buffer
                        if news_id and isinstance(ai_rating, dict):
                            _update_opennews_ai(news_id, ai_rating)

                        article = pending.pop(news_id, None)
                        if article and score >= C.OPENNEWS_MIN_SCORE:
                            display_text = en_summary or article["text"]
                            with _state_lock:
                                _source_status["opennews_ws"] = time.time()
                            process_signal(
                                text=display_text,
                                source_type="opennews",
                                source_name=article["newsType"],
                                group_category="opennews",
                                source_ts=article.get("arrival_ts", 0),
                            )
                            _log(f"OpenNews WS: score={score} signal={signal} src={article['newsType']}")

        except Exception as e:
            _opennews_ws_alive = False
            delay = backoff_secs[min(attempt, len(backoff_secs) - 1)]
            _log(f"OpenNews WS disconnected: {e} — reconnecting in {delay}s", "WARN")
            attempt += 1
            await asyncio.sleep(delay)


def _start_opennews_thread():
    """Start 6551.io WebSocket in a dedicated thread."""
    import asyncio
    def _run():
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            loop.run_until_complete(_opennews_monitor())
        except Exception as e:
            _log(f"OpenNews thread error: {e}", "ERROR")
            traceback.print_exc()
    t = threading.Thread(target=_run, daemon=True, name="opennews_ws")
    t.start()
    return t

# ═══════════════════════════════════════════════════════════════════════
#  WEBSOCKET SERVER (real-time signal push)
# ═══════════════════════════════════════════════════════════════════════
async def _ws_handler(websocket):
    """Handle a single WebSocket client connection."""
    _ws_clients.add(websocket)
    _ws_client_filters[websocket] = {}
    _log(f"WS: client connected ({len(_ws_clients)} total)")
    try:
        async for raw in websocket:
            try:
                msg = json.loads(raw)
            except (json.JSONDecodeError, TypeError):
                continue
            action = msg.get("action", "")
            if action == "subscribe":
                _ws_client_filters[websocket] = {
                    "direction": msg.get("direction", ""),
                    "min_mag": float(msg.get("min_mag", 0)),
                    "affects": msg.get("affects", []),
                }
                await websocket.send(json.dumps({"status": "subscribed", "filters": _ws_client_filters[websocket]}))
            elif action == "ping":
                await websocket.send(json.dumps({"status": "pong"}))
    except Exception:
        pass
    finally:
        _ws_clients.discard(websocket)
        _ws_client_filters.pop(websocket, None)
        _log(f"WS: client disconnected ({len(_ws_clients)} total)")


def _ws_signal_matches(signal: dict, filters: dict) -> bool:
    """Check if a signal matches a client's subscription filters."""
    if not filters:
        return True
    if filters.get("direction") and signal.get("direction") != filters["direction"]:
        return False
    if filters.get("min_mag", 0) > 0 and signal.get("magnitude", 0) < filters["min_mag"]:
        return False
    if filters.get("affects"):
        sig_affects = set(signal.get("affects", []))
        if not sig_affects.intersection(filters["affects"]):
            return False
    return True


async def _ws_broadcast_worker():
    """Pull signals from queue and broadcast to matching WS clients."""
    import asyncio
    while True:
        try:
            item = await asyncio.get_event_loop().run_in_executor(
                None, lambda: _ws_broadcast_queue.get(timeout=1)
            )
        except queue.Empty:
            continue
        except Exception:
            await asyncio.sleep(0.5)
            continue

        # OpenNews article broadcasts go to ALL clients (no signal filter matching)
        if item.get("_opennews_article"):
            payload = json.dumps({"type": item["type"], "data": item["data"]}, default=str)
            dead = set()
            for ws in list(_ws_clients):
                try:
                    await ws.send(payload)
                except Exception:
                    dead.add(ws)
            for ws in dead:
                _ws_clients.discard(ws)
                _ws_client_filters.pop(ws, None)
        else:
            # Regular signal broadcast with filter matching
            signal = item
            payload = json.dumps({"type": "signal", "data": signal}, default=str)
            dead = set()
            for ws in list(_ws_clients):
                filters = _ws_client_filters.get(ws, {})
                if not _ws_signal_matches(signal, filters):
                    continue
                try:
                    await ws.send(payload)
                except Exception:
                    dead.add(ws)
            for ws in dead:
                _ws_clients.discard(ws)
                _ws_client_filters.pop(ws, None)


async def _ws_server_loop():
    """Run WebSocket server + broadcast worker."""
    import asyncio
    try:
        import websockets
    except ImportError:
        _log("WS server: websockets not installed — disabled", "WARN")
        return

    server = await websockets.serve(
        _ws_handler, "0.0.0.0", C.WS_PORT,
        ping_interval=C.WS_PING_INTERVAL,
        ping_timeout=C.WS_PING_TIMEOUT,
        max_size=2**16,
    )
    _log(f"WS server listening on :{C.WS_PORT}")
    broadcast_task = asyncio.create_task(_ws_broadcast_worker())
    try:
        await asyncio.Future()  # run forever
    finally:
        broadcast_task.cancel()
        server.close()


def _start_ws_server_thread():
    """Start WebSocket server in a dedicated daemon thread."""
    import asyncio
    def _run():
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            loop.run_until_complete(_ws_server_loop())
        except Exception as e:
            _log(f"WS server thread error: {e}", "ERROR")
            traceback.print_exc()
    t = threading.Thread(target=_run, daemon=True, name="ws_server")
    t.start()
    return t


# ═══════════════════════════════════════════════════════════════════════
#  WEBHOOK PUSH (fire-and-forget POST)
# ═══════════════════════════════════════════════════════════════════════
def _webhook_push(signal: dict):
    """POST signal JSON to configured webhook URLs (non-blocking, daemon thread)."""
    if not C.WEBHOOK_URLS:
        return
    # Filter by magnitude
    if signal.get("magnitude", 0) < C.WEBHOOK_MIN_MAGNITUDE:
        return
    # Filter by event type
    if C.WEBHOOK_EVENTS and signal.get("event_type") not in C.WEBHOOK_EVENTS:
        return

    payload = json.dumps(signal, default=str).encode()

    def _post(url):
        try:
            req = Request(url, data=payload, method="POST",
                          headers={"Content-Type": "application/json",
                                   "User-Agent": "MacroIntelligence/2.0"})
            with urlopen(req, timeout=C.WEBHOOK_TIMEOUT_SEC):
                pass
        except Exception as e:
            _log(f"Webhook POST to {url[:50]} failed: {e}", "WARN")

    for url in C.WEBHOOK_URLS:
        threading.Thread(target=_post, args=(url,), daemon=True).start()


# ═══════════════════════════════════════════════════════════════════════
#  SIGNAL ACCURACY TRACKING
# ═══════════════════════════════════════════════════════════════════════
def _record_accuracy_checkpoint(signal: dict):
    """Record BTC/ETH price at signal time for later accuracy check."""
    if not C.ACCURACY_ENABLED:
        return
    direction = signal.get("direction")
    if direction not in ("bullish", "bearish"):
        return
    with _state_lock:
        btc = _price_tickers.get("BTC", {}).get("price", 0)
        eth = _price_tickers.get("ETH", {}).get("price", 0)
    if not btc:
        return
    now = time.time()
    check_at = [now + h * 3600 for h in C.ACCURACY_CHECK_HOURS]
    _accuracy_pending.append({
        "signal_ts": signal["ts"],
        "event_type": signal["event_type"],
        "direction": direction,
        "btc_price": btc,
        "eth_price": eth,
        "check_at": check_at,
    })
    # Cap pending list
    if len(_accuracy_pending) > 500:
        _accuracy_pending[:] = _accuracy_pending[-500:]


def _check_accuracy():
    """Background check: compare current prices to signal-time prices."""
    if not C.ACCURACY_ENABLED or not _accuracy_pending:
        return
    now = time.time()
    with _state_lock:
        btc_now = _price_tickers.get("BTC", {}).get("price", 0)
        eth_now = _price_tickers.get("ETH", {}).get("price", 0)
    if not btc_now:
        return

    still_pending = []
    for entry in _accuracy_pending:
        remaining_checks = []
        for check_ts in entry["check_at"]:
            if now >= check_ts:
                # Evaluate accuracy
                direction = entry["direction"]
                btc_moved_up = btc_now > entry["btc_price"]
                hit = (direction == "bullish" and btc_moved_up) or \
                      (direction == "bearish" and not btc_moved_up)
                et = entry["event_type"]
                with _state_lock:
                    if et not in _accuracy_results:
                        _accuracy_results[et] = {"hits": 0, "misses": 0, "checks": 0}
                    _accuracy_results[et]["checks"] += 1
                    if hit:
                        _accuracy_results[et]["hits"] += 1
                    else:
                        _accuracy_results[et]["misses"] += 1
            else:
                remaining_checks.append(check_ts)
        if remaining_checks:
            entry["check_at"] = remaining_checks
            still_pending.append(entry)
    _accuracy_pending[:] = still_pending


def get_accuracy() -> dict:
    """Return hit rates by event_type and overall."""
    with _state_lock:
        results = {}
        total_hits = 0
        total_checks = 0
        for et, data in _accuracy_results.items():
            checks = data["checks"]
            hits = data["hits"]
            rate = hits / checks if checks > 0 else 0
            results[et] = {"hits": hits, "misses": data["misses"],
                           "checks": checks, "hit_rate": round(rate, 3)}
            total_hits += hits
            total_checks += checks
        overall_rate = total_hits / total_checks if total_checks > 0 else 0
        return {"by_event_type": results, "overall_hit_rate": round(overall_rate, 3),
                "total_checks": total_checks}


# ═══════════════════════════════════════════════════════════════════════
#  TREND DETECTION
# ═══════════════════════════════════════════════════════════════════════
def _detect_trend(event_type: str, urgency: float, magnitude: float) -> tuple[float, float, dict | None]:
    """Check for trending event types (3+ in last hour).
    Returns (urgency_boost, magnitude_boost, meta_signal_or_None)."""
    now = time.time()
    hour_ago = now - 3600

    # Add current event
    _recent_event_types.append((now, event_type))

    # Prune old entries
    _recent_event_types[:] = [(ts, et) for ts, et in _recent_event_types if ts > hour_ago]

    # Count occurrences of this event type in last hour
    count = sum(1 for ts, et in _recent_event_types if et == event_type)

    if count >= 3:
        urgency_boost = 0.2
        magnitude_boost = 0.1
        meta_signal = None
        if count == 3:
            # Emit synthetic trend signal on the 3rd occurrence
            meta_signal = {
                "event_type": f"trend_{event_type}",
                "text": f"Trend detected: {event_type} appeared {count} times in the last hour",
                "direction": "bullish" if event_type in C.MACRO_PLAYBOOK and
                             C.MACRO_PLAYBOOK[event_type].get("direction") == "bullish" else "bearish",
                "magnitude": 0.8,
                "urgency": 0.9,
                "affects": C.MACRO_PLAYBOOK.get(event_type, {}).get("affects", []),
            }
        return (urgency_boost, magnitude_boost, meta_signal)
    return (0, 0, None)


# ═══════════════════════════════════════════════════════════════════════
#  FUZZY DEDUP (Jaccard similarity)
# ═══════════════════════════════════════════════════════════════════════
def _jaccard_similarity(text_a: str, text_b: str) -> float:
    """Compute Jaccard similarity between tokenized texts."""
    tokens_a = set(re.findall(r'\w+', text_a.lower()))
    tokens_b = set(re.findall(r'\w+', text_b.lower()))
    if not tokens_a or not tokens_b:
        return 0.0
    intersection = tokens_a & tokens_b
    union = tokens_a | tokens_b
    return len(intersection) / len(union)


def _is_fuzzy_duplicate(text: str) -> bool:
    """Check if text is too similar to recent signals (Jaccard)."""
    if not C.DEDUP_FUZZY_ENABLED:
        return False
    now = time.time()
    window = C.DEDUP_WINDOW_HOURS * 3600

    # Prune old entries
    _recent_texts[:] = [(ts, t) for ts, t in _recent_texts if now - ts < window]

    for _, prev_text in _recent_texts[-100:]:
        if _jaccard_similarity(text, prev_text) >= C.DEDUP_FUZZY_THRESHOLD:
            return True

    _recent_texts.append((now, text))
    # Cap at 100
    if len(_recent_texts) > 100:
        _recent_texts[:] = _recent_texts[-100:]
    return False


# ═══════════════════════════════════════════════════════════════════════
#  PUBLIC API — query functions
# ═══════════════════════════════════════════════════════════════════════
def get_latest_signals(hours: float = 6, affects: str = "", direction: str = "",
                       min_mag: float = 0.0, limit: int = 50) -> list[dict]:
    """Get filtered signals."""
    cutoff = time.time() - hours * 3600
    with _state_lock:
        results = []
        for s in reversed(_signals):
            if s["ts"] < cutoff:
                break
            if affects and affects not in s.get("affects", []):
                continue
            if direction and s.get("direction") != direction:
                continue
            if s.get("magnitude", 0) < min_mag:
                continue
            results.append(s)
            if len(results) >= limit:
                break
    return results

def get_sentiment(hours: float = 6) -> dict:
    """Get aggregate sentiment over time window."""
    cutoff = time.time() - hours * 3600
    sentiments = []
    with _state_lock:
        for s in reversed(_signals):
            if s["ts"] < cutoff:
                break
            sentiments.append(s.get("sentiment", 0))
    if not sentiments:
        return {"sentiment": 0.0, "regime": "neutral", "count": 0}
    avg = sum(sentiments) / len(sentiments)
    regime = "bullish" if avg > 0.15 else ("bearish" if avg < -0.15 else "neutral")
    return {"sentiment": round(avg, 3), "regime": regime, "count": len(sentiments)}

def get_regime(hours: float = 6) -> dict:
    """Get market regime based on recent signals."""
    s = get_sentiment(hours)
    return {"regime": s["regime"], "sentiment": s["sentiment"]}

def get_event_counts(hours: float = 6) -> dict:
    """Count event types in time window."""
    cutoff = time.time() - hours * 3600
    counts = defaultdict(int)
    with _state_lock:
        for s in reversed(_signals):
            if s["ts"] < cutoff:
                break
            counts[s["event_type"]] += 1
    return dict(counts)

def get_polymarket() -> list[dict]:
    with _state_lock:
        return list(_polymarket)

def get_signals_summary(hours: float = 6) -> dict:
    """All-in-one summary for downstream skills."""
    sigs = get_latest_signals(hours=hours, limit=100)
    sent = get_sentiment(hours)
    events = get_event_counts(hours)
    return {
        "sentiment": sent["sentiment"],
        "regime": sent["regime"],
        "signal_count": len(sigs),
        "event_counts": events,
        "top_events": sorted(events.items(), key=lambda x: -x[1])[:5],
        "polymarket": get_polymarket(),
        "latest_signals": sigs[:10],
    }

def get_top_senders(limit: int = 10) -> list[dict]:
    """Reputation leaderboard."""
    with _state_lock:
        items = [(sid, r["score"], r.get("hits", 0)) for sid, r in _reputation.items()]
    items.sort(key=lambda x: -x[1])
    return [{"sender": s, "score": round(sc, 2), "hits": h}
            for s, sc, h in items[:limit]]

def get_source_breakdown() -> dict:
    """Active sources with last-seen timestamps."""
    with _state_lock:
        return {k: {"last_seen": int(v), "ago_sec": int(time.time() - v)}
                for k, v in _source_status.items()}

# ═══════════════════════════════════════════════════════════════════════
#  DASHBOARD HTTP SERVER
# ═══════════════════════════════════════════════════════════════════════
def _diverse_recent_signals(max_total: int = 0, per_source: int = 0) -> list:
    """Return recent signals with per-source diversity quotas.

    Ensures minority sources (Finnhub, Polymarket, etc.) aren't buried
    by high-volume sources (OpenNews).
    """
    if max_total <= 0:
        max_total = getattr(C, "DASHBOARD_MAX_SIGNALS", 80)
    if per_source <= 0:
        per_source = getattr(C, "DASHBOARD_SOURCE_QUOTA", 5)

    # Group signals by source_type (newest first)
    by_source: dict[str, list] = {}
    for s in reversed(_signals):
        src = s.get("source_type", "unknown")
        by_source.setdefault(src, []).append(s)

    # Phase 1: Guarantee quota per source
    result = []
    used_ts = set()
    for src, sigs in by_source.items():
        for s in sigs[:per_source]:
            result.append(s)
            used_ts.add(s["ts"])

    # Phase 2: Fill remaining slots by recency
    remaining = max_total - len(result)
    if remaining > 0:
        for s in reversed(_signals):
            if s["ts"] not in used_ts:
                result.append(s)
                remaining -= 1
                if remaining <= 0:
                    break

    # Sort by timestamp descending (newest first)
    result.sort(key=lambda s: s["ts"], reverse=True)
    return result[:max_total]


def _dashboard_api_data() -> dict:
    """Full dashboard state."""
    now = time.time()
    sent = get_sentiment(6)
    with _state_lock:
        recent = _diverse_recent_signals()
        stats_copy = dict(_stats)
        sources = dict(_source_status)
    return {
        "ts": int(now),
        "uptime_sec": int(now - stats_copy.get("start_ts", now)),
        "regime": sent["regime"],
        "sentiment": sent["sentiment"],
        "signals": recent,
        "stats": stats_copy,
        "polymarket": get_polymarket(),
        "event_counts": get_event_counts(6),
        "top_senders": get_top_senders(10),
        "source_status": {k: {"last_seen": int(v), "ago_sec": int(now - v)}
                          for k, v in sources.items()},
        "telethon_active": _telethon_available,
        "fear_greed": _fear_greed,
        "fred_indicators": _fred_indicators,
        "price_tickers": _price_tickers,
        "opennews": {
            "article_count": len(_opennews_articles),
            "avg_score": _opennews_avg_score(_opennews_windowed_articles(3600)),
            "velocity": _opennews_velocity(_opennews_windowed_articles(3600), 3600).get("overall", 0),
            "ws_alive": _opennews_ws_alive,
        },
    }

class _DashboardHandler(SimpleHTTPRequestHandler):
    def log_message(self, format, *args):
        pass  # Suppress HTTP logs

    def _json_response(self, data, status=200):
        body = json.dumps(data, default=str).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        parsed = urlparse(self.path)
        path = parsed.path
        params = parse_qs(parsed.query)

        def _p(key, default):
            return params.get(key, [default])[0]

        if path == "/" or path == "/index.html":
            html_path = _BASE_DIR / "dashboard.html"
            if html_path.exists():
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.end_headers()
                self.wfile.write(html_path.read_bytes())
            else:
                self.send_response(404)
                self.end_headers()
                self.wfile.write(b"dashboard.html not found")
            return

        if path == "/api/state":
            self._json_response(_dashboard_api_data())
        elif path == "/api/signals":
            sigs = get_latest_signals(
                hours=float(_p("hours", "6")),
                affects=_p("affects", ""),
                direction=_p("direction", ""),
                min_mag=float(_p("min_mag", "0")),
                limit=int(_p("limit", "50")),
            )
            self._json_response(sigs)
        elif path == "/api/sentiment":
            self._json_response(get_sentiment(float(_p("hours", "6"))))
        elif path == "/api/regime":
            self._json_response(get_regime(float(_p("hours", "6"))))
        elif path == "/api/polymarket":
            self._json_response(get_polymarket())
        elif path == "/api/fng":
            self._json_response(_fear_greed)
        elif path == "/api/fred":
            with _state_lock:
                self._json_response(dict(_fred_indicators))
        elif path == "/api/prices":
            with _state_lock:
                self._json_response(dict(_price_tickers))
        elif path == "/api/senders":
            self._json_response(get_top_senders(int(_p("limit", "10"))))
        elif path == "/api/events":
            self._json_response(get_event_counts(float(_p("hours", "6"))))
        elif path == "/api/summary":
            self._json_response(get_signals_summary(float(_p("hours", "6"))))
        elif path == "/api/health":
            now = time.time()
            with _state_lock:
                last_sig_ts = _signals[-1]["ts"] if _signals else 0
                sig_total = len(_signals)
                sources = dict(_source_status)
            self._json_response({
                "status": "ok",
                "version": _VERSION,
                "uptime_sec": int(now - _stats.get("start_ts", now)),
                "last_signal_ts": last_sig_ts,
                "last_signal_ago_sec": int(now - last_sig_ts) if last_sig_ts else None,
                "signals_total": sig_total,
                "ws_clients": len(_ws_clients),
                "sources": {k: {"last_seen": int(v), "ago_sec": int(now - v)}
                            for k, v in sources.items()},
                "telethon_active": _telethon_available,
                "opennews_ws_alive": _opennews_ws_alive,
                "opennews_article_count": len(_opennews_articles),
            })
        elif path == "/api/accuracy":
            self._json_response(get_accuracy())
        elif path == "/api/opennews/state":
            window = int(_p("window", "3600"))
            with _state_lock:
                recent = list(_opennews_articles[-100:])
            recent.reverse()
            self._json_response({
                "articles": recent,
                "metrics": compute_opennews_metrics(window),
                "intelligence": compute_opennews_intelligence(window),
                "source_health": _opennews_source_health(),
                "ws_alive": _opennews_ws_alive,
            })
        elif path == "/api/opennews/articles":
            arts = _filter_opennews_articles(
                engine=_p("engine", ""),
                signal=_p("signal", ""),
                min_score=int(_p("min_score", "0")),
                coin=_p("coin", ""),
                q=_p("q", ""),
                limit=int(_p("limit", "100")),
            )
            self._json_response(arts)
        elif path == "/api/opennews/metrics":
            window = int(_p("window", "3600"))
            self._json_response(compute_opennews_metrics(window))
        elif path == "/api/opennews/sources":
            self._json_response(_opennews_source_health())
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        parsed = urlparse(self.path)
        path = parsed.path
        content_len = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_len) if content_len else b""

        if path == "/api/vote":
            try:
                data = json.loads(body) if body else {}
                key = data.get("signal_key", "")
                vote = int(data.get("vote", 0))  # +1 or -1
                if key and vote in (1, -1):
                    with _state_lock:
                        _signal_votes[key] = _signal_votes.get(key, 0) + vote
                    self._json_response({"status": "ok", "net_votes": _signal_votes.get(key, 0)})
                else:
                    self._json_response({"error": "need signal_key and vote (+1/-1)"}, 400)
            except Exception as e:
                self._json_response({"error": str(e)}, 400)
        else:
            self.send_response(404)
            self.end_headers()

    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()

# ═══════════════════════════════════════════════════════════════════════
#  SETUP MODE — list Telegram groups
# ═══════════════════════════════════════════════════════════════════════
async def _setup_mode():
    """Interactive: list all Telegram groups/channels for config."""
    api_id = C.TELETHON_API_ID or int(os.environ.get("TG_API_ID", "0"))
    api_hash = C.TELETHON_API_HASH or os.environ.get("TG_API_HASH", "")
    if not api_id or not api_hash:
        print("Set TELETHON_API_ID and TELETHON_API_HASH in config.py first.")
        return

    client = TelegramClient(str(_BASE_DIR / C.SESSION_NAME), api_id, api_hash)
    await client.start()
    print("\nYour Telegram Groups & Channels:\n")
    print(f"{'Type':<10} {'ID':<20} {'Title'}")
    print("-" * 60)

    async for dialog in client.iter_dialogs():
        entity = dialog.entity
        if isinstance(entity, (Channel, Chat)):
            dtype = "channel" if getattr(entity, 'broadcast', False) else "group"
            print(f"{dtype:<10} {entity.id:<20} {dialog.name}")

    await client.disconnect()
    print("\nAdd IDs or usernames to config.py GROUPS/CHANNELS dicts.")

# ═══════════════════════════════════════════════════════════════════════
#  COMPILE REGEX CACHES
# ═══════════════════════════════════════════════════════════════════════
def _compile_patterns():
    """Pre-compile all regex patterns for performance."""
    global _macro_regex, _noise_regex
    for event_type, patterns in C.MACRO_KEYWORDS.items():
        _macro_regex[event_type] = [re.compile(p) for p in patterns]
    _noise_regex = [re.compile(p, re.IGNORECASE) for p in C.NOISE_SKIP_PATTERNS]

# ═══════════════════════════════════════════════════════════════════════
#  MAIN
# ═══════════════════════════════════════════════════════════════════════
def main():
    # Setup mode
    if len(sys.argv) > 1 and sys.argv[1] == "setup":
        if not _telethon_available:
            print("Install telethon first: pip install telethon")
            return
        import asyncio
        asyncio.run(_setup_mode())
        return

    _compile_patterns()
    _load_state()
    _stats["start_ts"] = time.time()

    _log("=" * 60)
    _log(f"Macro Intelligence Skill v{_VERSION} — Intelligence Feed")
    _log(f"Dashboard:   http://localhost:{C.DASHBOARD_PORT}")
    _log(f"WebSocket:   {'ws://localhost:' + str(C.WS_PORT) if C.WS_ENABLED else 'disabled'}")
    _log(f"Telethon:    {'available' if _telethon_available else 'NOT installed'}")
    _log(f"OpenNews:    {'enabled' if C.OPENNEWS_ENABLED and C.OPENNEWS_TOKEN else 'disabled'}")
    _log(f"Finnhub:     {'enabled' if C.FINNHUB_ENABLED and C.FINNHUB_API_KEY else 'disabled'}")
    _log(f"FRED:        {'enabled' if C.FRED_ENABLED and C.FRED_API_KEY else 'disabled'}")
    _log(f"CryptoPanic: {'enabled' if C.CRYPTOPANIC_ENABLED and C.CRYPTOPANIC_TOKEN else 'disabled'}")
    _log(f"RSS feeds:   {len(C.RSS_FEEDS)} configured" if C.RSS_ENABLED else "RSS: disabled")
    _log(f"Webhooks:    {len(C.WEBHOOK_URLS)} configured" if C.WEBHOOK_URLS else "Webhooks: none")
    _log(f"Accuracy:    {'enabled' if C.ACCURACY_ENABLED else 'disabled'}")
    _log(f"Fuzzy dedup: {'enabled' if C.DEDUP_FUZZY_ENABLED else 'disabled'}")
    _log("=" * 60)

    # Start WebSocket server (if enabled)
    if C.WS_ENABLED:
        _start_ws_server_thread()

    # Start news collector thread
    news_thread = threading.Thread(target=_news_collector_loop, daemon=True, name="news_collector")
    news_thread.start()

    # Start Telegram collector (if available)
    if _telethon_available:
        _start_telethon_thread()
    else:
        _log("Telethon not installed — run: pip install telethon", "WARN")

    # Start 6551.io OpenNews WebSocket (if configured)
    if C.OPENNEWS_ENABLED and C.OPENNEWS_TOKEN:
        _start_opennews_thread()
        _log("OpenNews: WebSocket thread started")
    else:
        _log("OpenNews: disabled (no OPENNEWS_TOKEN)", "WARN")

    # Start HTTP dashboard
    server = HTTPServer(("0.0.0.0", C.DASHBOARD_PORT), _DashboardHandler)
    _log(f"HTTP server listening on :{C.DASHBOARD_PORT}")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        _log("Shutting down...")
        _save_state()
        server.shutdown()

if __name__ == "__main__":
    main()
