"""
sniper.py — Polymarket Liquidity Sniper | Main Autonomous Loop
Strategy: AI-confidence based directional trading
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

MAX_TRADE_USDC   = float(os.getenv("MAX_TRADE_USDC",   "1"))
MAX_SESSION_USDC = float(os.getenv("MAX_SESSION_USDC", "150"))
MIN_CONFIDENCE   = float(os.getenv("MIN_LLM_CONFIDENCE", "0.70"))
MIN_PROBABILITY  = float(os.getenv("MIN_PROBABILITY", "0.65"))

BANNER = f"""
{Fore.CYAN}╔══════════════════════════════════════════════════════════════╗
║          POLYMARKET LIQUIDITY SNIPER  v1.0.0                 ║
║   Groq-primary · OpenRouter-fallback · 10-model chain        ║
╚══════════════════════════════════════════════════════════════╝{Style.RESET_ALL}
"""


class LiquiditySniper:
    def __init__(self, args):
        self.args            = args
        self.dry_run         = not args.live and os.getenv("DRY_RUN", "true").lower() != "false"
        self.poly            = PolyClient()
        self.router          = LLMRouter()
        self.session_spend   = 0.0
        self.session_trades  = 0
        self.session_scanned = 0
        self.trade_log       = Path("logs/trades.jsonl")

        if args.live and not self.dry_run:
            log.warning(f"{Fore.RED}LIVE MODE — Real USDC will be spent.{Style.RESET_ALL}")
        elif not args.live:
            self.dry_run = True

    def run(self):
        print(BANNER)
        mode_label = (
            f"{Fore.YELLOW}[DRY RUN]{Style.RESET_ALL}"
            if self.dry_run else
            f"{Fore.RED}[LIVE TRADING]{Style.RESET_ALL}"
        )
        print(f"  Mode        : {mode_label}")
        print(f"  Scan limit  : {self.args.scan_limit} markets/cycle")
        print(f"  Interval    : {self.args.interval}s between cycles")
        print(f"  Trade size  : ${MAX_TRADE_USDC} USDC")
        print(f"  Session cap : ${MAX_SESSION_USDC} USDC")
        print(f"  Min confidence: {MIN_CONFIDENCE}")
        print(f"  Min probability: {MIN_PROBABILITY}\n")

        cycle = 0
        while True:
            cycle += 1
            if self.args.cycles > 0 and cycle > self.args.cycles:
                log.info(f"Completed {self.args.cycles} cycles. Exiting.")
                break
            if self.session_spend >= MAX_SESSION_USDC:
                log.warning(f"{Fore.RED}Session cap reached. Stopping.{Style.RESET_ALL}")
                break

            print(f"\n{Fore.CYAN}── Cycle {cycle} | {datetime.now().strftime('%H:%M:%S')} ──{Style.RESET_ALL}")
            self._run_cycle()

            if self.args.cycles == 0 or cycle < self.args.cycles:
                log.info(f"Waiting {self.args.interval}s...")
                time.sleep(self.args.interval)

        self._print_session_summary()

    def _run_cycle(self):
        markets = self.poly.get_active_markets(
            limit=self.args.scan_limit,
            topic=self.args.topic
        )
        if not markets:
            log.warning("No markets returned.")
            return

        log.info(f"Scanning {len(markets)} markets...")
        opportunities = []

        for mkt in markets:
            self.session_scanned += 1
            result = self._evaluate_market(mkt)
            if result:
                opportunities.append(result)

        opportunities.sort(key=lambda x: x["confidence"], reverse=True)

        if not opportunities:
            print(f"  {Fore.WHITE}No opportunities this cycle.{Style.RESET_ALL}")
            return

        print(f"\n  {Fore.GREEN}{len(opportunities)} opportunity(ies) found:{Style.RESET_ALL}")
        for opp in opportunities[:3]:  # max 3 trades per cycle
            self._print_opportunity(opp)
            if self.session_spend + MAX_TRADE_USDC <= MAX_SESSION_USDC:
                self._execute_trade(opp)

    def _evaluate_market(self, mkt):
        try:
            question      = mkt.get("question", "")
            cond_id       = mkt.get("conditionId", mkt.get("id", ""))
            yes_id, no_id = self.poly.extract_token_ids(mkt)

            if not yes_id:
                return None

            # Get outcome prices safely
            prices = mkt.get("outcomePrices", ["0.5", "0.5"])
            if isinstance(prices, str):
                prices = json.loads(prices)
            yes_price = float(prices[0]) if prices else 0.5

            # Skip already-resolved markets
            if yes_price < 0.02 or yes_price > 0.98:
                return None

            llm_result = self.router.analyze_market({
                "question":    question,
                "yes_price":   yes_price,
                "no_price":    1 - yes_price,
                "volume_24h":  float(mkt.get("volume24hr", 0) or 0),
                "end_date":    mkt.get("endDate", ""),
                "description": mkt.get("description", ""),
            })

            prob       = llm_result.get("probability", 0.5)
            confidence = llm_result.get("confidence",  0.0)

            # Only trade when AI is confident AND strongly disagrees with 50/50
            if confidence < MIN_CONFIDENCE:
                return None
            if prob < MIN_PROBABILITY and prob > (1 - MIN_PROBABILITY):
                return None

            trade_side = "YES" if prob >= MIN_PROBABILITY else "NO"
            token_id   = yes_id if trade_side == "YES" else no_id

            return {
                "market":       mkt,
                "condition_id": cond_id,
                "token_id":     token_id,
                "trade_side":   trade_side,
                "trade_size":   MAX_TRADE_USDC,
                "probability":  prob,
                "confidence":   confidence,
                "yes_price":    yes_price,
                "model_used":   llm_result.get("model_used", ""),
                "reasoning":    llm_result.get("reasoning", ""),
                "question":     question,
            }

        except Exception as e:
            log.debug(f"Evaluation error: {e}")
            return None

    def _execute_trade(self, opp):
        side     = opp["trade_side"]
        token_id = opp["token_id"]
        neg_risk = self.poly.is_neg_risk(opp["market"])
        tag      = "[DRY RUN] " if self.dry_run else ""

        book = self.poly.get_orderbook(token_id)
        if not book:
            print(f"  {Fore.RED}No orderbook — skipping{Style.RESET_ALL}")
            return

        asks = sorted(book.get("asks", []), key=lambda x: float(x.get("price", 1)))
        bids = sorted(book.get("bids", []), key=lambda x: float(x.get("price", 0)), reverse=True)

        if not asks or not bids:
            print(f"  {Fore.YELLOW}Empty orderbook — skipping{Style.RESET_ALL}")
            return

        best_ask = float(asks[0]["price"])
        best_bid = float(bids[0]["price"])
        spread   = round(best_ask - best_bid, 4)

        if spread <= 0.001:
            print(f"  {Fore.YELLOW}Spread too tight ({spread:.4f}) — skipping{Style.RESET_ALL}")
            return

        print(f"  Orderbook: bid={best_bid:.4f} | ask={best_ask:.4f} | spread={spread:.4f}")

        buy_result = self.poly.place_limit_order(
            token_id=token_id, side="BUY", price=best_bid,
            size_usdc=MAX_TRADE_USDC, neg_risk=neg_risk, dry_run=self.dry_run,
        )
        print(f"  {tag}BUY  LIMIT {side} | {buy_result.get('shares',0):.2f} shares @ ${best_bid:.4f} | ${MAX_TRADE_USDC:.2f} USDC")

        if not buy_result.get("success"):
            print(f"  {Fore.RED}BUY failed: {buy_result.get('error','?')}{Style.RESET_ALL}")
            return

        shares_bought = buy_result.get("shares", 0)
        sell_size     = max(round(shares_bought * best_ask, 2), 1.0)

        sell_result = self.poly.place_limit_order(
            token_id=token_id, side="SELL", price=best_ask,
            size_usdc=sell_size, neg_risk=neg_risk, dry_run=self.dry_run,
        )
        print(f"  {tag}SELL LIMIT {side} | {sell_result.get('shares',0):.2f} shares @ ${best_ask:.4f} | ${sell_size:.2f} USDC")

        pnl = round(spread * shares_bought, 4)
        pnl_color = Fore.GREEN if pnl >= 0 else Fore.RED
        print(f"  PnL: {pnl_color}{pnl:+.4f} USDC{Style.RESET_ALL} (spread={spread:.4f})")

        if not self.dry_run:
            self.session_spend += max(MAX_TRADE_USDC - sell_size, 0)
        self.session_trades += 1
        self._log_trade(opp, buy_result, sell_result, pnl)

    def _print_opportunity(self, opp):
        print(f"\n  {Fore.GREEN}OPPORTUNITY{Style.RESET_ALL}")
        print(f"  Market    : {opp['question'][:70]}")
        print(f"  Side      : {opp['trade_side']} | AI prob: {opp['probability']:.3f} | "
              f"Market price: {opp['yes_price']:.3f}")
        print(f"  Confidence: {opp['confidence']:.2f} | Model: {opp['model_used']}")
        print(f"  Reason    : {opp['reasoning']}")
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
            "ts":            datetime.utcnow().isoformat(),
            "question":      opp["question"],
            "condition_id":  opp["condition_id"],
            "token_id":      opp["token_id"],
            "side":          opp["trade_side"],
            "buy_price":     buy.get("price", 0),
            "sell_price":    sell.get("price", 0) if sell else None,
            "shares":        buy.get("shares", 0),
            "size_usdc":     opp["trade_size"],
            "pnl_usdc":      round(pnl, 4),
            "probability":   opp["probability"],
            "confidence":    opp["confidence"],
            "model":         opp["model_used"],
            "buy_order_id":  buy.get("order_id", ""),
            "sell_order_id": sell.get("order_id", "") if sell else None,
            "simulated":     self.dry_run,
        }
        with self.trade_log.open("a") as f:
            f.write(json.dumps(entry) + "\n")


def parse_args():
    p = argparse.ArgumentParser(description="Polymarket Liquidity Sniper")
    mode = p.add_mutually_exclusive_group()
    mode.add_argument("--dry-run", action="store_true", default=True)
    mode.add_argument("--live",    action="store_true", default=False)
    p.add_argument("--cycles",     type=int,   default=0)
    p.add_argument("--scan-limit", type=int,   default=100)
    p.add_argument("--interval",   type=int,   default=45)
    p.add_argument("--topic",      type=str,   default=None)
    p.add_argument("--verbose",    action="store_true")
    return p.parse_args()


if __name__ == "__main__":
    args = parse_args()
    sniper = LiquiditySniper(args)
    try:
        sniper.run()
    except KeyboardInterrupt:
        print("\nInterrupted by user.")
        sniper._print_session_summary()
