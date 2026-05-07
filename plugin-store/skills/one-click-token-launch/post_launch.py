"""
一键发币 v1.0 — Post-Launch Monitor

Shows live token stats in terminal after launch:
  - Bonding curve progress (pump.fun)
  - Price, market cap, volume
  - Holder count + distribution
  - Buy/sell activity
  - Auto-refresh every N seconds

Usage:
  from post_launch import monitor
  await monitor("8BWqp55pn6GvpKFZkqAAeytc5H6P7KBNpStMaRUz3a8U")
"""
from __future__ import annotations

import asyncio
import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

# Ensure skill directory is on sys.path
_SKILL_DIR = str(Path(__file__).resolve().parent)
if _SKILL_DIR not in sys.path:
    sys.path.insert(0, _SKILL_DIR)

from launchpads.base import onchainos_bin
_ONCHAINOS = onchainos_bin()

_SOLANA_EXPLORER = "https://solscan.io/tx"
_PUMPFUN_TRADE = "https://pump.fun"

# ── Box drawing chars ────────────────────────────────────────────────
_TL = "╔"; _TR = "╗"; _BL = "╚"; _BR = "╝"
_H = "═"; _V = "║"; _ML = "╠"; _MR = "╣"

# ── ANSI colors ──────────────────────────────────────────────────────
_RESET = "\033[0m"
_BOLD  = "\033[1m"
_DIM   = "\033[2m"
_GREEN = "\033[32m"
_RED   = "\033[31m"
_CYAN  = "\033[36m"
_YELLOW = "\033[33m"
_WHITE = "\033[37m"
_BG_GREEN = "\033[42m"
_BG_DIM   = "\033[48;5;236m"


# ══════════════════════════════════════════════════════════════════════
# Data Fetching (via onchainos CLI)
# ══════════════════════════════════════════════════════════════════════

def _cli(*args: str, timeout: int = 15) -> dict:
    """Run onchainos CLI and return parsed JSON."""
    cmd = [_ONCHAINOS] + list(args)
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        if proc.returncode != 0:
            return {}
        return json.loads(proc.stdout)
    except Exception:
        return {}


def fetch_memepump_details(token: str, chain: str = "501", wallet: str = "") -> dict:
    """Fetch pump.fun token details (bonding curve, holders, market data)."""
    args = ["memepump", "token-details", "--chain", chain, "--address", token]
    if wallet:
        args.extend(["--wallet", wallet])
    result = _cli(*args)
    return result.get("data", {}) if result else {}


def fetch_price_info(token: str, chain: str = "501") -> dict:
    """Fetch detailed price info (price, mcap, volume, change)."""
    result = _cli("token", "price-info", "--chain", chain, "--address", token, timeout=10)
    return result.get("data", {}) if result else {}


def fetch_holders(token: str, chain: str = "501") -> list:
    """Fetch top holders list."""
    result = _cli("token", "holders", "--chain", chain, "--address", token)
    data = result.get("data", [])
    return data if isinstance(data, list) else []


def fetch_trades(token: str, chain: str = "501", limit: int = 20) -> list:
    """Fetch recent trades."""
    result = _cli("token", "trades", "--chain", chain, "--address", token, "--limit", str(limit))
    data = result.get("data", [])
    return data if isinstance(data, list) else []


# ══════════════════════════════════════════════════════════════════════
# Stats Aggregation
# ══════════════════════════════════════════════════════════════════════

