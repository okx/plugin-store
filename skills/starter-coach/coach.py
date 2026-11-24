"""
Coaching engine — 6-step conversational flow for strategy creation.

Manages state transitions through the coaching journey:
  Step 1: Onboarding & User Activation
  Step 2: User Profiling
  Step 3: Customize & Build Trading Strategy
  Step 4: Paper Trade or Backtest
  Step 5: Go Live (User's Choice)
  Step 6: Auto-Evolve Engine (Optional)

State is persisted as JSON so conversations can resume across sessions.
"""
from __future__ import annotations

import json
import os as _os
import sys as _sys
import time
import unicodedata
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any


def _detect_env() -> str:
    """Return 'terminal' if running in a real TTY, 'chat' otherwise.

    Override via env var:  STARTER_COACH_ENV=terminal  or  =chat
    This lets Telegram / WeChat agents force plain-text mode explicitly.
    """
    override = _os.environ.get("STARTER_COACH_ENV", "").lower()
    if override in ("terminal", "chat"):
        return override
    try:
        return "terminal" if _sys.stdout.isatty() else "chat"
    except Exception:
        return "chat"


COACH_ENV: str = _detect_env()  # "terminal" | "chat"

from harness import validate_spec
from llm_strategy import generate_strategy_spec, get_fallback_theme
from paper_gate import (
    check_graduation,
    get_allowed_size_multiplier,
    record_paper_trade,
)

STATE_DIR = Path(__file__).parent / ".coach_state"

# ── Profile Questions ─────────────────────────────────────────────

_FIGLET_ART = """███████╗████████╗ █████╗ ██████╗ ████████╗███████╗██████╗
██╔════╝╚══██╔══╝██╔══██╗██╔══██╗╚══██╔══╝██╔════╝██╔══██╗
███████╗   ██║   ███████║██████╔╝   ██║   █████╗  ██████╔╝
╚════██║   ██║   ██╔══██║██╔══██╗   ██║   ██╔══╝  ██╔══██╗
███████║   ██║   ██║  ██║██║  ██║   ██║   ███████╗██║  ██║
╚══════╝   ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   ╚══════╝╚═╝  ╚═╝
 ██████╗ ██████╗  █████╗  ██████╗██╗  ██╗
██╔════╝██╔═══██╗██╔══██╗██╔════╝██║  ██║
██║     ██║   ██║███████║██║     ███████║
██║     ██║   ██║██╔══██║██║     ██╔══██║
╚██████╗╚██████╔╝██║  ██║╚██████╗██║  ██║
 ╚═════╝ ╚═════╝ ╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝"""

_SUB_TEXT = "  VIBE  TRADING  ASSISTANT  "
_SUB_W = len(_SUB_TEXT)
_SUBTITLE = (
    " \u2554" + "\u2550" * _SUB_W + "\u2557\n"
    " \u2551" + _SUB_TEXT + "\u2551\n"
    " \u255a" + "\u2550" * _SUB_W + "\u255d"
)

WELCOME_BANNER = f"\n```\n{_FIGLET_ART}\n\n{_SUBTITLE}\n```\n"

_WELCOME_BODY_EN = (
    "Welcome Builder! I see that you have made your way here, which means "
    "you need my help. Don't worry, I am here to help. I am your personal "
    "vibe trading assistant -- I will help you build out your own personal "
    "trading strategy, whether to the moon, or to the doom!\n\n"
    "Before we cook up something legendary, I need to know what kind of "
    "degen (or not) you are. Quick vibe check incoming..."
)

_WELCOME_BODY_ZH = (
    "\u6b22\u8fce\u6765\u5230\u8fd9\u91cc Builder\uff01\u65e2\u7136\u4f60\u627e\u5230\u4e86\u6211\uff0c"
    "\u8bf4\u660e\u4f60\u9700\u8981\u6211\u7684\u5e2e\u52a9\u3002\u522b\u62c5\u5fc3\uff0c\u6211\u662f\u4f60\u7684\u4e13\u5c5e "
    "Vibe \u4ea4\u6613\u52a9\u624b\u2014\u2014\u5e2e\u4f60\u6253\u9020\u5c5e\u4e8e\u4f60\u81ea\u5df1\u7684\u4ea4\u6613\u7b56\u7565\uff0c"
    "\u4e0a\u6708\u7403\u6216\u8005\u4e0b\u5730\u72f1\uff0c\u90fd\u7531\u4f60\u8bf4\u4e86\u7b97\uff01\n\n"
    "\u5728\u5f00\u59cb\u4e4b\u524d\uff0c\u6211\u5f97\u5148\u4e86\u89e3\u4e00\u4e0b\u4f60\u662f\u54ea\u79cd\u7c7b\u578b\u7684\u4ea4\u6613\u8005\u2026"
)

# With ASCII banner — terminal only
WELCOME_MESSAGE_EN = WELCOME_BANNER + "\n" + _WELCOME_BODY_EN
WELCOME_MESSAGE_ZH = WELCOME_BANNER + "\n" + _WELCOME_BODY_ZH

# Plain — no ASCII art, for Claude app / web / Telegram / other agents
WELCOME_MESSAGE_EN_PLAIN = "## 🤖 Starter Coach — Vibe Trading Assistant\n\n" + _WELCOME_BODY_EN
WELCOME_MESSAGE_ZH_PLAIN = "## 🤖 Starter Coach — Vibe 交易助手\n\n" + _WELCOME_BODY_ZH

# Default — auto-detected: ASCII art in terminal, plain markdown in chat/agents
WELCOME_MESSAGE = WELCOME_MESSAGE_EN if COACH_ENV == "terminal" else WELCOME_MESSAGE_EN_PLAIN

# ── Legal Disclaimer ──────────────────────────────────────────────

DISCLAIMER_MESSAGE = (
    "\n---\n"
    "**⚠️ Before we start — please read:**\n\n"
    "This tool helps you *build* your own automated strategy. It does **not** provide "
    "investment advice, financial recommendations, or any guarantee of profit. "
    "All strategies you create are your own — you configure them, you run them, "
    "and you are fully responsible for any outcomes.\n\n"
    "Crypto markets are highly volatile. You may lose some or all of your funds. "
    "Never invest more than you can afford to lose.\n\n"
    "*By continuing, you confirm you understand this.*\n"
    "---\n"
)

LIVE_MODE_DISCLAIMER = (
    "\n🔴 **SWITCHING TO LIVE MODE — REAL FUNDS AT RISK**\n\n"
    "Before going live, confirm you understand:\n"
    "- **Real money** will be used — losses are real and irreversible\n"
    "- This bot does **not guarantee** profitable outcomes\n"
    "- You assume **full liability** for all transactions executed\n"
    "- Safety filters reduce but **do not eliminate** risk of fraud or loss\n"
    "- High-frequency trading may generate **significant taxable events**\n\n"
    "Type `CONFIRM` to proceed to live mode, or anything else to stay in paper mode.\n"
)

# Smart defaults (no explicit questions needed):
#   token   → inferred from goal via _infer_token_from_goal()
#   chain   → inferred from token via _infer_chain_from_token()
#   automation → inferred from risk via _infer_automation_from_risk()
#   experience → defaults to "intermediate" (conservative features disabled)

PROFILE_QUESTIONS = [
    # Q1: identity hook — what kind of trader are you?
    {
        "key": "goal",
        "question": "What's your vibe? Pick your fighter:",
        "question_zh": "你的交易风格是？选一个：",
        "options": [
            {"icon": "🎯", "label": "Stack sats on autopilot", "label_zh": "自动定投", "tag": "DCA gang", "tag_zh": "DCA党"},
            {"icon": "📉", "label": "Buy the dip", "label_zh": "跌了就买", "tag": "blood in the streets is my perfume", "tag_zh": "别人恐惧我贪婪"},
            {"icon": "🐋", "label": "Follow smart money", "label_zh": "跟踪聪明钱", "tag": "whale watching", "tag_zh": "鲸鱼观察员"},
            {"icon": "📋", "label": "Copy-trade a wallet", "label_zh": "跟单钱包", "tag": "I found an alpha wallet", "tag_zh": "我找到alpha了"},
            {"icon": "🔫", "label": "Snipe new tokens", "label_zh": "狙击新币", "tag": "degen sniper", "tag_zh": "超级狙击手"},
            {"icon": "📈", "label": "Ride the trend", "label_zh": "趋势跟踪", "tag": "momentum is my religion", "tag_zh": "动量信仰"},
            {"icon": "💬", "label": "Something else", "label_zh": "其他想法", "tag": "let me tell you what I want", "tag_zh": "让我说说"},
        ],
        "display": "box",
        "map_to": {
            "DCA": "time_schedule", "定投": "time_schedule",
            "dip": "price_drop", "跌": "price_drop",
            "smart money": "smart_money_buy", "聪明钱": "smart_money_buy",
            "copy": "wallet_copy_buy", "跟单": "wallet_copy_buy",
            "snipe": "ranking_entry", "狙击": "ranking_entry",
            "grid": "grid", "网格": "grid",
            "trend": "ma_cross", "趋势": "ma_cross",
            "mean revert": "rsi_threshold", "均值": "rsi_threshold",
        },
    },
    # Q2: outcome-based risk — no trading jargon required
    {
        "key": "risk_level",
        "question": "If this trade goes wrong, what's your vibe?",
        "question_zh": "如果这笔交易亏了，你的反应是？",
        "options": [
            {"icon": "😴", "label": "Sleep easy", "label_zh": "睡得着觉", "tag": "protect my money, small steady bets", "tag_zh": "保住本金，小步稳走"},
            {"icon": "😬", "label": "Shake it off", "label_zh": "亏了扛着", "tag": "some risk is fine, medium bets", "tag_zh": "能承受一些损失，中等仓位"},
            {"icon": "💀", "label": "Send it", "label_zh": "梭哈！", "tag": "go big or go home, I know the risks", "tag_zh": "搏一搏，风险我承担"},
            {"icon": "💬", "label": "Let me explain", "label_zh": "我来说说", "tag": "my situation is different", "tag_zh": "我的情况比较特殊"},
        ],
        "display": "box",
        "map_to_params": {
            "conservative": {"sl_pct": 8, "sizing_pct": 2, "max_dd": 8},
            "moderate": {"sl_pct": 15, "sizing_pct": 5, "max_dd": 12},
            "aggressive": {"sl_pct": 20, "sizing_pct": 10, "max_dd": 15},
        },
    },
    # Q3: conditional — only for copy/smart money goals
    {
        "key": "target_wallets",
        "question": "Drop the wallet address(es) you want to mirror (up to 3):",
        "question_zh": "贴上要跟单的钱包地址（最多3个）：",
        "conditional": "goal == copy or goal == smart money",
        "type": "wallet_list",
        "guidance": "Paste the wallet addresses of the alpha traders you found.",
        "guidance_zh": "贴上你找到的alpha交易员钱包地址。",
    },
    # Q4: total budget — easier to think about than per-trade sizing
    {
        "key": "total_budget",
        "question": "Last thing — what's your total budget for this strategy? (USD)",
        "question_zh": "最后一步——你准备投入多少总预算？（美元）",
        "guidance": "Think of it as: how much total are you OK putting into this bot? We'll split it into trades automatically. $100-$500 is a solid starting range.",
        "guidance_zh": "想想你愿意投入的总金额，我们会自动分配每笔交易。$100-$500 是新手的好起点。",
        "type": "number",
        "min": 50,
        "max": 100000,
        "freeform": True,
    },
]


def _display_width(s: str) -> int:
    """Display width of string in a monospace terminal (emoji/CJK = 2 cols)."""
    return sum(
        2 if unicodedata.east_asian_width(c) in ("F", "W") else 1
        for c in s
    )


def _rpad(s: str, target: int) -> str:
    """Right-pad *s* with spaces to reach *target* display columns."""
    return s + " " * max(0, target - _display_width(s))


