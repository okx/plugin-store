from decimal import Decimal
from typing import Any, Dict, List, Optional, Tuple

from .models import optional_decimal


STABLE_SYMBOLS = {"USDC", "USDT", "DAI", "USDE", "USDS", "PYUSD", "FDUSD", "TUSD", "FRAX", "USD0"}
NATIVE_SYMBOLS = {"OKB", "WOKB", "SOL", "WSOL", "ETH", "WETH", "BNB", "WBNB", "MATIC", "WMATIC"}
BAD_TAGS = {"honeypot", "scam", "rug", "rugpull", "phishing", "blacklist", "malicious"}


def select_candidate(request: Dict[str, Any]) -> Dict[str, Any]:
    explicit = request.get("tokenOut")
    if explicit:
        candidate = dict(explicit)
        score, reasons = score_candidate(candidate)
        candidate.setdefault("source", "user-request")
        candidate["selectionScore"] = score
        candidate["selectionReasons"] = reasons
        return candidate

    candidates = request.get("candidates") or []
    if not candidates:
        token = {"symbol": "<candidate-token>", "source": "onchainos-token-discovery-required"}
        token["selectionScore"] = 0
        token["selectionReasons"] = ["No token candidate supplied; agent must query onchainOS token/signal data."]
        return token

    scored: List[Tuple[int, Dict[str, Any], List[str]]] = []
    for candidate in candidates:
        score, reasons = score_candidate(candidate)
        scored.append((score, candidate, reasons))
    scored.sort(key=lambda item: (-item[0], str(item[1].get("symbol") or item[1].get("address") or "")))
    best_score, best, reasons = scored[0]
    output = dict(best)
    output["selectionScore"] = best_score
    output["selectionReasons"] = reasons
    return output


def score_candidate(candidate: Dict[str, Any]) -> Tuple[int, List[str]]:
    score = 50
    reasons: List[str] = []
    symbol = str(candidate.get("symbol") or "").upper()
    tags = _lower_set(candidate.get("tags") or candidate.get("tokenTags") or candidate.get("riskTags") or [])
    risk_level = str(candidate.get("riskLevel") or candidate.get("riskControlLevel") or "").upper()

    if symbol in STABLE_SYMBOLS or symbol in NATIVE_SYMBOLS:
        score -= 35
        reasons.append("Candidate is stable/native; competition mode prefers real token trades.")
    if risk_level in ("CRITICAL", "HIGH"):
        score -= 45
        reasons.append("Candidate has high or critical risk metadata.")
    elif risk_level in ("LOW", "SAFE"):
        score += 10
        reasons.append("Candidate has low risk metadata.")
    if tags & BAD_TAGS:
        score -= 60
        reasons.append("Candidate has critical risk tags.")

    liquidity = _decimal(candidate, ["liquidityUsd", "liquidity", "liquidityUSD"])
    if liquidity is not None:
        if liquidity >= Decimal("100000"):
            score += 15
            reasons.append("Liquidity is healthy.")
        elif liquidity < Decimal("25000"):
            score -= 20
            reasons.append("Liquidity is low for competition trading.")

    volume = _decimal(candidate, ["volume24hUsd", "volumeUsd", "volume24h"])
    if volume is not None:
        if volume >= Decimal("50000"):
            score += 10
            reasons.append("Recent trading activity is meaningful.")
        elif volume < Decimal("5000"):
            score -= 10
            reasons.append("Recent trading activity is thin.")

    holder_concentration = _decimal(candidate, ["top10HolderPercent", "topHolderPercent"])
    if holder_concentration is not None and holder_concentration >= Decimal("70"):
        score -= 15
        reasons.append("Holder concentration is elevated.")

    score = max(0, min(100, score))
    if not reasons:
        reasons.append("Candidate selected from supplied onchainOS context.")
    return score, reasons


def is_real_competition_token(token: Optional[Dict[str, Any]]) -> bool:
    if not token:
        return False
    symbol = str(token.get("symbol") or "").upper()
    return bool(symbol) and symbol not in STABLE_SYMBOLS and symbol not in NATIVE_SYMBOLS


def _decimal(candidate: Dict[str, Any], keys: List[str]) -> Optional[Decimal]:
    for key in keys:
        value = optional_decimal(candidate.get(key))
        if value is not None:
            return value
    return None


def _lower_set(values: Any) -> set:
    if isinstance(values, str):
        values = [values]
    if not isinstance(values, list):
        return set()
    return {str(value).strip().lower() for value in values}
