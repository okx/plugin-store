"""
sniper.py — Polymarket Liquidity Sniper | Main Autonomous Loop
Usage:
  python scripts/sniper.py --dry-run
  python scripts/sniper.py --live
  python scripts/sniper.py --dry-run --topic crypto --verbose
  python scripts/sniper.py --dry-run --cycles 3 --scan-limit 50
"""

import argparse
import json
import logging
import os
import sys
import time
from datetime import datetime
from pathlib import Path

from dotenv import load_dotenv
from colorama import Fore, Style, init as colorama_init

sys.path.insert(0, str(Path(__file__).parent))
from llm_router  import LLMRouter
from edge_scorer import EdgeScorer
from poly_client import PolyClient

load_dotenv()
colorama_init(autoreset=True)

Path("logs").mkdir(exist_ok=True)
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[
        logging.StreamHandler(),
        logging.FileHandler("logs/sniper.log"),
    ],
)
log = logging.getLogger("sniper")

MAX_TRADE_USDC   = float(os.getenv("MAX_TRADE_USDC",   "20"))
MAX_SESSION_USDC = float(os.getenv("MAX_SESSION_USDC", "150"))
STOP_LOSS_PCT    = float(os.getenv("STOP_LOSS_PCT",    "0.25"))
MIN_EDGE         = float(os.getenv("MIN_EDGE",         "0.04"))

BANNER = f"""
{Fore.CYAN}╔══════════════════════════════════════════════════════════════╗
║          POLYMARKET LIQUIDITY SNIPER  v1.0.0                 ║
║   Groq-primary · OpenRouter-fallback · 10-model chain        ║
╚══════════════════════════════════════════════════════════════╝{Style.RESET_ALL}
"""