def render_options(question: dict, lang: str = "en", mode: str | None = None) -> str:
    """Render a question's options.

    *lang* — ``"en"`` or ``"zh"``.
    *mode* — ``"terminal"`` (bordered box) or ``"chat"`` (plain bullet list).
             Defaults to the auto-detected ``COACH_ENV``.

    Terminal → bordered box with box-drawing chars (┌─┐│└─┘)
    Chat     → plain emoji bullet list, safe for WeChat / Telegram / any AI model
    """
    _mode = mode or COACH_ENV
    zh = lang.startswith("zh")
    q_text = question.get("question_zh" if zh else "question", question["question"])
    options = question.get("options", [])
    freeform_hint = "或者用你自己的话告诉我" if zh else "or just tell me in your own words"

    if not options:
        guidance = question.get("guidance_zh" if zh else "guidance", question.get("guidance", ""))
        freeform_hint_free = "直接输入你的答案" if zh else "just type your answer"
        result = f"\n**{q_text}**\n"
        if guidance:
            result += f"\n*{guidance}*"
        result += f"\n\n*({freeform_hint_free})*"
        return result

    # ── Chat mode: plain emoji bullet list ──────────────────────────
    if _mode == "chat":
        result = f"\n**{q_text}**\n\n"
        for opt in options:
            if isinstance(opt, dict):
                icon  = opt.get("icon", "")
                label = opt.get("label_zh" if zh else "label", opt["label"])
                tag   = opt.get("tag_zh" if zh else "tag", opt.get("tag", ""))
                if icon and tag:
                    result += f"{icon} {label} — {tag}\n"
                elif icon:
                    result += f"{icon} {label}\n"
                else:
                    result += f"• {label}\n"
            else:
                result += f"• {opt}\n"
        result += f"\n*({freeform_hint})*"
        return result

    # ── Terminal mode: bordered box ──────────────────────────────────
    lines: list[str] = []
    for opt in options:
        if isinstance(opt, dict):
            icon = opt.get("icon", "")
            label = opt.get("label_zh" if zh else "label", opt["label"])
            tag = opt.get("tag_zh" if zh else "tag", opt.get("tag", ""))
            if icon and tag:
                line = f"  {icon} {label} \u2014 {tag}"
            elif icon:
                line = f"  {icon} {label}"
            elif tag:
                line = f"  {label} \u2014 {tag}"
            else:
                line = f"  {label}"
        else:
            line = f"  {opt}"
        lines.append(line)

    inner_w = max(_display_width(ln) for ln in lines) + 2
    top = "\u250c" + "\u2500" * inner_w + "\u2510"
    bot = "\u2514" + "\u2500" * inner_w + "\u2518"

    result = f"\n**{q_text}**\n\n{top}\n"
    for line in lines:
        result += f"\u2502{_rpad(line, inner_w)}\u2502\n"
    result += f"{bot}\n"
    result += f"\n*({freeform_hint})*"
    return result


# ── Strategy Card Renderer ────────────────────────────────────────

def _bold(text: str) -> str:
    """Prefix text with ▸ to visually distinguish section titles."""
    return f"▸ {text}"


def render_strategy_card(
    spec: dict[str, Any],
    theme: str = "",
    tagline: str = "",
) -> str:
    """
    Render a pretty strategy summary card using box-drawing characters.
    Uses _display_width() + _rpad() so the box never cracks.
    """
    meta     = spec.get("meta", {})
    entry    = spec.get("entry", {})
    exit_    = spec.get("exit", {})
    sizing   = spec.get("sizing", {})
    filters  = spec.get("filters", [])
    overlays = spec.get("risk_overlays", [])

    theme   = theme   or meta.get("theme", meta.get("name", "Strategy"))
    tagline = tagline or meta.get("tagline", meta.get("description", ""))
    risk_tier = meta.get("risk_tier", "moderate")

    tier_icon = {"conservative": "🟢", "moderate": "🟡", "aggressive": "🔥"}.get(risk_tier, "🟡")

    # ── Build content rows (plain strings, measured precisely) ────
    rows: list[str] = []

    def divider() -> None:
        rows.append("__DIVIDER__")

    def row(text: str = "") -> None:
        rows.append(f"  {text}")

    # Header
    rows.append(f"  {theme}")
    if tagline:
        # Word-wrap tagline at ~52 chars display width
        words = tagline.split()
        line = ""
        for w in words:
            candidate = f"{line} {w}".strip()
            if _display_width(candidate) > 52:
                rows.append(f"  {line}")
                line = w
            else:
                line = candidate
        if line:
            rows.append(f"  {line}")

    divider()

    # Entry
    entry_type = entry.get("type", "unknown")
    entry_labels = {
        "wallet_copy_buy":  "Wallet Copy Buy",
        "smart_money_buy":  "Smart Money Buy",
        "ranking_entry":    "Trending Snipe",
        "price_drop":       "Dip Buy",
        "ma_cross":         "MA Crossover",
        "rsi_threshold":    "RSI Threshold",
        "time_schedule":    "DCA Schedule",
        "bollinger_touch":  "Bollinger Touch",
        "volume_spike":     "Volume Spike",
        "macd_cross":       "MACD Cross",
        "price_breakout":   "Price Breakout",
        "dev_buy":          "Dev Buy Signal",
    }
    row(f"{_bold('ENTRY')}   {entry_labels.get(entry_type, entry_type)}")

    mirror_mode = entry.get("mirror_mode", "")
    if mirror_mode:
        row(f"        Mode: {mirror_mode}")
    if entry.get("min_wallets"):
        row(f"        Min wallets: {entry['min_wallets']}  Window: {entry.get('window_min', '?')}min")
    if entry.get("list_name"):
        row(f"        List: {entry['list_name']}  Top {entry.get('top_n', '?')}")
    if entry.get("pct"):
        row(f"        Trigger: -{entry['pct']}% drop  ({entry.get('lookback_bars', '?')} bars)")
    wallets = entry.get("target_wallet", [])
    for w in wallets:
        short = f"{w[:6]}...{w[-4:]}" if len(w) > 12 else w
        row(f"        Wallet: {short}")

    divider()

    # Exit
    sl = exit_.get("stop_loss", {}).get("pct", "?")
    row(f"{_bold('EXIT')}    Stop Loss       -{sl}%")
    for ex in exit_.get("other", []):
        t = ex.get("type", "")
        if t == "tiered_take_profit":
            tiers = ex.get("tiers", [])
            row(f"        Tiered TP       {len(tiers)} levels")
            for tier in tiers:
                row(f"          +{tier['pct_gain']}% → sell {tier['pct_sell']}%")
        elif t == "trailing_stop":
            row(f"        Trailing Stop   -{ex.get('pct', '?')}%")
        elif t == "take_profit":
            row(f"        Take Profit     +{ex.get('pct', '?')}%")
        elif t == "wallet_mirror_sell":
            row(f"        Mirror Sell     when wallet exits")
        elif t == "smart_money_sell":
            row(f"        Smart Exit      when whales sell")
        elif t == "dev_dump":
            row(f"        Dev Dump        instant exit")
        elif t == "fast_dump_exit":
            row(f"        Rug Guard       -{ex.get('drop_pct','?')}% in {ex.get('window_sec','?')}s")
        elif t == "time_exit":
            row(f"        Time Exit       after {ex.get('max_bars','?')} bars")

    divider()

    # Sizing
    if sizing.get("type") == "fixed_usd":
        row(f"{_bold('SIZING')}  ${sizing['usd']} per trade")
    elif sizing.get("type") == "fixed_pct":
        row(f"{_bold('SIZING')}  {sizing['pct']}% of portfolio per trade")
    elif sizing.get("type") == "volatility_scaled":
        row(f"{_bold('SIZING')}  Volatility scaled  (target {sizing.get('target_risk_pct','?')}%)")

    divider()

    # Risk overlays
    row(_bold("GUARDS"))
    for ov in overlays:
        t = ov.get("type", "")
        if t == "max_daily_trades":
            row(f"        Max trades/day  {ov['n']}")
        elif t == "max_concurrent_positions":
            row(f"        Max concurrent  {ov['n']}")
        elif t == "drawdown_pause":
            row(f"        Drawdown pause  -{ov['pause_pct']}%")
        elif t == "session_loss_pause":
            row(f"        Loss streak     pause after {ov['max_consecutive_losses']}")
        elif t == "correlation_cap":
            row(f"        Correlation cap {ov.get('max_correlated','?')}")

    divider()

    # Filters
    filter_types = [f.get("type", "") for f in filters]
    row(f"{_bold('FILTERS')}  {len(filters)} safety checks active")
    for ft in filter_types:
        row(f"  ✓  {ft.replace('_', ' ')}")

    divider()

    # Risk tier
    row(f"{_bold('TIER')}    {tier_icon} {risk_tier.capitalize()}")

    # ── Compute box width from widest row ─────────────────────────
    content_rows = [r for r in rows if r != "__DIVIDER__"]
    inner_w = max(_display_width(r) for r in content_rows) + 2

    top = "╔" + "═" * inner_w + "╗"
    div = "╠" + "═" * inner_w + "╣"
    bot = "╚" + "═" * inner_w + "╝"

    lines = [top]
    for r in rows:
        if r == "__DIVIDER__":
            lines.append(div)
        else:
            lines.append("║" + _rpad(r, inner_w) + "║")
    lines.append(bot)

    return "\n```\n" + "\n".join(lines) + "\n```"


def render_strategy_card_md(
    spec: dict[str, Any],
    theme: str = "",
    tagline: str = "",
) -> str:
    """
    Markdown-native strategy card — works on Telegram, web UIs, all agents.
    No box drawing characters, pure markdown formatting.
    """
    meta     = spec.get("meta", {})
    entry    = spec.get("entry", {})
    exit_    = spec.get("exit", {})
    sizing   = spec.get("sizing", {})
    filters  = spec.get("filters", [])
    overlays = spec.get("risk_overlays", [])

    theme   = theme   or meta.get("theme", meta.get("name", "Strategy"))
    tagline = tagline or meta.get("tagline", meta.get("description", ""))
    risk_tier = meta.get("risk_tier", "moderate")
    tier_icon = {"conservative": "🟢", "moderate": "🟡", "aggressive": "🔥"}.get(risk_tier, "🟡")

    entry_labels = {
        "wallet_copy_buy": "Wallet Copy Buy",
        "smart_money_buy": "Smart Money Buy",
        "ranking_entry":   "Trending Snipe",
        "price_drop":      "Dip Buy",
        "ma_cross":        "MA Crossover",
        "rsi_threshold":   "RSI Threshold",
        "time_schedule":   "DCA Schedule",
        "bollinger_touch": "Bollinger Touch",
        "volume_spike":    "Volume Spike",
        "macd_cross":      "MACD Cross",
        "price_breakout":  "Price Breakout",
        "dev_buy":         "Dev Buy Signal",
    }

    lines = []
    lines.append(f"## 🎯 {theme}")
    if tagline:
        lines.append(f"*{tagline}*")
    lines.append("")

    # Entry
    entry_type = entry.get("type", "unknown")
    lines.append(f"**▸ ENTRY** — {entry_labels.get(entry_type, entry_type)}")
    if entry.get("min_wallets"):
        lines.append(f"  Min wallets: {entry['min_wallets']}  ·  Window: {entry.get('window_min', '?')}min")
    if entry.get("list_name"):
        lines.append(f"  List: {entry['list_name']}  ·  Top {entry.get('top_n', '?')}")
    if entry.get("pct"):
        lines.append(f"  Trigger: -{entry['pct']}% drop  ({entry.get('lookback_bars', '?')} bars)")
    wallets = entry.get("target_wallet", [])
    for w in wallets:
        short = f"{w[:6]}...{w[-4:]}" if len(w) > 12 else w
        lines.append(f"  Wallet: `{short}`")
    lines.append("")

    # Exit
    sl = exit_.get("stop_loss", {}).get("pct", "?")
    lines.append(f"**▸ EXIT**")
    lines.append(f"  Stop Loss: **-{sl}%**")
    for ex in exit_.get("other", []):
        t = ex.get("type", "")
        if t == "tiered_take_profit":
            tiers = ex.get("tiers", [])
            lines.append(f"  Tiered TP ({len(tiers)} levels):")
            for tier in tiers:
                lines.append(f"    +{tier['pct_gain']}% → sell {tier['pct_sell']}%")
        elif t == "trailing_stop":
            lines.append(f"  Trailing Stop: -{ex.get('pct', '?')}%")
        elif t == "take_profit":
            lines.append(f"  Take Profit: +{ex.get('pct', '?')}%")
        elif t == "wallet_mirror_sell":
            lines.append(f"  Mirror Sell: when wallet exits")
        elif t == "smart_money_sell":
            lines.append(f"  Smart Exit: when whales sell")
        elif t == "dev_dump":
            lines.append(f"  Dev Dump: instant exit")
        elif t == "fast_dump_exit":
            lines.append(f"  Rug Guard: -{ex.get('drop_pct','?')}% in {ex.get('window_sec','?')}s")
        elif t == "time_exit":
            lines.append(f"  Time Exit: after {ex.get('max_bars','?')} bars")
    lines.append("")

    # Sizing
    if sizing.get("type") == "fixed_usd":
        lines.append(f"**▸ SIZING** — ${sizing['usd']} per trade")
    elif sizing.get("type") == "fixed_pct":
        lines.append(f"**▸ SIZING** — {sizing['pct']}% of portfolio per trade")
    lines.append("")

    # Guards
    if overlays:
        guard_parts = []
        for ov in overlays:
            t = ov.get("type", "")
            if t == "max_daily_trades":
                guard_parts.append(f"max {ov['n']}/day")
            elif t == "max_concurrent_positions":
                guard_parts.append(f"max {ov['n']} concurrent")
            elif t == "drawdown_pause":
                guard_parts.append(f"pause at -{ov['pause_pct']}% DD")
            elif t == "session_loss_pause":
                guard_parts.append(f"pause after {ov['max_consecutive_losses']} losses")
        lines.append(f"**▸ GUARDS** — {' · '.join(guard_parts)}")
        lines.append("")

    # Filters
    if filters:
        filter_names = [f.get("type", "").replace("_", " ") for f in filters]
        lines.append(f"**▸ FILTERS** — {len(filters)} safety checks")
        lines.append("  " + "  ·  ".join(f"✓ {n}" for n in filter_names))
        lines.append("")

    # Tier
    lines.append(f"**▸ TIER** — {tier_icon} {risk_tier.capitalize()}")

    return "\n".join(lines)


