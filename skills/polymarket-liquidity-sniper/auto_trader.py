"""
auto_trader.py — Autonomous Polymarket trader via OKX Agentic Wallet
Refreshes credentials automatically and executes trades continuously
Strategy: BUY then immediately SELL to capture spread (scalping)
"""
import json, os, sys, time, subprocess, logging, argparse
from pathlib import Path
from datetime import datetime
from dotenv import load_dotenv

load_dotenv()
sys.path.insert(0, str(Path(__file__).parent))
from refresh_creds import refresh_credentials

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[
        logging.StreamHandler(),
        logging.FileHandler("logs/auto_trader.log"),
    ],
)
log = logging.getLogger("auto_trader")

PATH = os.environ.get("PATH", "") + ":/home/" + os.environ.get("USER", "mestarkris") + "/.local/bin"

MAX_TRADE_USDC  = float(os.getenv("MAX_TRADE_USDC", "2"))
TRADE_INTERVAL  = int(os.getenv("TRADE_INTERVAL", "120"))
MAX_TRADES      = int(os.getenv("MAX_TRADES", "0"))  # 0 = unlimited

TRADED_SLUGS = set()  # avoid repeating same market in a session

SKIP_KEYWORDS = [
    "lol-", "csgo-", "cs2-", "nba-", "nfl-", "mlb-", "nhl-",
    "-vs-", "match", "game-", "series-", "cup-",
    "election", "president", "governor", "senate", "vote",
    "ipl-", "cricket", "football", "soccer", "tennis",
    "atp-", "wta-", "epl-", "tur-", "sea-",
]


def run_plugin(args, dry_run=False):
    """Run polymarket-plugin command, return parsed JSON."""
    if dry_run:
        log.info(f"[DRY RUN] Would run: polymarket-plugin {' '.join(args)}")
        # Return simulated success response
        if args[0] == "buy":
            return {"ok": True, "data": {"order_id": "DRY-RUN-BUY", "shares": MAX_TRADE_USDC / 0.5}}
        elif args[0] == "sell":
            return {"ok": True, "data": {"order_id": "DRY-RUN-SELL"}}
        return {"ok": True, "data": {}}

    cmd = ["polymarket-plugin"] + args
    result = subprocess.run(
        cmd, capture_output=True, text=True, timeout=60,
        env={**os.environ, "PATH": PATH}
    )
    try:
        return json.loads(result.stdout)
    except Exception:
        return {"ok": False, "error": result.stdout + result.stderr}


def get_markets(limit=50, dry_run=False):
    """Fetch 5-minute BTC markets for high transaction count."""
    if dry_run:
        return [{"slug": "btc-updown-5m-test", "yes_price": 0.505, "no_price": 0.495}]

    markets = []
    for coin in ["BTC", "ETH", "SOL"]:
        result = subprocess.run(
            ["polymarket-plugin", "list-5m", "--coin", coin],
            capture_output=True, text=True, timeout=30,
            env={**os.environ, "PATH": PATH}
        )
        try:
            data = json.loads(result.stdout)
            for m in data.get("data", {}).get("markets", []):
                markets.append({
                    "slug": m["conditionId"],  # use conditionId as market-id
                    "yes_price": m["upPrice"],
                    "no_price": m["downPrice"],
                    "outcome_yes": "up",
                    "outcome_no": "down",
                })
        except Exception:
            continue
    return markets


from datetime import datetime, timezone

def pick_trade(markets):
    now = datetime.now(timezone.utc)
    for m in markets:
        slug = m.get("slug", "")

        if slug in TRADED_SLUGS:
            continue

        # Skip markets closing within 3 minutes
        end_date = m.get("end_date") or m.get("endDateUtc", "")
        if end_date:
            try:
                end_dt = datetime.fromisoformat(end_date.replace("Z", "+00:00"))
                seconds_left = (end_dt - now).total_seconds()
                if seconds_left < 180:  # skip if less than 3 minutes left
                    log.info(f"Skipping {slug} — closes in {seconds_left:.0f}s")
                    continue
            except Exception:
                pass

        try:
            yes_price = float(m.get("yes_price", 0))
            no_price  = float(m.get("no_price", 0))
        except (ValueError, TypeError):
            continue

        if yes_price <= 0 or no_price <= 0:
            continue

        if 0.10 <= yes_price <= 0.90:
            if no_price <= yes_price:
                return slug, m.get("outcome_no", "No"), no_price
            else:
                return slug, m.get("outcome_yes", "Yes"), yes_price

    return None, None, None