class LiquiditySniper:
    def __init__(self, args):
        self.args           = args
        self.dry_run        = args.dry_run or os.getenv("DRY_RUN", "true").lower() == "true"
        self.poly           = PolyClient()
        self.router         = LLMRouter()
        self.scorer         = EdgeScorer()
        self.session_spend  = 0.0
        self.session_trades = 0
        self.session_scanned = 0
        self.trade_log      = Path("logs/trades.jsonl")
        self.scan_log       = Path("logs/scans.jsonl")

        if args.live and not self.dry_run:
            log.warning(f"{Fore.RED}LIVE MODE ENABLED — Real USDC will be spent.{Style.RESET_ALL}")
        elif not args.live:
            self.dry_run = True

    def run(self):
        print(BANNER)
        mode_label = (
            f"{Fore.YELLOW}[DRY RUN]{Style.RESET_ALL}"
            if self.dry_run else
            f"{Fore.RED}[LIVE TRADING]{Style.RESET_ALL}"
        )
        print(f"  Mode      : {mode_label}")
        print(f"  Scan limit: {self.args.scan_limit} markets/cycle")
        print(f"  Interval  : {self.args.interval}s between cycles")
        print(f"  Max trade : ${MAX_TRADE_USDC} USDC")
        print(f"  Session cap: ${MAX_SESSION_USDC} USDC")
        print(f"  Min edge  : {MIN_EDGE*100:.1f}c\n")

        cycle = 0
        while True:
            cycle += 1
            if self.args.cycles > 0 and cycle > self.args.cycles:
                log.info(f"Completed {self.args.cycles} cycles. Exiting.")
                break
            if self.session_spend >= MAX_SESSION_USDC:
                log.warning(f"{Fore.RED}Session spend limit ${MAX_SESSION_USDC} reached. Stopping.{Style.RESET_ALL}")
                break

            print(f"\n{Fore.CYAN}── Cycle {cycle} | {datetime.now().strftime('%H:%M:%S')} ──{Style.RESET_ALL}")
            self._run_cycle()

            if self.args.cycles == 0 or cycle < self.args.cycles:
                log.info(f"Waiting {self.args.interval}s before next cycle...")
                time.sleep(self.args.interval)

        self._print_session_summary()

    def _run_cycle(self):
        markets = self.poly.get_active_markets(limit=self.args.scan_limit, topic=self.args.topic)
        if not markets:
            log.warning("No markets returned. Skipping cycle.")
            return

        log.info(f"Scanning {len(markets)} markets...")
        opportunities = []

        for mkt in markets:
            self.session_scanned += 1
            result = self._evaluate_market(mkt)
            if result and result.get("should_trade"):
                opportunities.append(result)

        opportunities.sort(key=lambda x: x["score"]["composite_score"], reverse=True)

        if not opportunities:
            print(f"  {Fore.WHITE}No opportunities this cycle.{Style.RESET_ALL}")
            return

        print(f"\n  {Fore.GREEN}{len(opportunities)} opportunity(ies) found:{Style.RESET_ALL}")
        for opp in opportunities:
            self._print_opportunity(opp)
            if self.session_spend + opp["trade_size"] <= MAX_SESSION_USDC:
                self._execute_trade(opp)
            else:
                print(f"  Skipped (session cap would be hit)")

    def _evaluate_market(self, mkt):
        try:
            question      = mkt.get("question", "")
            cond_id       = mkt.get("conditionId", mkt.get("id", ""))
            yes_id, no_id = self.poly.extract_token_ids(mkt)

            if not yes_id:
                return None

            book = self.poly.get_orderbook(yes_id)
            if not book:
                return None

            bids = book.get("bids", [])
            asks = book.get("asks", [])
            if not bids or not asks:
                return None

            best_bid = float(bids[0]["price"]) if bids else 0
            best_ask = float(asks[0]["price"]) if asks else 1
            spread   = best_ask - best_bid
            if spread > float(os.getenv("MAX_SPREAD", "0.12")):
                return None

            yes_price = mkt.get("outcomePrices", ["0.5", "0.5"])
            if isinstance(yes_price, list):
                yes_price = float(yes_price[0]) if yes_price else 0.5
            else:
                yes_price = 0.5

            llm_input = {
                "question":    question,
                "yes_price":   yes_price,
                "no_price":    1 - yes_price,
                "volume_24h":  mkt.get("volume24hr", 0),
                "end_date":    mkt.get("endDate", ""),
                "description": mkt.get("description", ""),
            }
            llm_result = self.router.analyze_market(llm_input)
            score      = self.scorer.score(mkt, book, llm_result)

            if not score.get("should_trade"):
                return None

            edge       = score["edge"]
            confidence = score["confidence"]
            raw_size   = MAX_TRADE_USDC * min(edge / 0.10, 1.0) * confidence
            trade_size = round(min(max(raw_size, 0.01), MAX_TRADE_USDC), 2)
            token_id   = yes_id if score["trade_side"] == "YES" else no_id

            self._log_scan(cond_id, question, score)

            return {
                "market":       mkt,
                "condition_id": cond_id,
                "token_id":     token_id,
                "trade_side":   score["trade_side"],
                "trade_size":   trade_size,
                "score":        score,
                "question":     question,
            }

        except Exception as e:
            log.debug(f"Evaluation error: {e}")
            return None

    def _execute_trade(self, opp):
        score      = opp["score"]
        trade_size = opp["trade_size"]
        side       = opp["trade_side"]
        token_id   = opp["token_id"]
        neg_risk   = self.poly.is_neg_risk(opp["market"])

        buy_result = self.poly.place_market_order(
            token_id=token_id, side="BUY", size_usdc=trade_size,
            neg_risk=neg_risk, dry_run=self.dry_run,
        )

        buy_tag = "[DRY RUN] " if self.dry_run else ""
        print(f"  BUY  {side} | {buy_result.get('shares', 0):.2f} shares @ "
              f"${buy_result.get('price', 0):.4f} | ${trade_size:.2f} USDC | {buy_tag}")

        if not buy_result["success"]:
            print(f"  BUY failed: {buy_result.get('error', '?')}")
            return

        shares_bought = buy_result.get("shares", 0)
        buy_price     = buy_result.get("price",  0)
        sell_size     = max(round(shares_bought * buy_price, 2), 1.00)

        sell_result = self.poly.place_market_order(
            token_id=token_id, side="SELL", size_usdc=sell_size,
            neg_risk=neg_risk, dry_run=self.dry_run,
        )

        print(f"  SELL {side} | {sell_result.get('shares', 0):.2f} shares @ "
              f"${sell_result.get('price', 0):.4f} | ${sell_size:.2f} USDC | {buy_tag}")

        buy_px  = buy_result.get("price",  0)
        sell_px = sell_result.get("price", 0)
        shares  = buy_result.get("shares", 0)
        pnl     = (sell_px - buy_px) * shares
        print(f"  Spread captured: {pnl:+.4f} USDC ({buy_px:.4f} -> {sell_px:.4f})")

        if buy_result["success"]:
            net_spend = trade_size - sell_size if sell_result["success"] else trade_size
            self.session_spend  += max(net_spend, 0) if not self.dry_run else 0
            self.session_trades += 1
            self._log_trade(opp, buy_result, sell_result, pnl)

    def _print_opportunity(self, opp):
        s = opp["score"]
        print(f"\n  EDGE FOUND")
        print(f"  Market    : {opp['question'][:72]}")
        print(f"  Side      : {s['trade_side']} @ {s['entry_price']:.4f} "
              f"(AI: {s['ai_probability']:.4f} mid: {s['mid_price']:.4f})")
        print(f"  Edge      : +{s['edge']*100:.1f}c | Spread: {s['breakdown']['spread']*100:.1f}c "
              f"| Depth: ${s['breakdown']['depth_usdc']:.0f}")
        print(f"  Score     : {s['composite_score']:.3f} | Confidence: {s['confidence']:.2f} "
              f"| Model: {s['model_used']}")
        print(f"  Reason    : {s['reasoning']}")
        print(f"  Trade size: ${opp['trade_size']:.2f} USDC")

    def _print_session_summary(self):
        print(f"\n{'='*64}")
        print(f"  SESSION SUMMARY")
        print(f"  Markets scanned : {self.session_scanned}")
        print(f"  Trades executed : {self.session_trades}")
        print(f"  USDC spent      : ${self.session_spend:.2f}")
        llm_status = self.router.get_status()
        print(f"  LLM API calls   : {llm_status['session_calls']}")
        print(f"  LLM tokens used : {llm_status['session_tokens']:,}")
        print(f"{'='*64}\n")

    def _log_trade(self, opp, buy, sell=None, pnl=0.0):
        entry = {
            "ts": datetime.utcnow().isoformat(), "question": opp["question"],
            "condition_id": opp["condition_id"], "token_id": opp["token_id"],
            "side": opp["trade_side"], "buy_price": buy.get("price", 0),
            "sell_price": sell.get("price", 0) if sell else None,
            "shares": buy.get("shares", 0), "size_usdc": opp["trade_size"],
            "pnl_usdc": round(pnl, 4), "edge": opp["score"]["edge"],
            "ai_prob": opp["score"]["ai_probability"], "model": opp["score"]["model_used"],
            "buy_order_id": buy.get("order_id", ""),
            "sell_order_id": sell.get("order_id", "") if sell else None,
            "sell_success": sell.get("success", False) if sell else False,
            "simulated": self.dry_run,
        }
        with self.trade_log.open("a") as f:
            f.write(json.dumps(entry) + "\n")

    def _log_scan(self, cond_id, question, score):
        entry = {
            "ts": datetime.utcnow().isoformat(), "cond": cond_id,
            "question": question[:80], "edge": score.get("edge", 0),
            "score": score.get("composite_score", 0), "model": score.get("model_used", ""),
        }
        with self.scan_log.open("a") as f:
            f.write(json.dumps(entry) + "\n")


def parse_args():
    p = argparse.ArgumentParser(description="Polymarket Liquidity Sniper")
    mode = p.add_mutually_exclusive_group()
    mode.add_argument("--dry-run", action="store_true", default=True)
    mode.add_argument("--live",    action="store_true", default=False)
    p.add_argument("--cycles",     type=int,   default=0)
    p.add_argument("--scan-limit", type=int,   default=100)
    p.add_argument("--interval",   type=int,   default=45)
    p.add_argument("--min-edge",   type=float, default=None)
    p.add_argument("--topic",      type=str,   default=None)
    p.add_argument("--verbose",    action="store_true")
    return p.parse_args()


if __name__ == "__main__":
    args = parse_args()
    if args.min_edge is not None:
        os.environ["MIN_EDGE"] = str(args.min_edge)
    sniper = LiquiditySniper(args)
    try:
        sniper.run()
    except KeyboardInterrupt:
        print("\nInterrupted by user.")
        sniper._print_session_summary()