# ── Risk Presets ──────────────────────────────────────────────────

RISK_PRESETS: dict[str, dict[str, Any]] = {
    "conservative": {
        "risk_tier": "conservative",
        "sl_pct": 8,
        "tp_pct": 12,
        "sizing_pct": 2,
        "sizing_usd": 50,
        "max_daily_trades": 3,
        "max_dd_pause": 8,
        "max_concurrent": 3,
        "session_loss_pause": 2,
    },
    "moderate": {
        "risk_tier": "moderate",
        "sl_pct": 15,
        "tp_pct": 20,
        "sizing_pct": 5,
        "sizing_usd": 100,
        "max_daily_trades": 5,
        "max_dd_pause": 12,
        "max_concurrent": 5,
        "session_loss_pause": 3,
    },
    "aggressive": {
        "risk_tier": "aggressive",
        "sl_pct": 20,
        "tp_pct": 30,
        "sizing_pct": 10,
        "sizing_usd": 200,
        "max_daily_trades": 10,
        "max_dd_pause": 15,
        "max_concurrent": 8,
        "session_loss_pause": 5,
    },
}


# ── Goal-to-Template Mapping ─────────────────────────────────────

GOAL_TEMPLATES: dict[str, dict[str, Any]] = {
    "dca": {
        "name": "Weekly DCA",
        "entry_type": "time_schedule",
        "entry_params": {"interval": "1W", "anchor_utc": "09:00"},
        "exit_style": "trailing",
        "needs_safety_stack": False,
        "live_only": False,
        "description": "Buy on a fixed schedule with trailing stop protection.",
    },
    "dip_buy": {
        "name": "Dip Buyer",
        "entry_type": "price_drop",
        "entry_params": {"pct": 5, "lookback_bars": 12},
        "exit_style": "sl_tp",
        "needs_safety_stack": False,
        "live_only": False,
        "description": "Buy when price drops X% in a set window.",
    },
    "trend_follow": {
        "name": "Trend Follower",
        "entry_type": "ma_cross",
        "entry_params": {"fast_period": 10, "slow_period": 50, "ma_type": "EMA"},
        "exit_style": "trailing",
        "needs_safety_stack": False,
        "live_only": False,
        "description": "Buy when fast moving average crosses above slow.",
    },
    "mean_revert": {
        "name": "Mean Reverter",
        "entry_type": "rsi_threshold",
        "entry_params": {"period": 14, "level": 30, "direction": "cross_up"},
        "exit_style": "sl_tp",
        "needs_safety_stack": False,
        "live_only": False,
        "description": "Buy when RSI signals oversold, sell when overbought.",
    },
    "smart_money": {
        "name": "Smart Money Follower",
        "entry_type": "smart_money_buy",
        "entry_params": {"min_wallets": 2, "window_min": 60},
        "exit_style": "sm_exit",
        "needs_safety_stack": True,
        "live_only": True,
        "description": "Buy when smart money wallets buy a token.",
    },
    "copy_trade": {
        "name": "Wallet Copy-Trader",
        "entry_type": "wallet_copy_buy",
        "entry_params": {"min_usd": 100, "mirror_mode": "instant"},
        "exit_style": "mirror_exit",
        "needs_safety_stack": True,
        "live_only": True,
        "description": "Mirror buy/sell actions of specific wallets.",
    },
    "meme_sniper": {
        "name": "Meme Sniper",
        "entry_type": "ranking_entry",
        "entry_params": {"list_name": "trending", "top_n": 20},
        "exit_style": "tiered_tp",
        "needs_safety_stack": True,
        "live_only": True,
        "description": "Snipe trending/new tokens with full safety checks.",
    },
    "grid": {
        "name": "Grid Trader",
        "entry_type": "grid",
        "entry_params": {},
        "exit_style": "grid",
        "needs_safety_stack": False,
        "live_only": False,
        "description": "Buy low and sell high within a price range.",
    },
}


# ── Coach State ───────────────────────────────────────────────────

@dataclass
class CoachState:
    """Persisted coaching state for a user session."""
    session_id: str = ""
    step: int = 1              # Current step (1-6)
    profile: dict[str, Any] = field(default_factory=dict)
    selected_goal: str = ""
    selected_template: str = ""
    custom_params: dict[str, Any] = field(default_factory=dict)
    strategy_spec: dict[str, Any] = field(default_factory=dict)
    backtest_history: list[dict[str, Any]] = field(default_factory=list)
    improvements: list[str] = field(default_factory=list)
    live_deployed: bool = False
    live_confirmed: bool = False
    live_confirmed_ts: float = 0.0
    evolve_enabled: bool = False
    run_mode: str = ""         # "chat" or "python" — set after strategy card shown
    created_ts: float = 0.0
    updated_ts: float = 0.0


def _state_path(session_id: str) -> Path:
    STATE_DIR.mkdir(parents=True, exist_ok=True)
    return STATE_DIR / f"{session_id}.json"


def load_state(session_id: str) -> CoachState:
    path = _state_path(session_id)
    if not path.exists():
        return CoachState(
            session_id=session_id,
            created_ts=time.time(),
            updated_ts=time.time(),
        )
    with open(path) as f:
        data = json.load(f)
    return CoachState(**data)


def save_state(state: CoachState) -> None:
    state.updated_ts = time.time()
    path = _state_path(state.session_id)
    with open(path, "w") as f:
        json.dump(asdict(state), f, indent=2)


# ── Step Handlers ─────────────────────────────────────────────────

def get_current_step_info(state: CoachState) -> dict[str, Any]:
    """Return info about the current step for the LLM to use in conversation."""
    step = state.step
    info: dict[str, Any] = {"step": step, "step_name": ""}

    if step == 1:
        info["step_name"] = "welcome"
        info["action"] = "welcome"
        info["message"] = WELCOME_MESSAGE
        info["disclaimer"] = DISCLAIMER_MESSAGE
        info["next_action"] = "vibe_check"

    elif step == 2:
        info["step_name"] = "vibe_check"
        info["action"] = "ask_questions"
        # Determine which questions still need answers
        answered = set(state.profile.keys())
        remaining = []
        for q in PROFILE_QUESTIONS:
            if q["key"] not in answered:
                # Check conditional
                cond = q.get("conditional", "")
                if cond:
                    goal = state.profile.get("goal", "")
                    if "copy" not in goal.lower() and "smart money" not in goal.lower():
                        continue  # Skip conditional questions
                remaining.append(q)
        info["questions_remaining"] = remaining
        info["profile_so_far"] = state.profile
        info["all_answered"] = len(remaining) == 0

    elif step == 3:
        info["step_name"] = "build_strategy"
        info["action"] = "build_spec"
        info["profile"] = state.profile
        # Suggest templates based on goal
        goal = state.selected_goal or _infer_goal(state.profile.get("goal", ""))
        templates = _suggest_templates(goal)
        info["suggested_templates"] = templates
        info["selected_template"] = state.selected_template
        info["spec"] = state.strategy_spec
        # After spec is built, ask how the user wants to run it
        if state.strategy_spec and not state.run_mode:
            info["needs_run_mode"] = True
            info["run_mode_question"] = {
                "question": "How do you want to run this strategy?",
                "question_zh": "你想怎么运行这个策略？",
                "options": [
                    {
                        "icon": "💬",
                        "label": "Trade in chat",
                        "label_zh": "在对话里交易",
                        "tag": "I'll guide every move — just talk to me",
                        "tag_zh": "我来引导每一步，直接聊就行",
                    },
                    {
                        "icon": "🖥️",
                        "label": "Python bot + dashboard",
                        "label_zh": "生成 Python 机器人",
                        "tag": "Generate a script I can run 24/7",
                        "tag_zh": "生成一个可以24小时运行的脚本",
                    },
                ],
                "display": "box",
            }
        info["run_mode"] = state.run_mode

    elif step == 4:
        info["step_name"] = "test_drive"
        info["action"] = "test"
        info["spec"] = state.strategy_spec
        info["run_mode"] = state.run_mode  # "chat" or "python"
        if state.run_mode == "python":
            info["action"] = "test_python"
            info["instruction"] = (
                "The user chose Python bot mode. Call generate_bot_script() and show them "
                "the filename + how to run it. Tell them to set PAPER_TRADE = True (default) "
                "to test first. Show the quick-start commands."
            )
        else:
            info["action"] = "test_chat"
            info["instruction"] = (
                "The user chose chat mode. Walk them through a simulated paper trade "
                "step-by-step using onchainos MCP tools. "
                "Use workflow commands where possible: "
                "workflow_smart_money() for SM strategies, "
                "workflow_new_tokens() for meme/sniper strategies, "
                "workflow_token_research() for single-token strategies. "
                "Show each signal check, the hypothetical entry, and how exits would work."
            )
        info["backtest_disclaimer"] = (
            "⚠️ *Backtest results are based on historical simulation only. "
            "Past performance does not guarantee future results.*"
        )
        if state.strategy_spec:
            _, _, meta = validate_spec(state.strategy_spec)
            info["live_only"] = meta.get("live_only", False)
            if meta.get("live_only"):
                strategy_name = state.strategy_spec.get("meta", {}).get("name", state.session_id)
                graduated, progress = check_graduation(strategy_name)
                info["graduated"] = graduated
                info["progress"] = progress
            info["backtest_history"] = state.backtest_history

    elif step == 5:
        info["step_name"] = "go_live"
        info["action"] = "deploy"
        info["spec"] = state.strategy_spec
        info["run_mode"] = state.run_mode
        info["live_deployed"] = state.live_deployed
        info["live_confirmed"] = state.live_confirmed
        # Bot summary — human-readable plain English description
        if state.strategy_spec:
            info["bot_summary"] = render_bot_summary(state.strategy_spec, state.profile)
        # Live mode gate — must confirm before switching from paper to live
        if not state.live_confirmed:
            info["needs_live_confirmation"] = True
            info["live_disclaimer"] = LIVE_MODE_DISCLAIMER
            info["instruction"] = (
                "IMPORTANT: Before going live, show the user the bot_summary so they know "
                "exactly what the bot will do. Then show the live_disclaimer and ask them "
                "to type CONFIRM. Only call confirm_live_mode() once they type exactly CONFIRM. "
                "Do NOT proceed to live mode without this confirmation."
            )
        elif state.run_mode == "python":
            info["instruction"] = (
                "Live mode confirmed. Python mode: tell user to set PAPER_TRADE = False "
                "in their bot script and restart. Confirm onchainos wallet is logged in. "
                "Then monitor together."
            )
        else:
            info["instruction"] = (
                "Live mode confirmed. Chat mode: check onchainos wallet status, "
                "then execute the first real trade together using onchainos swap."
            )

    elif step == 6:
        info["step_name"] = "evolve"
        info["action"] = "evolve"
        info["evolve_enabled"] = state.evolve_enabled
        info["spec"] = state.strategy_spec

    return info


def advance_step(state: CoachState) -> CoachState:
    """Move to the next step if current step is complete."""
    if state.step < 6:
        state.step += 1
        save_state(state)
    return state


def set_profile_answer(state: CoachState, key: str, value: Any) -> CoachState:
    """Record a profile answer."""
    state.profile[key] = value
    save_state(state)
    return state


def select_template(state: CoachState, goal_key: str) -> CoachState:
    """Select a strategy template based on goal."""
    state.selected_goal = goal_key
    state.selected_template = goal_key
    save_state(state)
    return state


def set_run_mode(state: CoachState, mode: str) -> CoachState:
    """Set how the user wants to run their strategy: 'chat' or 'python'."""
    state.run_mode = mode if mode in ("chat", "python") else "chat"
    save_state(state)
    return state