def gather_stats(token: str, chain: str = "501", launchpad: str = "pumpfun", wallet: str = "") -> dict:
    """Gather all stats for a token. Returns a flat dict."""
    stats = {
        "token": token,
        "chain": chain,
        "launchpad": launchpad,
        "name": "",
        "symbol": "",
        "bonding_pct": None,
        "price_usd": None,
        "mcap_usd": None,
        "volume_1h": None,
        "buy_count_1h": None,
        "sell_count_1h": None,
        "tx_count_1h": None,
        "holders": None,
        "top10_pct": None,
        "bundler_pct": None,
        "sniper_pct": None,
        "dev_hold_pct": None,
        "fresh_wallet_pct": None,
        "creator": "",
        "created_ts": None,
        "status": "UNKNOWN",
        "twitter": "",
        "telegram": "",
        "website": "",
        "dexscreener_paid": False,
        "cto": False,
        "top_holders": [],
        "recent_trades": [],
        "fetch_time": time.time(),
    }

    # ── Fetch all data concurrently ──────────────────────────────────
    is_pump = launchpad in ("pumpfun", "letsbonk")

    with ThreadPoolExecutor(max_workers=4) as pool:
        f_details = pool.submit(fetch_memepump_details, token, chain, wallet) if is_pump else None
        f_price   = pool.submit(fetch_price_info, token, chain)
        f_holders = pool.submit(fetch_holders, token, chain)
        f_trades  = pool.submit(fetch_trades, token, chain, 10)

        details      = f_details.result() if f_details else {}
        price_data   = f_price.result()
        holders_list = f_holders.result()
        trades       = f_trades.result()

    # ── Pump.fun details (primary data source) ──────────────────────
    if is_pump and details:
        stats["name"] = details.get("name", "")
        stats["symbol"] = details.get("symbol", "")
        stats["creator"] = details.get("creatorAddress", "")

        bp = details.get("bondingPercent", "")
        stats["bonding_pct"] = float(bp) if bp else 0.0

        ts = details.get("createdTimestamp", "")
        stats["created_ts"] = int(ts) / 1000 if ts else None

        # Market data
        mkt = details.get("market", {})
        mcap = mkt.get("marketCapUsd", "")
        stats["mcap_usd"] = float(mcap) if mcap else None
        vol = mkt.get("volumeUsd1h", "")
        stats["volume_1h"] = float(vol) if vol else None
        bc = mkt.get("buyTxCount1h", "")
        stats["buy_count_1h"] = int(bc) if bc else 0
        sc = mkt.get("sellTxCount1h", "")
        stats["sell_count_1h"] = int(sc) if sc else 0
        tc = mkt.get("txCount1h", "")
        stats["tx_count_1h"] = int(tc) if tc else 0

        # Tags
        tags = details.get("tags", {})
        h = tags.get("totalHolders", "")
        stats["holders"] = int(h) if h else 0
        t10 = tags.get("top10HoldingsPercent", "")
        stats["top10_pct"] = float(t10) if t10 else 0.0
        stats["bundler_pct"] = float(tags.get("bundlersPercent", "0") or "0")
        stats["sniper_pct"] = float(tags.get("snipersPercent", "0") or "0")
        stats["dev_hold_pct"] = float(tags.get("devHoldingsPercent", "0") or "0")
        stats["fresh_wallet_pct"] = float(tags.get("freshWalletsPercent", "0") or "0")

        # Social
        social = details.get("social", {})
        stats["twitter"] = social.get("x", "")
        stats["telegram"] = social.get("telegram", "")
        stats["website"] = social.get("website", "")
        stats["dexscreener_paid"] = social.get("dexScreenerPaid", False)
        stats["cto"] = social.get("communityTakeover", False)

        # Status
        migrated_end = details.get("migratedEndTimestamp", "")
        migrating = details.get("migratedBeginTimestamp", "")
        if migrated_end:
            stats["status"] = "MIGRATED"
        elif migrating:
            stats["status"] = "MIGRATING"
        elif stats["bonding_pct"] is not None:
            stats["status"] = "BONDING CURVE"
        else:
            stats["status"] = "ACTIVE"

    # ── Price info (supplement) ─────────────────────────────────────
    if price_data:
        p = price_data.get("price", "") or price_data.get("priceUsd", "")
        if p:
            stats["price_usd"] = float(p)
        if not stats["mcap_usd"]:
            m = price_data.get("marketCapUsd", "") or price_data.get("marketCap", "")
            if m:
                stats["mcap_usd"] = float(m)
        if not stats["name"]:
            stats["name"] = price_data.get("tokenName", "") or price_data.get("name", "")
        if not stats["symbol"]:
            stats["symbol"] = price_data.get("tokenSymbol", "") or price_data.get("symbol", "")

    # ── Holders (supplement — count from list if needed) ────────────
    if holders_list:
        stats["top_holders"] = holders_list[:5]
        if not stats["holders"] or stats["holders"] < len(holders_list):
            stats["holders"] = max(stats["holders"] or 0, len(holders_list))

    # ── Recent trades ───────────────────────────────────────────────
    if trades:
        stats["recent_trades"] = trades[:5]

    return stats


# ══════════════════════════════════════════════════════════════════════
# Terminal Display
# ══════════════════════════════════════════════════════════════════════