def execute_trade(slug, outcome, price, dry_run=False):
    """BUY then immediately SELL — scalping strategy."""
    tag = "[DRY RUN] " if dry_run else ""

    # ── BUY ──────────────────────────────────────────────────────────────────
    buy_result = run_plugin([
        "buy",
        "--market-id", slug,
        "--outcome", outcome.lower(),
        "--amount", str(MAX_TRADE_USDC),
    ], dry_run=dry_run)

    if not buy_result.get("ok"):
        err = str(buy_result.get("error", "?"))
        if "STALE_CREDENTIALS" in err:
            return "stale_creds"
        log.warning(f"{tag}BUY failed: {err[:120]}")
        return "buy_failed"

    shares = buy_result.get("data", {}).get("shares", 0)
    order_id = buy_result.get("data", {}).get("order_id", "?")
    log.info(f"{tag}✅ BUY filled | {outcome} {shares:.2f} shares @ {price:.3f} | order={str(order_id)[:16]}...")

    # Mark market as traded to avoid repetition
    TRADED_SLUGS.add(slug)

    # ── Wait briefly then SELL ────────────────────────────────────────────────
    if not dry_run:
        time.sleep(5)

    # Refresh credentials before sell to avoid stale key errors
    if not dry_run:
        refresh_credentials()

    sell_result = run_plugin([
        "sell",
        "--market-id", slug,
        "--outcome", outcome.lower(),
        "--shares", str(round(shares, 4)),
        "--order-type", "GTC",
    ], dry_run=dry_run)

    if not sell_result.get("ok"):
        err = str(sell_result.get("error", "?"))
        log.warning(f"{tag}SELL failed: {err[:120]}")
        log.warning(f"  → Manual sell: polymarket-plugin sell --market-id {slug} --outcome {outcome.lower()} --shares {shares:.4f} --order-type FOK")
        return "sell_failed"

    sell_order = sell_result.get("data", {}).get("order_id", "?")
    log.info(f"{tag}✅ SELL filled | order={str(sell_order)[:16]}...")
    return "success"


def run(dry_run=False):
    mode = "[DRY RUN] " if dry_run else "[LIVE] "
    log.info(f"🚀 Auto Trader starting... {mode}")
    log.info(f"   Trade size: ${MAX_TRADE_USDC} USDC | Interval: {TRADE_INTERVAL}s")

    trades = 0
    cred_refresh_interval = 240  # refresh every 4 minutes
    last_cred_refresh = 0

    while True:
        try:
            # ── Refresh credentials ───────────────────────────────────────────
            if not dry_run and time.time() - last_cred_refresh > cred_refresh_interval:
                log.info("Refreshing credentials...")
                if refresh_credentials():
                    log.info("✅ Credentials refreshed")
                    last_cred_refresh = time.time()
                else:
                    log.error("❌ Failed to refresh credentials — retrying in 30s")
                    time.sleep(30)
                    continue

            # ── Fetch markets ─────────────────────────────────────────────────
            markets = get_markets(50, dry_run=dry_run)
            if not markets:
                log.warning("No markets found — retrying in 30s")
                time.sleep(30)
                continue

            # ── Pick and execute trade ────────────────────────────────────────
            slug, outcome, price = pick_trade(markets)
            if not slug:
                log.info("No suitable market found this cycle — all rotated or filtered")
                # Reset traded slugs after full rotation
                if len(TRADED_SLUGS) > 20:
                    TRADED_SLUGS.clear()
                    log.info("Market rotation reset — rescanning all markets")
                time.sleep(TRADE_INTERVAL)
                continue

            log.info(f"Trading: BUY {outcome.upper()} on {slug} @ {price:.3f}")
            result = execute_trade(slug, outcome, price, dry_run=dry_run)

            if result == "stale_creds":
                log.warning("Stale credentials — refreshing now")
                last_cred_refresh = 0
                continue
            elif result == "success":
                trades += 1
                log.info(f"✅ Trade complete | total trades: {trades}")
            elif result == "sell_failed":
                trades += 1  # buy succeeded even if sell failed

            # ── Check trade limit ─────────────────────────────────────────────
            if MAX_TRADES > 0 and trades >= MAX_TRADES:
                log.info(f"Reached max trades ({MAX_TRADES}). Stopping.")
                break

            log.info(f"Waiting {TRADE_INTERVAL}s...")
            time.sleep(TRADE_INTERVAL)

        except KeyboardInterrupt:
            log.info(f"Stopped by user. Total trades: {trades}")
            break
        except Exception as e:
            log.error(f"Unexpected error: {e}")
            time.sleep(30)

    log.info(f"Session complete. Total trades: {trades}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Polymarket Auto Trader")
    parser.add_argument("--dry-run", action="store_true", default=False,
                        help="Simulate trades without executing real orders")
    args = parser.parse_args()

    # Also respect DRY_RUN env var
    dry_run = args.dry_run or os.getenv("DRY_RUN", "false").lower() == "true"
    run(dry_run=dry_run)