def generate_bot_script(state: CoachState) -> tuple[str, str]:
    """
    Generate a fully runnable Python bot script from the strategy spec.

    Returns (filename, script_content).
    The script imports OnchainOS, has real signal detection per entry type,
    real safety filter checking, real position monitoring, and real swap execution.
    Paper mode by default. After writing, always run verify_bot_script() to check syntax.
    """
    import subprocess as _sp

    spec    = state.strategy_spec
    meta    = spec.get("meta", {})
    entry   = spec.get("entry", {})
    exit_   = spec.get("exit", {})
    sizing  = spec.get("sizing", {})
    inst    = spec.get("instrument", {})
    filters = spec.get("filters", [])
    overlays = spec.get("risk_overlays", [])

    name       = meta.get("name", "my_strategy")
    theme      = meta.get("theme", "My Strategy")
    tagline    = meta.get("tagline", "")
    risk_tier  = meta.get("risk_tier", "moderate")
    entry_type = entry.get("type", "price_drop")
    symbol     = inst.get("symbol", "*")
    timeframe  = inst.get("timeframe", "1H")
    chain      = spec.get("universe", {}).get("chain", "solana")
    usd_size   = sizing.get("usd", 100) if sizing.get("type") == "fixed_usd" else 100
    sl_pct     = exit_.get("stop_loss", {}).get("pct", 10)
    total_budget = state.profile.get("total_budget") or (usd_size * max_daily * 2)

    other_exits = exit_.get("other", [])
    # take_profit / trailing_stop / tiered_take_profit are direct exit keys, NOT in other[]
    tp_pct    = exit_.get("take_profit",          {}).get("pct", 20)
    trail_pct = exit_.get("trailing_stop",         {}).get("pct", None)
    tiered_tp = exit_.get("tiered_take_profit",    None)
    # dev_dump and fast_dump_exit live in other[]
    dev_dump  = any(e.get("type") == "dev_dump"        for e in other_exits)
    fast_dump = next((e for e in other_exits if e.get("type") == "fast_dump_exit"), None)

    max_daily   = next((o.get("n", 5)  for o in overlays if o.get("type") == "max_daily_trades"), 5)
    max_conc    = next((o.get("n", 3)  for o in overlays if o.get("type") == "max_concurrent_positions"), 3)
    sess_pause  = next((o.get("max_consecutive_losses", 3) for o in overlays if o.get("type") == "session_loss_pause"), 3)

    # Build SAFETY dict from filters
    safety_dict: dict[str, Any] = {}
    for f in filters:
        ft = f.get("type", "")
        if ft == "honeypot_check":         safety_dict["honeypot_check"] = True
        elif ft == "phishing_exclude":     safety_dict["phishing_exclude"] = True
        elif ft == "lp_locked":            safety_dict["lp_locked_min_pct"] = f.get("min_pct_locked", 50)
        elif ft == "buy_tax_max":          safety_dict["buy_tax_max"] = f.get("max_pct", 5)
        elif ft == "sell_tax_max":         safety_dict["sell_tax_max"] = f.get("max_pct", 5)
        elif ft == "liquidity_min":        safety_dict["liquidity_min_usd"] = f.get("min_usd", 5000)
        elif ft == "top_holders_max":      safety_dict["top_holders_max_pct"] = f.get("max_pct", 40)
        elif ft == "bundler_ratio_max":    safety_dict["bundler_max_pct"] = f.get("max_pct", 20)
        elif ft == "dev_holding_max":      safety_dict["dev_holding_max_pct"] = f.get("max_pct", 10)
        elif ft == "insider_holding_max":  safety_dict["insider_max_pct"] = f.get("max_pct", 20)
        elif ft == "whale_concentration_max": safety_dict["whale_max_pct"] = f.get("max_pct", 20)
        elif ft == "fresh_wallet_ratio_max":  safety_dict["fresh_wallet_max_pct"] = f.get("max_pct", 40)
        elif ft == "smart_money_present_min": safety_dict["sm_present_min"] = f.get("min_wallets", 1)
        elif ft == "mcap_range":
            if "min_usd" in f: safety_dict["mcap_min_usd"] = f["min_usd"]
            if "max_usd" in f: safety_dict["mcap_max_usd"] = f["max_usd"]

    entry_params = {k: v for k, v in entry.items() if k != "type"}
    tiered_tiers = json.dumps(tiered_tp.get("tiers", []) if tiered_tp else [])
    fast_drop    = fast_dump.get("drop_pct", 20) if fast_dump else None
    fast_window  = fast_dump.get("window_sec", 60) if fast_dump else None

    # Signal logic block — different per entry type
    if entry_type == "ranking_entry":
        signal_block = f"""\
def find_candidates() -> list[dict]:
    list_name = ENTRY_PARAMS.get("list_name", "new")
    # Primary: workflow_new_tokens for new/bonding/migrated — returns safety-enriched top 10
    if list_name in ("new", "bonding", "migrated", "migrating"):
        stage = "MIGRATING" if list_name in ("new", "bonding", "migrating") else "MIGRATED"
        result = oc.workflow_new_tokens(stage=stage)
        tokens = []
        if isinstance(result, dict):
            inner = result.get("data") or result
            tokens = inner.get("tokens") or inner.get("items") or []
        if tokens and isinstance(tokens, list):
            return [{{"token": t.get("tokenAddress") or t.get("address", ""),
                      "symbol": t.get("symbol") or t.get("tokenSymbol", ""),
                      "signal_data": {{"rank": i, "dd": t}}}}
                    for i, t in enumerate(tokens)
                    if t.get("tokenAddress") or t.get("address")]
    # Fallback: raw ranking subscription
    items = oc.subscribe_ranking(list_name, ENTRY_PARAMS.get("top_n", 50))
    return [{{"token": i.token, "symbol": i.symbol, "signal_data": {{"rank": i.rank}}}}
            for i in items if i.token]"""

    elif entry_type == "smart_money_buy":
        signal_block = f"""\
def find_candidates() -> list[dict]:
    min_w = ENTRY_PARAMS.get("min_wallets", 1)
    # Primary: workflow_smart_money aggregates signals + per-token DD in one call
    result = oc.workflow_smart_money()
    tokens = []
    if isinstance(result, dict):
        inner = result.get("data") or result
        tokens = inner.get("tokens") or inner.get("items") or inner.get("signals") or []
    if tokens and isinstance(tokens, list):
        candidates = []
        for tok in tokens:
            addr   = tok.get("tokenAddress") or tok.get("address") or tok.get("token", "")
            symbol = tok.get("symbol") or tok.get("tokenSymbol", "")
            count  = tok.get("walletCount") or tok.get("signalCount") or tok.get("count") or 1
            if addr and count >= min_w:
                candidates.append({{"token": addr, "symbol": symbol,
                                    "signal_data": {{"wallet_count": count, "dd": tok}}}})
        if candidates:
            return candidates
    # Fallback: raw signal list with manual grouping
    signals = oc.get_signals(wallet_type=1, min_amount_usd=ENTRY_PARAMS.get("min_usd_each"))
    seen: dict[str, int] = {{}}
    for s in signals:
        t = s.get("tokenAddress", "")
        if t: seen[t] = seen.get(t, 0) + 1
    return [{{"token": t, "symbol": "", "signal_data": {{"wallet_count": n}}}}
            for t, n in seen.items() if n >= min_w]"""

    elif entry_type == "wallet_copy_buy":
        signal_block = f"""\
def find_candidates() -> list[dict]:
    # Pre-trade research: workflow_wallet_analysis shows 7d/30d performance of copied wallets
    # (run once at startup for context; not repeated every poll cycle)
    wallets = ENTRY_PARAMS.get("target_wallet", [])
    if isinstance(wallets, str): wallets = [wallets]
    # Real-time signal: track live buy events from the target wallets
    events = oc.track_wallets(wallets, trade_type=1)
    seen: dict[str, float] = {{}}
    for e in events:
        if e.token and e.side == "buy" and e.usd_amount >= ENTRY_PARAMS.get("min_usd", 10):
            seen[e.token] = e.usd_amount
    return [{{"token": t, "symbol": "", "signal_data": {{"copy_usd": usd}}}}
            for t, usd in seen.items()]"""

    elif entry_type == "price_drop":
        signal_block = f"""\
def find_candidates() -> list[dict]:
    candles = oc.get_candles(TOKEN_ADDRESS, TIMEFRAME, ENTRY_PARAMS.get("lookback_bars", 12))
    if len(candles) < 2: return []
    high = max(c["high"] for c in candles)
    current = candles[-1]["close"]
    if high <= 0: return []
    drop = (high - current) / high * 100  # how far price has fallen from lookback high
    if drop >= ENTRY_PARAMS.get("pct", 5):
        return [{{"token": TOKEN_ADDRESS, "symbol": SYMBOL, "signal_data": {{"drop_pct": round(drop, 2)}}}}]
    return []"""

    elif entry_type == "time_schedule":
        signal_block = f"""\
def find_candidates() -> list[dict]:
    now = datetime.now(timezone.utc)
    interval = ENTRY_PARAMS.get("interval", "1D")
    anchor_h, anchor_m = map(int, ENTRY_PARAMS.get("anchor_utc", "09:00").split(":"))
    if not (now.hour == anchor_h and abs(now.minute - anchor_m) <= 2): return []
    if interval == "1W" and now.weekday() != 0: return []
    return [{{"token": TOKEN_ADDRESS, "symbol": SYMBOL, "signal_data": {{"scheduled": True}}}}]"""

    elif entry_type == "rsi_threshold":
        signal_block = f"""\
def find_candidates() -> list[dict]:
    candles = oc.get_candles(TOKEN_ADDRESS, TIMEFRAME, 50)
    if len(candles) < 16: return []
    closes = [c["close"] for c in candles]
    rsi      = _rsi_last(closes,    ENTRY_PARAMS.get("period", 14))
    prev_rsi = _rsi_last(closes[:-1], ENTRY_PARAMS.get("period", 14))
    if rsi is None or prev_rsi is None: return []
    level = ENTRY_PARAMS.get("level", 30)
    direction = ENTRY_PARAMS.get("direction", "cross_up")
    if direction == "cross_up"   and prev_rsi < level <= rsi:
        return [{{"token": TOKEN_ADDRESS, "symbol": SYMBOL, "signal_data": {{"rsi": round(rsi, 2)}}}}]
    if direction == "cross_down" and prev_rsi > level >= rsi:
        return [{{"token": TOKEN_ADDRESS, "symbol": SYMBOL, "signal_data": {{"rsi": round(rsi, 2)}}}}]
    return []"""

    elif entry_type == "ma_cross":
        signal_block = f"""\
def find_candidates() -> list[dict]:
    candles = oc.get_candles(TOKEN_ADDRESS, TIMEFRAME, 100)
    if len(candles) < ENTRY_PARAMS.get("slow_period", 21) + 2: return []
    closes = [c["close"] for c in candles]
    fast = [x for x in _ema(closes, ENTRY_PARAMS.get("fast_period", 9)) if x is not None]
    slow = [x for x in _ema(closes, ENTRY_PARAMS.get("slow_period", 21)) if x is not None]
    if len(fast) < 2 or len(slow) < 2: return []
    if fast[-2] < slow[-2] and fast[-1] > slow[-1]:  # bullish cross
        return [{{"token": TOKEN_ADDRESS, "symbol": SYMBOL, "signal_data": {{"cross": "bullish"}}}}]
    return []"""

    elif entry_type == "macd_cross":
        signal_block = f"""\
def find_candidates() -> list[dict]:
    candles = oc.get_candles(TOKEN_ADDRESS, TIMEFRAME, 100)
    if len(candles) < 35: return []
    closes = [c["close"] for c in candles]
    fast_ema = [x for x in _ema(closes, ENTRY_PARAMS.get("fast_period", 12)) if x is not None]
    slow_ema = [x for x in _ema(closes, ENTRY_PARAMS.get("slow_period", 26)) if x is not None]
    n = min(len(fast_ema), len(slow_ema))
    if n < 10: return []
    macd_line = [fast_ema[-n+i] - slow_ema[-n+i] for i in range(n)]
    signal_line = _ema(macd_line, ENTRY_PARAMS.get("signal_period", 9))
    if len(signal_line) < 2 or signal_line[-1] is None or signal_line[-2] is None: return []
    hist_prev = macd_line[-2] - signal_line[-2]
    hist_last = macd_line[-1] - signal_line[-1]
    direction = ENTRY_PARAMS.get("direction", "cross_up")
    if direction == "cross_up"   and hist_prev < 0 <= hist_last:
        return [{{"token": TOKEN_ADDRESS, "symbol": SYMBOL, "signal_data": {{"macd_hist": round(hist_last, 6)}}}}]
    if direction == "cross_down" and hist_prev > 0 >= hist_last:
        return [{{"token": TOKEN_ADDRESS, "symbol": SYMBOL, "signal_data": {{"macd_hist": round(hist_last, 6)}}}}]
    return []"""

    elif entry_type == "bollinger_touch":
        signal_block = f"""\
def find_candidates() -> list[dict]:
    candles = oc.get_candles(TOKEN_ADDRESS, TIMEFRAME, 60)
    period = ENTRY_PARAMS.get("period", 20)
    if len(candles) < period: return []
    closes = [c["close"] for c in candles]
    window = closes[-period:]
    mid = sum(window) / period
    std = (sum((x - mid) ** 2 for x in window) / period) ** 0.5
    std_dev = ENTRY_PARAMS.get("std_dev", 2.0)
    upper, lower = mid + std_dev * std, mid - std_dev * std
    price = closes[-1]
    band = ENTRY_PARAMS.get("band", "lower")
    if band == "lower" and price <= lower:
        return [{{"token": TOKEN_ADDRESS, "symbol": SYMBOL, "signal_data": {{"band": "lower", "price": price, "lower": lower}}}}]
    if band == "upper" and price >= upper:
        return [{{"token": TOKEN_ADDRESS, "symbol": SYMBOL, "signal_data": {{"band": "upper", "price": price, "upper": upper}}}}]
    return []"""

    elif entry_type == "volume_spike":
        signal_block = f"""\
def find_candidates() -> list[dict]:
    candles = oc.get_candles(TOKEN_ADDRESS, TIMEFRAME, ENTRY_PARAMS.get("avg_bars", 24) + 2)
    if len(candles) < 3: return []
    vols = [c["volume"] for c in candles]
    avg_vol = sum(vols[:-1]) / len(vols[:-1])
    if avg_vol > 0 and vols[-1] >= avg_vol * ENTRY_PARAMS.get("multiplier", 2.0):
        return [{{"token": TOKEN_ADDRESS, "symbol": SYMBOL, "signal_data": {{"vol_ratio": round(vols[-1] / avg_vol, 2)}}}}]
    return []"""

    else:  # dev_buy or any other type
        signal_block = f"""\
def find_candidates() -> list[dict]:
    # Entry type: {entry_type}
    signals = oc.get_signals(wallet_type=2)  # wallet_type 2 = dev/deployer
    return [{{"token": s.get("tokenAddress", ""), "symbol": "", "signal_data": s}}
            for s in signals if s.get("tokenAddress")]"""

    # Tiered TP exit block
    if tiered_tp:
        tiered_block = f"""\
        TIERED_TP = {tiered_tiers}
        if TIERED_TP:
            for tier in TIERED_TP:
                if pnl_pct >= tier["pct_gain"] and not pos.get(f"tp_tier_{{tier['pct_gain']}}_hit"):
                    pos[f"tp_tier_{{tier['pct_gain']}}_hit"] = True
                    sell_pct = tier["pct_sell"] / 100
                    print(f"[TP TIER] {{pos['symbol'] or pos['token'][:8]}} +{{pnl_pct:.1f}}% → selling {{tier['pct_sell']}}%")
                    # In paper mode, log it; in live mode, sell a portion
                    if not PAPER_TRADE and pos.get("mode") == "live":
                        bal = oc.get_token_balance(pos["token"])
                        sell_amt = bal * sell_pct
                        if sell_amt > 0:
                            oc.swap_execute(pos["token"], _usdc_address(), str(sell_amt), WALLET_ADDRESS)
            return  # tiered TP manages exit — only stop_loss fully closes"""
    else:
        tiered_block = ""

    dev_dump_block = ""
    if dev_dump:
        dev_dump_block = """\
        # Dev dump check
        dev_info = oc.get_dev_info(pos["token"])
        dev_wallet = dev_info.get("devWallet", "")
        if dev_wallet:
            events = oc.track_wallets([dev_wallet], trade_type=2)
            _dev_dumped = any(e.token == pos["token"] and e.side == "sell" for e in events)
            if _dev_dumped:
                close_position(pos, "dev_dump", current_price)
                continue  # skip to next position"""

    fast_dump_block = ""
    if fast_dump:
        fast_dump_block = f"""\
        # Fast dump / rug guard
        if "price_history" not in pos: pos["price_history"] = []
        pos["price_history"].append((time.time(), current_price))
        pos["price_history"] = [(t, p) for t, p in pos["price_history"] if time.time() - t <= {fast_window}]
        if pos["price_history"]:
            oldest_price = pos["price_history"][0][1]
            if oldest_price > 0:
                fast_drop = (oldest_price - current_price) / oldest_price * 100
                if fast_drop >= {fast_dump.get("drop_pct", 20)}:
                    close_position(pos, "fast_dump_exit", current_price)
                    continue"""

    # Fixed-symbol token address (well-known tokens)
    well_known = {
        "SOL-USDC": "So11111111111111111111111111111111111111112",
        "ETH-USDC": "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs",
        "BTC-USDC": "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E",
        "WBTC-USDC": "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E",
    }
    token_address = well_known.get(symbol, "")
    token_address_line = (
        f'TOKEN_ADDRESS = "{token_address}"  # {symbol}'
        if token_address else
        f'TOKEN_ADDRESS = ""  # TODO: set {symbol} contract address for chain {chain}'
    )
    is_dynamic = symbol == "*"

    # Dashboard port (deterministic per strategy name, range 8200-8299)
    dashboard_port = 8200 + (abs(hash(name)) % 100)

    # Dashboard HTML (built as plain string — no double-brace needed)
    _DASH_HTML = (
        "<!DOCTYPE html><html><head><meta charset='utf-8'>"
        "<title>__THEME__ Dashboard</title>"
        "<style>"
        "*{box-sizing:border-box;margin:0;padding:0}"
        "body{background:#0a0e14;color:#ccd6f6;font-family:'SF Mono','Fira Code',monospace;font-size:13px;padding:16px}"
        "h2{color:#00e676;margin-bottom:10px;font-size:15px}"
        ".panel{background:#12171f;border:1px solid #252d3a;border-radius:8px;padding:14px;margin-bottom:12px}"
        ".row{display:flex;justify-content:space-between;padding:4px 0;border-bottom:1px solid #1a2030}"
        ".row:last-child{border-bottom:none}"
        ".label{color:#8892b0}"
        ".green{color:#00e676}.red{color:#ff5252}.yellow{color:#ffd740}.blue{color:#448aff}"
        ".grid2{display:grid;grid-template-columns:1fr 1fr;gap:12px}"
        "table{width:100%;border-collapse:collapse}"
        "th{color:#448aff;text-align:left;padding:6px 8px;border-bottom:1px solid #252d3a;font-weight:normal}"
        "td{padding:5px 8px;border-bottom:1px solid #1a2030}"
        ".badge{display:inline-block;padding:2px 8px;border-radius:4px;font-size:11px}"
        ".badge-paper{background:#1a2a4a;color:#448aff}"
        ".badge-live{background:#2a1a1a;color:#ff5252}"
        ".feed-item{padding:3px 0;border-bottom:1px solid #1a2030;font-size:12px}"
        ".ts{color:#4a5568;margin-right:8px}"
        "</style></head><body>"
        "<div class='panel'>"
        "<div style='display:flex;justify-content:space-between;align-items:center;margin-bottom:8px'>"
        "<h2>&#x1F40B; __THEME__</h2><span id='mode-badge' class='badge'></span></div>"
        "<div class='grid2'><div>"
        "<div class='row'><span class='label'>Entry</span><span id='entry-type'></span></div>"
        "<div class='row'><span class='label'>Stop Loss</span><span class='red' id='sl'></span></div>"
        "<div class='row'><span class='label'>Take Profit</span><span class='green' id='tp'></span></div>"
        "<div class='row'><span class='label'>Poll</span><span id='poll'></span></div>"
        "</div><div>"
        "<div class='row'><span class='label'>Wallet</span><span class='blue' id='wallet'></span></div>"
        "<div class='row'><span class='label'>Session PnL</span><span id='session-pnl'></span></div>"
        "<div class='row'><span class='label'>Trades</span><span id='trades'></span></div>"
        "<div class='row'><span class='label'>Win Rate</span><span id='winrate'></span></div>"
        "</div></div></div>"
        "<div class='panel'>"
        "<h2>Open Positions (<span id='open-count'>0</span>)</h2>"
        "<table><thead><tr><th>Symbol</th><th>Entry</th><th>Current</th><th>PnL</th><th>Since</th></tr></thead>"
        "<tbody id='pos-body'></tbody></table></div>"
        "<div class='panel'><h2>Signal Feed</h2><div id='feed'></div></div>"
        "<script>"
        "async function refresh(){"
        "try{"
        "const r=await fetch('/api/state');const d=await r.json();"
        "const mb=document.getElementById('mode-badge');"
        "mb.textContent=d.mode==='paper'?'PAPER \U0001F4C4':'LIVE \U0001F534';"
        "mb.className='badge badge-'+(d.mode==='paper'?'paper':'live');"
        "document.getElementById('entry-type').textContent=d.entry_type||'';"
        "document.getElementById('sl').textContent='-'+d.sl_pct+'%';"
        "document.getElementById('tp').textContent='+'+d.tp_pct+'%';"
        "document.getElementById('poll').textContent=d.poll_sec+'s';"
        "document.getElementById('wallet').textContent=d.wallet||'—';"
        "const s=d.stats||{};"
        "const pnl=s.pnl_usd||0;"
        "const pe=document.getElementById('session-pnl');"
        "pe.textContent=(pnl>=0?'+':'')+'$'+pnl.toFixed(2);"
        "pe.className=pnl>=0?'green':'red';"
        "document.getElementById('trades').textContent=(s.daily_trades||0)+'/'+(s.daily_limit||0)+' today';"
        "const wr=(s.wins&&s.total_trades)?Math.round(s.wins/s.total_trades*100):0;"
        "document.getElementById('winrate').textContent=s.total_trades?(wr+'% ('+s.wins+'W/'+s.losses+'L)'):'—';"
        "const open=(d.positions||[]).filter(p=>p.status==='open');"
        "document.getElementById('open-count').textContent=open.length;"
        "document.getElementById('pos-body').innerHTML=open.map(p=>{"
        "const pct=p.pnl_pct||0;const cl=pct>=0?'green':'red';"
        "const since=(p.entry_ts||'').slice(11,19);"
        "return '<tr><td>'+(p.symbol||p.token.slice(0,12))+'</td>"
        "<td>'+(p.entry_price||0).toFixed(6)+'</td>"
        "<td>'+(p.current_price||0).toFixed(6)+'</td>"
        "<td class=\"'+cl+'\">'+(pct>=0?'+':'')+pct.toFixed(1)+'%</td>"
        "<td>'+since+'</td></tr>';"
        "}).join('');"
        "const feed=document.getElementById('feed');"
        "feed.innerHTML=(d.signal_feed||[]).slice(-30).reverse().map(e=>{"
        "const cl=e.type==='buy'?'green':e.type==='sell'?'red':e.type==='filter'?'yellow':'';"
        "return '<div class=\"feed-item\"><span class=\"ts\">'+(e.ts||'').slice(11,19)+'</span>"
        "<span class=\"'+cl+'\">'+e.msg+'</span></div>';"
        "}).join('');"
        "}catch(err){console.error(err);}"
        "}"
        "setInterval(refresh,5000);refresh();"
        "</script></body></html>"
    )
    dashboard_html = _DASH_HTML.replace("__THEME__", theme)

    filename = f"{name}_bot.py"

    script = f'''#!/usr/bin/env python3
"""
{theme}
{tagline}

Generated by Starter Coach v{meta.get("version", "1.0")}
Strategy : {name}
Risk tier: {risk_tier}
Entry    : {entry_type} on {symbol} ({timeframe})
Chain    : {chain}

⚠️  Paper trade by default (PAPER_TRADE = True). Set to False to go live.
    Requires: onchainos CLI installed, wallet logged in.
    Run    : python3 {filename}
"""
from __future__ import annotations

import argparse, json, sys, time, threading
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

# ── Import OnchainOS from the same skill directory ─────────────────
sys.path.insert(0, str(Path(__file__).parent))
from onchainos import OnchainOS

# ── Config ─────────────────────────────────────────────────────────
PAPER_TRADE      = True
CHAIN            = {repr(chain)}
POLL_SEC         = 30
SYMBOL           = {repr(symbol)}
TIMEFRAME        = {repr(timeframe)}
ENTRY_TYPE       = {repr(entry_type)}
USD_PER_TRADE    = {usd_size}
STOP_LOSS_PCT    = {sl_pct}
TAKE_PROFIT_PCT  = {tp_pct}
{f"TRAIL_STOP_PCT   = {trail_pct}" if trail_pct else "TRAIL_STOP_PCT   = None"}
MAX_DAILY_TRADES = {max_daily}
MAX_CONCURRENT   = {max_conc}
SESSION_LOSS_PAUSE = {sess_pause}
MAX_BUDGET_USD   = {total_budget}   # total budget cap — bot stops spending beyond this
DASHBOARD_PORT   = {dashboard_port}

{token_address_line}

ENTRY_PARAMS = {repr(entry_params)}

SAFETY = {repr(safety_dict)}

# ── OnchainOS client ───────────────────────────────────────────────
oc = OnchainOS(chain=CHAIN)
WALLET_ADDRESS = ""

# ── State ──────────────────────────────────────────────────────────
positions:      list[dict] = []
daily_trades    = 0
consec_losses   = 0
last_reset_day  = ""
signal_feed:    list[dict] = []
total_spent_usd = 0.0   # tracks cumulative live spend against MAX_BUDGET_USD
DASHBOARD_HTML  = {repr(dashboard_html)}

# ── TA helpers ─────────────────────────────────────────────────────
def _ema(vals: list[float], n: int) -> list[float | None]:
    if len(vals) < n: return [None] * len(vals)
    k = 2 / (n + 1)
    seed = sum(vals[:n]) / n
    out: list[float | None] = [None] * (n - 1) + [seed]
    for v in vals[n:]:
        out.append(out[-1] * (1 - k) + v * k)  # type: ignore[operator]
    return out

def _rsi_last(closes: list[float], period: int = 14) -> float | None:
    if len(closes) < period + 1: return None
    deltas = [closes[i] - closes[i - 1] for i in range(1, len(closes))]
    gains  = [max(0.0, d) for d in deltas]
    losses = [max(0.0, -d) for d in deltas]
    ag = sum(gains[:period])  / period
    al = sum(losses[:period]) / period
    for i in range(period, len(gains)):
        ag = (ag * (period - 1) + gains[i])  / period
        al = (al * (period - 1) + losses[i]) / period
    return 100.0 if al == 0 else 100.0 - 100.0 / (1.0 + ag / al)

# ── Event logger ───────────────────────────────────────────────────
def _log(event_type: str, msg: str, data: dict | None = None) -> None:
    ts = datetime.utcnow().isoformat()
    entry = {{"ts": ts, "type": event_type, "msg": msg}}
    if data:
        entry["data"] = data
    signal_feed.append(entry)
    if len(signal_feed) > 100:
        signal_feed.pop(0)
    print(f"[{{ts[11:19]}}] {{msg}}")

# ── USDC address helper ────────────────────────────────────────────
def _usdc_address() -> str:
    return {{
        "solana":   "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "ethereum": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        "base":     "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "bsc":      "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d",
        "arbitrum": "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
    }}.get(CHAIN, "")

# ── Dashboard server ───────────────────────────────────────────────
class _DashHandler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass  # silence access logs
    def _send(self, code: int, ctype: str, body: bytes) -> None:
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def do_GET(self) -> None:
        if self.path == "/api/state":
            closed = [p for p in positions if p["status"] == "closed"]
            total_pnl = sum(p.get("pnl_usd", 0) for p in closed)
            wins  = sum(1 for p in closed if p.get("pnl_usd", 0) > 0)
            losses = len(closed) - wins
            # Add current_price snapshot for open positions
            state_positions = []
            for p in positions:
                snap = dict(p)
                if p["status"] == "open":
                    pd = oc.get_price_info(p["token"])
                    snap["current_price"] = float((pd.get("data") or {{}}).get("price", 0) or 0)
                state_positions.append(snap)
            payload = {{
                "mode":       "paper" if PAPER_TRADE else "live",
                "strategy":   {repr(name)},
                "entry_type": ENTRY_TYPE,
                "sl_pct":     STOP_LOSS_PCT,
                "tp_pct":     TAKE_PROFIT_PCT,
                "poll_sec":   POLL_SEC,
                "wallet":     WALLET_ADDRESS[:8] + "..." + WALLET_ADDRESS[-4:] if WALLET_ADDRESS else "",
                "positions":  state_positions,
                "signal_feed": signal_feed[-50:],
                "stats": {{
                    "total_trades": len(closed),
                    "wins":         wins,
                    "losses":       losses,
                    "pnl_usd":      round(total_pnl, 2),
                    "daily_trades": daily_trades,
                    "daily_limit":  MAX_DAILY_TRADES,
                }},
            }}
            body = json.dumps(payload).encode()
            self._send(200, "application/json", body)
        else:
            body = DASHBOARD_HTML.encode()
            self._send(200, "text/html", body)


def _start_dashboard() -> None:
    srv = HTTPServer(("", DASHBOARD_PORT), _DashHandler)
    t = threading.Thread(target=srv.serve_forever, daemon=True)
    t.start()
    print(f"📊  Dashboard : http://localhost:{{DASHBOARD_PORT}}")


# ── Preflight ──────────────────────────────────────────────────────
def preflight() -> bool:
    global WALLET_ADDRESS
    status = oc.wallet_status()
    logged_in = status.get("loggedIn") or status.get("data", {{}}).get("loggedIn")
    if not logged_in:
        print("❌  Not logged in. Run: onchainos wallet login <email>")
        return False
    addr = oc.get_wallet_address()
    if not addr:
        print("❌  Could not resolve wallet address for chain:", CHAIN)
        return False
    WALLET_ADDRESS = addr
    balances = oc.get_all_balances()
    usdc_bal = next(
        (float(b.get("balanceUsd", 0) or 0) for b in balances
         if "USDC" in (b.get("symbol") or "").upper()),
        0.0,
    )
    print(f"✅  Wallet : {{WALLET_ADDRESS[:8]}}...{{WALLET_ADDRESS[-4:]}}")
    print(f"✅  USDC   : ${{usdc_bal:.2f}}")
    if usdc_bal < USD_PER_TRADE:
        print(f"⚠️   Low balance: ${{usdc_bal:.2f}} — need ≥ ${{USD_PER_TRADE}} to trade")
    if not _usdc_address():
        print(f"⚠️   USDC address not mapped for chain {{CHAIN}} — live swaps will fail")
    return True

# ── Safety filters ─────────────────────────────────────────────────
def passes_safety(token: str) -> tuple[bool, list[str]]:
    """Run safety filters derived from the spec. Returns (passed, [failed_reasons])."""
    if not SAFETY:
        return True, []
    tags   = oc.get_safety_tags(token)
    failed: list[str] = []
    if SAFETY.get("honeypot_check") and tags.honeypot:
        failed.append("HONEYPOT")
    if SAFETY.get("phishing_exclude") and tags.phishing_flagged:
        failed.append("PHISHING")
    if (v := SAFETY.get("lp_locked_min_pct")) and tags.lp_locked_pct < v:
        failed.append(f"LP lock {{tags.lp_locked_pct:.0f}}%<{{v}}%")
    if (v := SAFETY.get("buy_tax_max")) and tags.buy_tax_pct > v:
        failed.append(f"buy_tax {{tags.buy_tax_pct:.1f}}%>{{v}}%")
    if (v := SAFETY.get("sell_tax_max")) and tags.sell_tax_pct > v:
        failed.append(f"sell_tax {{tags.sell_tax_pct:.1f}}%>{{v}}%")
    if (v := SAFETY.get("liquidity_min_usd")) and tags.liquidity_usd < v:
        failed.append(f"liq ${{tags.liquidity_usd:.0f}}<${{v}}")
    if (v := SAFETY.get("top_holders_max_pct")) and tags.top_holders_pct > v:
        failed.append(f"top_holders {{tags.top_holders_pct:.0f}}%>{{v}}%")
    if (v := SAFETY.get("bundler_max_pct")) and tags.bundler_ratio_pct > v:
        failed.append(f"bundler {{tags.bundler_ratio_pct:.0f}}%>{{v}}%")
    if (v := SAFETY.get("dev_holding_max_pct")) and tags.dev_holding_pct > v:
        failed.append(f"dev_hold {{tags.dev_holding_pct:.0f}}%>{{v}}%")
    if (v := SAFETY.get("insider_max_pct")) and tags.insider_holding_pct > v:
        failed.append(f"insider {{tags.insider_holding_pct:.0f}}%>{{v}}%")
    if (v := SAFETY.get("whale_max_pct")) and tags.whale_max_pct > v:
        failed.append(f"whale {{tags.whale_max_pct:.0f}}%>{{v}}%")
    if (v := SAFETY.get("sm_present_min")) and tags.smart_money_count < v:
        failed.append(f"SM count {{tags.smart_money_count}}<{{v}}")
    if (v := SAFETY.get("mcap_min_usd")) and tags.mcap_usd < v:
        failed.append(f"mcap ${{tags.mcap_usd:.0f}}<${{v}}")
    if (v := SAFETY.get("mcap_max_usd")) and tags.mcap_usd > v:
        failed.append(f"mcap ${{tags.mcap_usd:.0f}}>${{v}}")
    return len(failed) == 0, failed

# ── Signal detection ───────────────────────────────────────────────
{signal_block}

# ── Position lifecycle ─────────────────────────────────────────────
def open_position(token: str, symbol_str: str, signal_data: dict) -> None:
    global daily_trades, total_spent_usd
    if daily_trades >= MAX_DAILY_TRADES:
        return
    if sum(1 for p in positions if p["status"] == "open") >= MAX_CONCURRENT:
        return
    if any(p["token"] == token and p["status"] == "open" for p in positions):
        return  # already in this token
    if not PAPER_TRADE and total_spent_usd + USD_PER_TRADE > MAX_BUDGET_USD:
        _log("guard", f"[BUDGET CAP] ${total_spent_usd:.2f} spent — max budget ${MAX_BUDGET_USD} reached")
        return

    price_data  = oc.get_price_info(token)
    entry_price = float((price_data.get("data") or {{}}).get("price", 0) or 0)
    if entry_price <= 0:
        print(f"[SKIP] No price for {{symbol_str or token[:12]}}")
        return

    ts  = datetime.utcnow().isoformat()
    pos = {{
        "token": token, "symbol": symbol_str, "entry_price": entry_price,
        "usd": USD_PER_TRADE, "entry_ts": ts, "status": "open",
        "peak_price": entry_price, "signal_data": signal_data, "mode": "paper",
    }}

    if PAPER_TRADE:
        _log("buy", f"[PAPER] BUY {{symbol_str or token[:12]}} @ ${{entry_price:.6f}} | ${{USD_PER_TRADE}} | SL -{{STOP_LOSS_PCT}}%")
    else:
        usdc = _usdc_address()
        if not usdc:
            _log("error", f"[ERROR] USDC address not mapped for {{CHAIN}}"); return
        result = oc.swap_execute(usdc, token, str(USD_PER_TRADE), WALLET_ADDRESS)
        if not result.ok:
            _log("error", f"[ERROR] Swap failed: {{result.error}}"); return
        pos["mode"]    = "live"
        pos["tx_hash"] = result.tx_hash
        total_spent_usd += USD_PER_TRADE
        _log("buy", f"[LIVE] BUY {{symbol_str}} @ ${{entry_price:.6f}} | tx: {{result.tx_hash[:16]}}... | total spent ${{total_spent_usd:.2f}}/${{MAX_BUDGET_USD}}")

    positions.append(pos)
    daily_trades += 1


def close_position(pos: dict, reason: str, current_price: float) -> None:
    global consec_losses
    ep  = pos["entry_price"]
    pnl_pct = (current_price - ep) / ep * 100 if ep else 0.0
    pnl_usd = pos["usd"] * pnl_pct / 100
    pos.update({{
        "status": "closed", "exit_ts": datetime.utcnow().isoformat(),
        "exit_price": current_price, "exit_reason": reason,
        "pnl_pct": round(pnl_pct, 2), "pnl_usd": round(pnl_usd, 2),
    }})
    consec_losses = consec_losses + 1 if pnl_pct < 0 else 0
    icon = "📗" if pnl_pct >= 0 else "📕"
    _log("sell", f"{{icon}} CLOSE {{pos['symbol'] or pos['token'][:12]}} | {{reason}} | {{pnl_pct:+.1f}}% | ${{pnl_usd:+.2f}}")

    if not PAPER_TRADE and pos.get("mode") == "live":
        usdc = _usdc_address()
        bal  = oc.get_token_balance(pos["token"])
        if bal > 0 and usdc:
            result = oc.swap_execute(pos["token"], usdc, str(bal), WALLET_ADDRESS)
            if result.ok:
                print(f"   exit tx: {{result.tx_hash[:16]}}...")
            else:
                print(f"   [ERROR] Exit swap failed: {{result.error}}")


# ── Monitor open positions ─────────────────────────────────────────
def monitor_positions() -> None:
    for pos in [p for p in positions if p["status"] == "open"]:
        price_data    = oc.get_price_info(pos["token"])
        current_price = float((price_data.get("data") or {{}}).get("price", 0) or 0)
        if not current_price:
            continue

        pos["peak_price"] = max(pos.get("peak_price", 0), current_price)
        ep      = pos["entry_price"]
        pnl_pct = (current_price - ep) / ep * 100 if ep else 0.0

{dev_dump_block}
{fast_dump_block}

        # Trailing stop
        if TRAIL_STOP_PCT is not None:
            peak  = pos["peak_price"]
            trail = (peak - current_price) / peak * 100 if peak else 0.0
            if trail >= TRAIL_STOP_PCT:
                close_position(pos, "trailing_stop", current_price)
                continue

        # Stop loss
        if pnl_pct <= -STOP_LOSS_PCT:
            close_position(pos, "stop_loss", current_price)
            continue

        # Tiered take profit
{tiered_block}

        # Flat take profit
        if pnl_pct >= TAKE_PROFIT_PCT:
            close_position(pos, "take_profit", current_price)
            continue

        print(f"[POS] {{pos['symbol'] or pos['token'][:12]}} | ${{ep:.6f}} → ${{current_price:.6f}} | {{pnl_pct:+.1f}}%")


# ── Risk guards ────────────────────────────────────────────────────
def reset_daily() -> None:
    global daily_trades, last_reset_day
    today = datetime.utcnow().strftime("%Y-%m-%d")
    if today != last_reset_day:
        daily_trades   = 0
        last_reset_day = today


def should_pause() -> bool:
    if consec_losses >= SESSION_LOSS_PAUSE:
        print(f"[PAUSE] {{consec_losses}} consecutive losses — sitting out this session")
        return True
    return False


# ── Main loop ──────────────────────────────────────────────────────
def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--smoke-test", action="store_true", help="Verify dashboard endpoints then exit")
    args = ap.parse_args()

    print(f"""
╔══════════════════════════════════════╗
║  {theme[:36]:<36} ║
║  {(tagline[:36] if tagline else ""):<36} ║
╠══════════════════════════════════════╣
║  Mode   : {{"PAPER 📄" if PAPER_TRADE else "LIVE  🔴":<29}} ║
║  Entry  : {{ENTRY_TYPE:<29}} ║
║  SL -{{STOP_LOSS_PCT}}%  TP +{{TAKE_PROFIT_PCT}}%{" " * 21} ║
╚══════════════════════════════════════╝
""")

    if args.smoke_test:
        import urllib.request as _ureq
        _start_dashboard()
        time.sleep(1.5)
        try:
            r1 = _ureq.urlopen(f"http://localhost:{{DASHBOARD_PORT}}/", timeout=4)
            assert r1.status == 200, f"/ returned {{r1.status}}"
            r2 = _ureq.urlopen(f"http://localhost:{{DASHBOARD_PORT}}/api/state", timeout=4)
            assert r2.status == 200, f"/api/state returned {{r2.status}}"
            json.loads(r2.read())  # must be valid JSON
            print("SMOKE_TEST_OK")
            sys.exit(0)
        except Exception as e:
            print(f"SMOKE_TEST_FAIL: {{e}}")
            sys.exit(1)

    if not preflight():
        sys.exit(1)

    # ── Live mode gate ─────────────────────────────────────────────
    if not PAPER_TRADE:
        print("\\n" + "="*50)
        print("🔴  LIVE MODE — REAL FUNDS AT RISK")
        print("="*50)
        print(f"  Strategy   : {theme}")
        print(f"  Per trade  : ${{{usd_size}}} USD")
        print(f"  Budget cap : ${{MAX_BUDGET_USD}} USD")
        print(f"  Stop loss  : -{{STOP_LOSS_PCT}}%")
        print("\\nReal money will be used. Losses are permanent.")
        print("Safety filters do not guarantee protection against all fraud.")
        print("Automated trading may generate significant taxable events.")
        print("\\nType CONFIRM to proceed, or press Ctrl+C to cancel:\\n")
        try:
            user_input = input("> ").strip()
        except (KeyboardInterrupt, EOFError):
            print("\\nCancelled. Run with PAPER_TRADE = True to test first.")
            sys.exit(0)
        if user_input.upper() != "CONFIRM":
            print("\\nLive mode not activated. Set PAPER_TRADE = True to run in paper mode.")
            sys.exit(0)
        # Log acknowledgment
        import json as _json
        from pathlib import Path as _P
        _ack = {{"ts": datetime.utcnow().isoformat(), "strategy": {repr(name)},
                 "confirmation": "CONFIRM", "budget_cap": MAX_BUDGET_USD}}
        _ack_path = _P(__file__).parent / f"{repr(name)[1:-1]}_live_ack.jsonl"
        with open(_ack_path, "a") as _f:
            _f.write(_json.dumps(_ack) + "\\n")
        print("\\n✅ Live mode confirmed. Starting bot...\\n")

    _start_dashboard()

    print(f"Scanning every {{POLL_SEC}}s — Ctrl+C to stop\\n")

    try:
        while True:
            reset_daily()
            if should_pause():
                time.sleep(POLL_SEC * 2)
                continue

            monitor_positions()

            open_cnt = sum(1 for p in positions if p["status"] == "open")
            if open_cnt < MAX_CONCURRENT and daily_trades < MAX_DAILY_TRADES:
                for cand in find_candidates():
                    if not cand.get("token"):
                        continue
                    ok, failed = passes_safety(cand["token"])
                    if ok:
                        open_position(cand["token"], cand.get("symbol", ""), cand.get("signal_data", {{}}))
                    else:
                        sym = cand.get("symbol") or cand["token"][:8]
                        _log("filter", f"[FILTER] {{sym}} ✗ {{', '.join(failed)}}")

            time.sleep(POLL_SEC)

    except KeyboardInterrupt:
        closed    = [p for p in positions if p["status"] == "closed"]
        open_pos  = sum(1 for p in positions if p["status"] == "open")
        total_pnl = sum(p.get("pnl_usd", 0) for p in closed)
        wins      = sum(1 for p in closed if p.get("pnl_usd", 0) > 0)
        print(f"\\n👋 Bot stopped.")
        print(f"   {{len(closed)}} trades | {{wins}}W / {{len(closed)-wins}}L | PnL: ${{total_pnl:+.2f}}")
        if open_pos:
            print(f"   ⚠️  {{open_pos}} position(s) still open — close manually if live")


if __name__ == "__main__":
    main()
'''

    return filename, script