def _bar(pct: float, width: int = 20) -> str:
    """Render a progress bar."""
    pct = max(0.0, min(pct, 100.0))
    filled = int(pct / 100 * width)
    empty = width - filled
    bar = f"{_BG_GREEN}{_WHITE}" + "█" * filled + f"{_RESET}"
    bar += f"{_BG_DIM}" + "░" * empty + f"{_RESET}"
    return bar


def _fmt_usd(val: Optional[float]) -> str:
    if val is None:
        return f"{_DIM}—{_RESET}"
    if val >= 1_000_000:
        return f"${val / 1_000_000:.2f}M"
    if val >= 1_000:
        return f"${val / 1_000:.1f}K"
    if val >= 1:
        return f"${val:.2f}"
    return f"${val:.6f}"


def _fmt_price(val: Optional[float]) -> str:
    if val is None:
        return f"{_DIM}—{_RESET}"
    if val >= 1:
        return f"${val:.4f}"
    if val >= 0.0001:
        return f"${val:.6f}"
    return f"${val:.10f}"


def _fmt_pct(val: Optional[float]) -> str:
    if val is None:
        return f"{_DIM}—{_RESET}"
    return f"{val:.1f}%"


def _fmt_int(val: Optional[int]) -> str:
    if val is None or val == 0:
        return f"{_DIM}0{_RESET}"
    if val >= 1_000_000:
        return f"{val / 1_000_000:.1f}M"
    if val >= 1_000:
        return f"{val / 1_000:.1f}K"
    return str(val)


