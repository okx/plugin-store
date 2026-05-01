"""
edge_scorer.py — Multi-factor edge detection for Polymarket Liquidity Sniper
"""

import math
import logging
import os
from dotenv import load_dotenv

load_dotenv()
log = logging.getLogger("edge_scorer")

MIN_EDGE            = float(os.getenv("MIN_EDGE",            "0.04"))
MAX_SPREAD          = float(os.getenv("MAX_SPREAD",          "0.12"))
MIN_LIQUIDITY_USDC  = float(os.getenv("MIN_LIQUIDITY_USDC",  "30"))
MIN_LLM_CONFIDENCE  = float(os.getenv("MIN_LLM_CONFIDENCE",  "0.65"))


class EdgeScorer:

    def score(self, market, orderbook, llm_result):
        ob      = self._parse_orderbook(orderbook)
        spread  = ob["spread"]
        mid     = ob["mid"]
        depth   = ob["total_depth_usdc"]
        best_ask = ob["best_ask"]
        best_bid = ob["best_bid"]

        if depth < MIN_LIQUIDITY_USDC:
            return self._reject(f"depth ${depth:.0f} < min ${MIN_LIQUIDITY_USDC}")

        if spread > MAX_SPREAD:
            return self._reject(f"spread {spread:.3f} > max {MAX_SPREAD}")

        confidence = llm_result.get("confidence", 0)
        if confidence < MIN_LLM_CONFIDENCE:
            return self._reject(f"LLM confidence {confidence:.2f} < {MIN_LLM_CONFIDENCE}")

        ai_prob   = llm_result.get("probability", 0.5)
        edge      = abs(ai_prob - mid)
        trade_side = "YES" if ai_prob > mid else "NO"

        if edge < MIN_EDGE:
            return self._reject(f"edge {edge:.3f} < min {MIN_EDGE}")

        spread_score    = self._spread_score(spread)
        thinness_score  = self._thinness_score(depth)
        time_score      = self._time_score(market.get("endDate", ""))
        momentum_score  = self._momentum_score(
            market.get("volume24hr", 0),
            market.get("volume",     0),
        )
        composite = (
            0.35 * min(edge / 0.15, 1.0) +
            0.20 * confidence             +
            0.20 * thinness_score         +
            0.15 * spread_score           +
            0.10 * time_score
        )

        if trade_side == "YES":
            entry_price = round(min(best_ask, round(ai_prob - 0.01, 2)), 2)
            entry_price = max(0.01, min(entry_price, 0.99))
        else:
            entry_price = round(max(best_bid, round((1 - ai_prob) - 0.01, 2)), 2)
            entry_price = max(0.01, min(entry_price, 0.99))

        return {
            "should_trade":   True,
            "trade_side":     trade_side,
            "entry_price":    entry_price,
            "edge":           round(edge, 4),
            "mid_price":      round(mid, 4),
            "ai_probability": round(ai_prob, 4),
            "composite_score": round(composite, 4),
            "confidence":     round(confidence, 4),
            "model_used":     llm_result.get("model_used", "unknown"),
            "reasoning":      llm_result.get("reasoning", ""),
            "breakdown": {
                "spread":       round(spread, 4),
                "depth_usdc":   round(depth, 2),
                "spread_score": round(spread_score, 4),
                "thinness":     round(thinness_score, 4),
                "time_score":   round(time_score, 4),
                "momentum":     round(momentum_score, 4),
            },
            "orderbook": {
                "best_bid":     round(best_bid, 4),
                "best_ask":     round(best_ask, 4),
            },
        }

    def _spread_score(self, spread):
        return min(spread / MAX_SPREAD, 1.0)

    def _thinness_score(self, depth_usdc):
        return 1.0 / (1.0 + math.log(max(depth_usdc, 1) / 50))

    def _time_score(self, end_date_str):
        from datetime import datetime, timezone
        if not end_date_str:
            return 0.3
        try:
            if isinstance(end_date_str, (int, float)):
                end = datetime.fromtimestamp(end_date_str, tz=timezone.utc)
            else:
                end = datetime.fromisoformat(end_date_str.replace("Z", "+00:00"))
            days_left = (end - datetime.now(tz=timezone.utc)).days
            if days_left < 0:
                return 0.0
            elif days_left < 1:
                return 0.5
            elif days_left <= 7:
                return 1.0
            elif days_left <= 14:
                return 0.7
            elif days_left <= 30:
                return 0.4
            else:
                return 0.1
        except Exception:
            return 0.3

    def _momentum_score(self, vol_24h, vol_total):
        if not vol_total or vol_total <= 0:
            return 0.3
        daily_share = vol_24h / vol_total
        if daily_share > 0.20:
            return 1.0
        elif daily_share > 0.10:
            return 0.7
        elif daily_share > 0.05:
            return 0.4
        else:
            return 0.2

    def _parse_orderbook(self, book):
        bids = sorted(book.get("bids", []), key=lambda x: float(x.get("price", 0)), reverse=True)
        asks = sorted(book.get("asks", []), key=lambda x: float(x.get("price", 1)))
        best_bid = float(bids[0]["price"]) if bids else 0.0
        best_ask = float(asks[0]["price"]) if asks else 1.0
        spread   = best_ask - best_bid
        mid      = (best_bid + best_ask) / 2 if (best_bid and best_ask) else 0.5
        depth = sum(
            float(x.get("price", 0)) * float(x.get("size", 0))
            for x in bids + asks
        )
        return {
            "best_bid": best_bid, "best_ask": best_ask,
            "spread": spread, "mid": mid, "total_depth_usdc": depth,
        }

    def _reject(self, reason):
        return {"should_trade": False, "reason": reason, "edge": 0.0, "composite_score": 0.0}