def verify_bot_script(filepath: str, code: str = "") -> tuple[bool, str]:
    """
    Full harness for a generated bot script. Three-layer check:
      1. Syntax  — py_compile
      2. Methods — all oc.xxx() calls resolve against OnchainOS
      3. Dashboard — --smoke-test: start server, hit / and /api/state
    Returns (ok, error_message).
    """
    import subprocess as _sp, re as _re, sys as _sys
    from pathlib import Path as _Path

    # Layer 1: syntax
    r = _sp.run(["python3", "-m", "py_compile", filepath], capture_output=True, text=True)
    if r.returncode != 0:
        return False, f"[syntax] {r.stderr.strip()}"

    # Layer 2: OnchainOS method names
    src = code or _Path(filepath).read_text()
    calls = set(_re.findall(r'\boc\.(\w+)\(', src))
    _sys.path.insert(0, str(_Path(__file__).parent))
    from onchainos import OnchainOS as _OC
    valid_methods = {m for m in dir(_OC) if not m.startswith('_')}
    bad = sorted(calls - valid_methods)
    if bad:
        return False, f"[onchainos] Unknown methods: {', '.join(bad)}"

    # Layer 3: dashboard smoke-test
    r2 = _sp.run(
        ["python3", filepath, "--smoke-test"],
        capture_output=True, text=True, timeout=12,
    )
    if r2.returncode != 0 or "SMOKE_TEST_OK" not in r2.stdout:
        out = (r2.stdout + r2.stderr).strip()
        return False, f"[dashboard] {out or 'smoke-test failed'}"

    return True, ""