def _age_str(created_ts: Optional[float]) -> str:
    if not created_ts:
        return f"{_DIM}—{_RESET}"
    elapsed = time.time() - created_ts
    if elapsed < 0:
        return "just now"
    if elapsed < 60:
        return f"{int(elapsed)}s"
    if elapsed < 3600:
        m = int(elapsed // 60)
        s = int(elapsed % 60)
        return f"{m}m {s}s"
    if elapsed < 86400:
        h = int(elapsed // 3600)
        m = int((elapsed % 3600) // 60)
        return f"{h}h {m}m"
    d = int(elapsed // 86400)
    h = int((elapsed % 86400) // 3600)
    return f"{d}d {h}h"


def _status_color(status: str) -> str:
    if status == "MIGRATED":
        return _GREEN
    if status == "MIGRATING":
        return _YELLOW
    if status == "BONDING CURVE":
        return _CYAN
    return _WHITE


def _short(addr: str, n: int = 6) -> str:
    if len(addr) <= n * 2 + 3:
        return addr
    return f"{addr[:n]}...{addr[-4:]}"


def render(stats: dict, width: int = 58) -> str:
    """Render stats into a terminal box string."""
    inner = width - 4  # space inside box (between ║ and ║)

    name = stats.get("name", "?")
    symbol = stats.get("symbol", "?")
    token = stats.get("token", "")
    launchpad = stats.get("launchpad", "")

    lp_display = {
        "pumpfun": "pump.fun", "letsbonk": "LetsBonk", "bags": "Bags.fm",
        "moonit": "Moonit", "fourmeme": "Four.Meme", "flap": "Flap.sh",
    }.get(launchpad, launchpad)

    lines = []

    # Top border
    lines.append(f"{_CYAN}{_TL}{_H * (width - 2)}{_TR}{_RESET}")

    # Title
    title = f"  {_BOLD}{name}{_RESET} ({symbol}) — {lp_display}"
    lines.append(f"{_CYAN}{_V}{_RESET}{title:<{inner + 20}}{_CYAN}{_V}{_RESET}")

    # Token address
    addr_line = f"  {_DIM}{token}{_RESET}"
    lines.append(f"{_CYAN}{_V}{_RESET}{addr_line:<{inner + 12}}{_CYAN}{_V}{_RESET}")

    # Separator
    lines.append(f"{_CYAN}{_ML}{_H * (width - 2)}{_MR}{_RESET}")

    # ── Stats rows ──────────────────────────────────────────────────
    def row(label: str, value: str, label_w: int = 16):
        padded_label = f"  {label:<{label_w}}"
        return f"{_CYAN}{_V}{_RESET}{padded_label}{value}"

    # Bonding curve (pump.fun only)
    bp = stats.get("bonding_pct")
    if bp is not None:
        bar = _bar(bp)
        bp_str = f"{bp:.1f}%"
        color = _GREEN if bp >= 80 else _YELLOW if bp >= 50 else _WHITE
        lines.append(row("Bonding Curve", f"{bar} {color}{bp_str}{_RESET}"))

    # Price
    lines.append(row("Price", f"{_BOLD}{_fmt_price(stats.get('price_usd'))}{_RESET}"))

    # Market cap
    lines.append(row("Market Cap", _fmt_usd(stats.get("mcap_usd"))))

    # Volume
    lines.append(row("Volume (1h)", _fmt_usd(stats.get("volume_1h"))))

    # Holders
    h = stats.get("holders")
    lines.append(row("Holders", f"{_BOLD}{_fmt_int(h)}{_RESET}"))

    # Buy / Sell
    buys = stats.get("buy_count_1h", 0) or 0
    sells = stats.get("sell_count_1h", 0) or 0
    bs = f"{_GREEN}{buys}{_RESET} / {_RED}{sells}{_RESET}"
    lines.append(row("Buys / Sells", bs))

    # Top 10 %
    lines.append(row("Top 10 Hold%", _fmt_pct(stats.get("top10_pct"))))

    # Dev holdings
    dev = stats.get("dev_hold_pct") or 0
    dev_color = _RED if dev > 5 else _YELLOW if dev > 2 else _GREEN
    lines.append(row("Dev Holdings", f"{dev_color}{_fmt_pct(dev)}{_RESET}"))

    # Bundlers + Snipers
    bund = stats.get("bundler_pct") or 0
    snip = stats.get("sniper_pct") or 0
    if bund or snip:
        warn_color = _RED if (bund > 10 or snip > 10) else _YELLOW
        lines.append(row("Bundle / Sniper", f"{warn_color}{_fmt_pct(bund)} / {_fmt_pct(snip)}{_RESET}"))

    # Age
    lines.append(row("Age", _age_str(stats.get("created_ts"))))

    # Separator
    lines.append(f"{_CYAN}{_ML}{_H * (width - 2)}{_MR}{_RESET}")

    # Status
    status = stats.get("status", "UNKNOWN")
    sc = _status_color(status)
    lines.append(row("Status", f"{sc}{_BOLD}{status}{_RESET}"))

    # Creator
    creator = stats.get("creator", "")
    if creator:
        lines.append(row("Creator", f"{_DIM}{_short(creator, 8)}{_RESET}"))

    # Flags
    flags = []
    if stats.get("cto"):
        flags.append(f"{_YELLOW}CTO{_RESET}")
    if stats.get("dexscreener_paid"):
        flags.append(f"{_GREEN}DS Paid{_RESET}")
    if flags:
        lines.append(row("Flags", " ".join(flags)))

    # Links
    links = []
    if stats.get("twitter"):
        links.append("X")
    if stats.get("telegram"):
        links.append("TG")
    if stats.get("website"):
        links.append("Web")
    if links:
        lines.append(row("Socials", f"{_DIM}{' | '.join(links)}{_RESET}"))

    # Bottom border
    lines.append(f"{_CYAN}{_BL}{_H * (width - 2)}{_BR}{_RESET}")

    # Trade page
    if launchpad in ("pumpfun", "letsbonk"):
        lines.append(f"  {_DIM}{_PUMPFUN_TRADE}/{token}{_RESET}")

    return "\n".join(lines)


# ══════════════════════════════════════════════════════════════════════
# Recent Trades Display
# ══════════════════════════════════════════════════════════════════════

def render_trades(stats: dict) -> str:
    """Render recent trades summary."""
    trades = stats.get("recent_trades", [])
    if not trades:
        return ""

    lines = [f"\n  {_BOLD}Recent Trades{_RESET}"]
    for t in trades[:5]:
        side = t.get("tradeDirection", t.get("side", ""))
        is_buy = side in ("buy", "1")
        color = _GREEN if is_buy else _RED
        direction = "BUY " if is_buy else "SELL"

        amount = t.get("tradeAmount", t.get("amount", ""))
        price = t.get("price", "")
        wallet = t.get("makerAddress", t.get("traderAddress", ""))
        tag = t.get("traderTag", "")

        tag_str = ""
        tag_map = {"1": "KOL", "2": "Dev", "3": "SM", "4": "Whale", "5": "New", "7": "Sniper", "9": "Bundle"}
        if tag and tag in tag_map:
            tag_str = f" [{tag_map[tag]}]"

        lines.append(
            f"  {color}{direction}{_RESET} "
            f"{_short(wallet, 4)} "
            f"{_DIM}{tag_str}{_RESET}"
        )

    return "\n".join(lines)


# ══════════════════════════════════════════════════════════════════════
# Top Holders Display
# ══════════════════════════════════════════════════════════════════════

def render_holders(stats: dict) -> str:
    """Render top holders summary."""
    holders = stats.get("top_holders", [])
    if not holders:
        return ""

    lines = [f"\n  {_BOLD}Top Holders{_RESET}"]
    for i, h in enumerate(holders[:5], 1):
        addr = h.get("holderWalletAddress", "")
        pct = h.get("holdPercent", "0")
        pnl = h.get("totalPnlUsd", "")

        pct_f = float(pct) if pct else 0
        pnl_str = ""
        if pnl:
            pnl_f = float(pnl)
            pnl_color = _GREEN if pnl_f >= 0 else _RED
            pnl_str = f" {pnl_color}{_fmt_usd(pnl_f)}{_RESET}"

        lines.append(
            f"  {_DIM}{i}.{_RESET} {_short(addr, 6)} "
            f"{_BOLD}{pct_f:.1f}%{_RESET}{pnl_str}"
        )

    return "\n".join(lines)


# ══════════════════════════════════════════════════════════════════════
# Main Monitor Loop
# ══════════════════════════════════════════════════════════════════════

async def monitor(
    token: str,
    chain: str = "501",
    launchpad: str = "pumpfun",
    wallet: str = "",
    refresh: int = 10,
    max_rounds: int = 0,
) -> None:
    """Live post-launch monitor. Refreshes every `refresh` seconds.

    Args:
        token: Token contract address
        chain: Chain index (501=Solana, 56=BSC)
        launchpad: pumpfun, bags, letsbonk, moonit, fourmeme, flap
        wallet: Optional wallet address (for position/PnL tracking)
        refresh: Seconds between refreshes
        max_rounds: 0 = infinite, >0 = stop after N rounds
    """
    round_num = 0

    try:
        while True:
            round_num += 1

            # Fetch (blocking — runs onchainos CLI calls)
            stats = await asyncio.get_event_loop().run_in_executor(
                None, gather_stats, token, chain, launchpad, wallet
            )

            # Clear screen
            print("\033[2J\033[H", end="")

            # Render
            print(render(stats))
            print(render_holders(stats))
            print(render_trades(stats))

            # Footer
            elapsed = time.time() - stats["fetch_time"]
            print(f"\n  {_DIM}Updated {elapsed:.0f}s ago | Refresh: {refresh}s | Ctrl+C to stop{_RESET}")

            if max_rounds and round_num >= max_rounds:
                break

            await asyncio.sleep(refresh)

    except KeyboardInterrupt:
        print(f"\n  {_DIM}Monitor stopped.{_RESET}")


def monitor_sync(
    token: str,
    chain: str = "501",
    launchpad: str = "pumpfun",
    wallet: str = "",
    refresh: int = 10,
    max_rounds: int = 0,
) -> None:
    """Synchronous wrapper for monitor()."""
    asyncio.run(monitor(token, chain, launchpad, wallet, refresh, max_rounds))


# ══════════════════════════════════════════════════════════════════════
# One-shot snapshot (no loop)
# ══════════════════════════════════════════════════════════════════════

def snapshot(token: str, chain: str = "501", launchpad: str = "pumpfun", wallet: str = "") -> dict:
    """Fetch and display a single snapshot. Returns the stats dict."""
    stats = gather_stats(token, chain, launchpad, wallet)
    print(render(stats))
    print(render_holders(stats))
    print(render_trades(stats))
    return stats


# ══════════════════════════════════════════════════════════════════════
# CLI Entry
# ══════════════════════════════════════════════════════════════════════

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 post_launch.py <token_address> [--refresh 10] [--chain 501] [--launchpad pumpfun]")
        sys.exit(1)

    token_addr = sys.argv[1]
    refresh_s = 10
    chain_id = "501"
    lp = "pumpfun"

    args = sys.argv[2:]
    i = 0
    while i < len(args):
        if args[i] == "--refresh" and i + 1 < len(args):
            refresh_s = int(args[i + 1]); i += 2
        elif args[i] == "--chain" and i + 1 < len(args):
            chain_id = args[i + 1]; i += 2
        elif args[i] == "--launchpad" and i + 1 < len(args):
            lp = args[i + 1]; i += 2
        else:
            i += 1

    monitor_sync(token_addr, chain_id, lp, refresh=refresh_s)