def build_spec_from_profile(state: CoachState) -> tuple[dict[str, Any], list[str]]:
    """
    Generate a strategy spec from the user profile and selected template.
    Returns (spec, errors). If errors is empty, spec is valid.
    """
    profile = state.profile
    goal_key = state.selected_goal or _infer_goal(profile.get("goal", ""))
    template = GOAL_TEMPLATES.get(goal_key)
    if not template:
        return {}, [f"Unknown goal: {goal_key}"]

    risk = RISK_PRESETS.get(
        _infer_risk_level(profile.get("risk_level", "moderate")),
        RISK_PRESETS["moderate"],
    )

    # Handle grid separately
    if goal_key == "grid":
        return _build_grid_spec(profile, risk)

    # Build instrument — token/chain inferred from goal (no explicit questions)
    raw_token = profile.get("token") or _infer_token_from_goal(goal_key)
    token = _normalize_token(raw_token)
    chain = profile.get("chain") or _infer_chain_from_token(token)
    timeframe = "5m" if template.get("live_only") else "1H"
    symbol = f"{token}-USDC" if goal_key not in ("meme_sniper", "smart_money", "copy_trade") else "*"

    spec: dict[str, Any] = {
        "meta": {
            "name": _make_name(goal_key, token),
            "version": "1.0",
            "risk_tier": _infer_risk_level(profile.get("risk_level", "moderate")),
            "description": template["description"],
            "author_intent": profile.get("goal", ""),
        },
        "instrument": {
            "symbol": symbol,
            "timeframe": timeframe,
        },
        "entry": {
            "type": template["entry_type"],
            **template["entry_params"],
        },
        "exit": _build_exit(template["exit_style"], risk),
        "sizing": _build_sizing(profile, risk),
        "filters": [],
        "risk_overlays": _build_overlays(risk),
    }

    # Add universe for dynamic-symbol strategies
    if symbol == "*":
        spec["universe"] = {
            "selector": template["entry_type"],
            "chain": chain,
        }

    # Copy-trade: inject wallet addresses into entry AND mirror exit
    if goal_key == "copy_trade":
        wallets = profile.get("target_wallets", [])
        if not wallets:
            # Cannot build a valid copy_trade spec without wallet addresses — harness requires minItems:1
            return spec, ["copy_trade requires at least one target wallet address. "
                          "Ask the user: 'Which wallet address do you want to copy?'"]
        spec["entry"]["target_wallet"] = wallets
        # Also inject into wallet_mirror_sell exit
        for other_exit in spec["exit"].get("other", []):
            if other_exit.get("type") == "wallet_mirror_sell":
                other_exit["target_wallet"] = wallets

    # Safety stack for meme/live_only
    if template.get("needs_safety_stack"):
        spec["filters"] = _build_safety_stack(risk)

    # Beginner defaults
    if _infer_experience(profile.get("experience", "")) == "beginner":
        spec["filters"].append({"type": "cooldown", "bars": 6})
        # Ensure session_loss_pause is present
        overlay_types = {o["type"] for o in spec["risk_overlays"]}
        if "session_loss_pause" not in overlay_types:
            spec["risk_overlays"].append({
                "type": "session_loss_pause",
                "max_consecutive_losses": 2,
            })

    # Validate
    ok, errors, meta = validate_spec(spec)
    if meta.get("live_only"):
        spec["meta"]["live_only"] = True

    state.strategy_spec = spec
    save_state(state)

    return spec, errors


def build_spec_llm_first(state: CoachState) -> tuple[dict[str, Any], str, str, list[str]]:
    """
    Generate strategy spec using LLM + primitives. Falls back to template on failure.

    Returns (spec, theme, tagline, errors).
    Theme + tagline are always populated (fallback values used if LLM unavailable).
    """
    goal_key = state.selected_goal or _infer_goal(state.profile.get("goal", ""))

    # Try LLM generation first
    spec, theme, tagline, errors = generate_strategy_spec(state.profile)

    if spec and not errors:
        # Validate LLM output
        ok, val_errors, meta = validate_spec(spec)
        if ok or not val_errors:
            if meta.get("live_only"):
                spec["meta"]["live_only"] = True
            state.strategy_spec = spec
            save_state(state)
            return spec, theme, tagline, []
        # LLM spec failed validation — fall through to template
        errors = val_errors

    # Fall back to template
    spec, template_errors = build_spec_from_profile(state)
    fb_theme, fb_tagline = get_fallback_theme(goal_key)
    theme   = theme   or fb_theme
    tagline = tagline or fb_tagline

    # Embed theme into spec meta
    if spec:
        spec.setdefault("meta", {}).update({"theme": theme, "tagline": tagline})
        state.strategy_spec = spec
        save_state(state)

    return spec, theme, tagline, template_errors


def record_backtest_result(state: CoachState, result: dict[str, Any]) -> CoachState:
    """Store a backtest result for coaching loop."""
    state.backtest_history.append({
        "ts": time.time(),
        "result": result,
    })
    save_state(state)
    return state


def record_improvement(state: CoachState, change: str) -> CoachState:
    """Log a parameter improvement."""
    state.improvements.append(change)
    save_state(state)
    return state


def mark_live(state: CoachState) -> CoachState:
    """Mark strategy as deployed live."""
    state.live_deployed = True
    state.step = 5
    save_state(state)
    return state


def confirm_live_mode(state: CoachState, confirmation_text: str) -> tuple[bool, str]:
    """Process user's live mode confirmation.

    User must type exactly 'CONFIRM' (case-insensitive).
    Logs the acknowledgment to a local file for compliance purposes.
    Returns (confirmed, message).
    """
    if confirmation_text.strip().upper() != "CONFIRM":
        return False, "❌ Live mode not activated. Type `CONFIRM` to proceed, or keep running in paper mode."

    state.live_confirmed = True
    state.live_confirmed_ts = time.time()
    save_state(state)

    # Write logged acknowledgment (legal requirement — retain 5 years)
    ack_path = STATE_DIR / f"{state.session_id}_live_ack.jsonl"
    ack_record = {
        "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "session_id": state.session_id,
        "strategy": state.strategy_spec.get("meta", {}).get("name", ""),
        "confirmation": "CONFIRM",
        "message": "User confirmed: real funds at risk, no profit guarantee, full liability assumed",
    }
    with open(ack_path, "a") as f:
        f.write(json.dumps(ack_record) + "\n")

    return True, "✅ Live mode confirmed and logged. Proceeding to live trading."


def render_bot_summary(spec: dict[str, Any], profile: dict[str, Any] | None = None) -> str:
    """Generate a plain-English summary of what the bot will do before going live."""
    meta    = spec.get("meta", {})
    entry   = spec.get("entry", {})
    exit_   = spec.get("exit", {})
    sizing  = spec.get("sizing", {})
    overlays = spec.get("risk_overlays", [])

    entry_type = entry.get("type", "unknown")
    entry_labels = {
        "wallet_copy_buy": "copy trades from specific wallets",
        "smart_money_buy": "buy when smart money / whale wallets buy",
        "ranking_entry":   "snipe new trending tokens",
        "price_drop":      f"buy when price drops {entry.get('pct', '?')}%",
        "ma_cross":        "buy on moving average crossover signal",
        "rsi_threshold":   "buy when RSI hits oversold level",
        "time_schedule":   f"buy on a fixed {entry.get('interval', 'scheduled')} schedule",
        "bollinger_touch":  "buy when price touches Bollinger Band",
        "volume_spike":    "buy on volume spike signal",
        "macd_cross":      "buy on MACD crossover signal",
    }
    entry_desc = entry_labels.get(entry_type, entry_type.replace("_", " "))

    sl = exit_.get("stop_loss", {}).get("pct", "?")
    tp_pct = exit_.get("take_profit", {}).get("pct")
    trail = exit_.get("trailing_stop", {}).get("pct")
    tiered = exit_.get("tiered_take_profit")

    if tiered:
        tp_levels = ", ".join(f"+{t['pct_gain']}%" for t in tiered.get("tiers", []))
        exit_desc = f"tiered take profit at {tp_levels}, stop loss at -{sl}%"
    elif trail:
        exit_desc = f"trailing stop at -{trail}%, stop loss at -{sl}%"
    elif tp_pct:
        exit_desc = f"take profit at +{tp_pct}%, stop loss at -{sl}%"
    else:
        exit_desc = f"stop loss at -{sl}%"

    usd = sizing.get("usd", "?") if sizing.get("type") == "fixed_usd" else "?"
    budget = (profile or {}).get("total_budget", "")
    budget_note = f" (out of your ${budget} total budget)" if budget else ""

    max_daily = next((o.get("n") for o in overlays if o.get("type") == "max_daily_trades"), None)
    max_conc  = next((o.get("n") for o in overlays if o.get("type") == "max_concurrent_positions"), None)
    dd_pause  = next((o.get("pause_pct") for o in overlays if o.get("type") == "drawdown_pause"), None)

    guards = []
    if max_daily: guards.append(f"max {max_daily} trades/day")
    if max_conc:  guards.append(f"max {max_conc} positions at once")
    if dd_pause:  guards.append(f"pause if drawdown hits -{dd_pause}%")

    filters = spec.get("filters", [])
    safety_note = f" with {len(filters)} safety checks" if filters else ""

    lines = [
        f"**Your bot will:**",
        f"- Watch the market and **{entry_desc}**{safety_note}",
        f"- Spend **${usd} per trade**{budget_note}",
        f"- Exit positions via **{exit_desc}**",
    ]
    if guards:
        lines.append(f"- Stop and protect capital if: {', '.join(guards)}")
    lines.append(f"\n*⚠️ Safety filters reduce risk but do not guarantee protection against all losses.*")

    return "\n".join(lines)


def enable_evolve(state: CoachState) -> CoachState:
    """Enable auto-evolve engine."""
    state.evolve_enabled = True
    state.step = 6
    save_state(state)
    return state


# ── Internal Helpers ──────────────────────────────────────────────

def _infer_goal(text: str) -> str:
    """Map free-text goal to a template key."""
    text = text.lower()
    if any(w in text for w in ("dca", "schedule", "weekly", "daily")):
        return "dca"
    if any(w in text for w in ("dip", "drop", "crash")):
        return "dip_buy"
    if any(w in text for w in ("trend", "momentum", "breakout")):
        return "trend_follow"
    if any(w in text for w in ("rsi", "oversold", "mean revert", "bollinger")):
        return "mean_revert"
    if any(w in text for w in ("smart money", "whale", "follow smart")):
        return "smart_money"
    if any(w in text for w in ("copy", "mirror", "wallet")):
        return "copy_trade"
    if any(w in text for w in ("snipe", "meme", "trending", "new token", "pump")):
        return "meme_sniper"
    if "grid" in text:
        return "grid"
    return "dip_buy"  # safe default


def _infer_risk_level(text: str) -> str:
    text = text.lower()
    if any(w in text for w in (
        "conservative", "safe", "low", "mild",
        "sleep easy", "sleep well", "protect",   # new labels
        "保守", "稳", "睡得着",
    )):
        return "conservative"
    if any(w in text for w in (
        "aggressive", "high", "yolo", "ghost pepper", "lfg",
        "send it", "send", "degen", "go big",    # new labels
        "梭哈", "激进",
    )):
        return "aggressive"
    return "moderate"


def _infer_experience(text: str) -> str:
    text = text.lower()
    if any(w in text for w in ("beginner", "first", "new", "never")):
        return "beginner"
    if any(w in text for w in ("advanced", "expert", "pro")):
        return "advanced"
    return "intermediate"


def _infer_token_from_goal(goal_key: str) -> str:
    """Default token for each goal — no explicit token question needed."""
    return {
        "meme_sniper": "trending",
        "smart_money": "dynamic",
        "copy_trade": "dynamic",
        "grid": "SOL",
    }.get(goal_key, "SOL")


def _infer_chain_from_token(token: str) -> str:
    """Infer chain from token symbol — defaults to Solana."""
    t = token.upper()
    if t in ("ETH", "WETH", "USDC", "USDT", "LINK", "UNI", "AAVE", "WBTC"):
        return "ethereum"
    if t in ("BNB", "CAKE", "BUSD"):
        return "bsc"
    return "solana"


def _infer_automation_from_risk(risk: str) -> str:
    """Conservative users get co-pilot; everyone else gets full auto."""
    return "co_pilot" if risk == "conservative" else "full_auto"


def _normalize_token(token: str) -> str:
    """Normalize token name to uppercase symbol."""
    mapping = {
        "solana": "SOL", "sol": "SOL",
        "ethereum": "ETH", "eth": "ETH",
        "bitcoin": "BTC", "btc": "BTC", "wbtc": "WBTC",
        "bnb": "BNB", "avax": "AVAX", "doge": "DOGE",
    }
    return mapping.get(token.lower(), token.upper())


def _make_name(goal_key: str, token: str) -> str:
    """Generate a spec-compliant name."""
    name = f"{goal_key}_{token.lower()}"
    # Ensure ^[a-z0-9_]{3,64}$
    name = "".join(c for c in name if c.isalnum() or c == "_").lower()
    return name[:64] if len(name) >= 3 else f"{name}_strategy"


def _build_exit(style: str, risk: dict[str, Any]) -> dict[str, Any]:
    """Build exit block based on style."""
    sl = {"pct": risk["sl_pct"]}

    if style == "sl_tp":
        return {
            "stop_loss": sl,
            "take_profit": {"pct": risk["tp_pct"]},
        }
    elif style == "trailing":
        return {
            "stop_loss": sl,
            "trailing_stop": {"pct": min(risk["sl_pct"], 20)},
        }
    elif style == "tiered_tp":
        return {
            "stop_loss": sl,
            "tiered_take_profit": {
                "tiers": [
                    {"pct_gain": 50, "pct_sell": 33},
                    {"pct_gain": 200, "pct_sell": 33},
                    {"pct_gain": 500, "pct_sell": 34},
                ],
            },
            "other": [
                {"type": "fast_dump_exit", "drop_pct": 30, "window_sec": 60},
            ],
        }
    elif style == "sm_exit":
        return {
            "stop_loss": sl,
            "other": [
                {"type": "smart_money_sell", "min_wallets": 2, "window_min": 60},
                {"type": "fast_dump_exit", "drop_pct": 30, "window_sec": 60},
            ],
        }
    elif style == "mirror_exit":
        return {
            "stop_loss": sl,
            "other": [
                {"type": "wallet_mirror_sell", "target_wallet": ["PLACEHOLDER"], "min_pct_sold": 50},
                {"type": "dev_dump", "min_usd": 500},
                {"type": "fast_dump_exit", "drop_pct": 30, "window_sec": 60},
            ],
        }
    else:
        return {"stop_loss": sl}


_BUDGET_ALLOC = {
    "conservative": 0.10,  # 10% per trade → up to 10 concurrent positions
    "moderate":     0.15,  # 15% per trade → up to ~7 concurrent positions
    "aggressive":   0.20,  # 20% per trade → up to 5 concurrent positions
}


def _build_sizing(profile: dict[str, Any], risk: dict[str, Any]) -> dict[str, Any]:
    """Derive per-trade size from total_budget + risk tier.

    Falls back to risk preset sizing_usd if no total_budget provided.
    """
    total = profile.get("total_budget") or profile.get("budget_per_trade")
    if total and isinstance(total, (int, float)):
        tier = risk.get("risk_tier", "moderate")
        pct = _BUDGET_ALLOC.get(tier, 0.15)
        per_trade = round(total * pct, 2)
        return {"type": "fixed_usd", "usd": max(per_trade, 10)}  # schema minimum is 10
    return {"type": "fixed_usd", "usd": risk["sizing_usd"]}


def _build_overlays(risk: dict[str, Any]) -> list[dict[str, Any]]:
    """Build risk overlays from risk preset."""
    return [
        {"type": "max_daily_trades", "n": risk["max_daily_trades"]},
        {"type": "max_concurrent_positions", "n": risk["max_concurrent"]},
        {"type": "drawdown_pause", "pause_pct": risk["max_dd_pause"]},
        {"type": "session_loss_pause", "max_consecutive_losses": risk["session_loss_pause"]},
    ]


def _build_safety_stack(risk: dict[str, Any]) -> list[dict[str, Any]]:
    """Build full token safety filter stack for meme/live_only strategies."""
    return [
        {"type": "mcap_range", "min_usd": 50000, "max_usd": 5000000},
        {"type": "launch_age", "min_hours": 1, "max_hours": 168},
        {"type": "honeypot_check"},
        {"type": "lp_locked", "min_pct_locked": 70, "min_lock_days": 14},
        {"type": "buy_tax_max", "max_pct": 5},
        {"type": "sell_tax_max", "max_pct": 5},
        {"type": "liquidity_min", "min_usd": 10000},
        {"type": "top_holders_max", "top_n": 10, "max_pct": 40},
        {"type": "bundler_ratio_max", "max_pct": 15},
        {"type": "dev_holding_max", "max_pct": 15},
        {"type": "insider_holding_max", "max_pct": 20},
        {"type": "fresh_wallet_ratio_max", "max_pct": 30},
        {"type": "smart_money_present_min", "min_wallets": 1},
        {"type": "phishing_exclude"},
        {"type": "whale_concentration_max", "max_pct": 20},
    ]


def _build_grid_spec(
    profile: dict[str, Any], risk: dict[str, Any]
) -> tuple[dict[str, Any], list[str]]:
    """Build a grid meta-template spec."""
    raw_token = profile.get("token") or _infer_token_from_goal("grid")
    token = _normalize_token(raw_token)
    total = profile.get("total_budget") or profile.get("budget_per_trade")
    budget = total * (risk.get("sizing_pct", 5) / 100) if total else risk["sizing_usd"]

    spec = {
        "meta": {
            "name": f"grid_{token.lower()}",
            "version": "1.0",
            "risk_tier": "moderate",
            "description": f"Grid trading {token} within a price range",
        },
        "instrument": {
            "symbol": f"{token}-USDC",
            "timeframe": "1H",
        },
        "grid": {
            "price_min": 0,       # User must fill these
            "price_max": 0,
            "levels": 10,
            "usd_per_level": budget,
            "take_profit_per_level_pct": 3,
            "portfolio_stop_loss_pct": 20,
        },
    }
    errors = ["Grid spec needs price_min and price_max. Ask the user for the price range."]
    return spec, errors


def _suggest_templates(goal_key: str) -> list[dict[str, Any]]:
    """Suggest 1-3 templates based on goal."""
    primary = GOAL_TEMPLATES.get(goal_key)
    suggestions = []
    if primary:
        suggestions.append({"key": goal_key, **primary})

    # Add related alternatives
    related: dict[str, list[str]] = {
        "dca": ["dip_buy"],
        "dip_buy": ["dca", "mean_revert"],
        "trend_follow": ["dip_buy"],
        "mean_revert": ["trend_follow", "dip_buy"],
        "smart_money": ["copy_trade", "meme_sniper"],
        "copy_trade": ["smart_money"],
        "meme_sniper": ["smart_money"],
        "grid": ["dca"],
    }
    for alt_key in related.get(goal_key, []):
        if len(suggestions) >= 3:
            break
        alt = GOAL_TEMPLATES.get(alt_key)
        if alt:
            suggestions.append({"key": alt_key, **alt})

    return suggestions
