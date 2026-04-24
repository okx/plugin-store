#!/usr/bin/env python3
"""
Polymarket Spread Arb v5 — SINGLE-SLOT EVENT-DRIVEN

One slot = one trading session for the full market lifetime.
Two WebSocket streams:
  1. Market WS — real-time order book (bids/asks depth, tick_size_change)
  2. User WS   — real-time fills (MATCHED/CONFIRMED/FAILED)

Logic:
  - Wait for both sides of the book to populate before trading
  - Check depth: only bid if there's enough liquidity to fill
  - Stop when any side reaches 96¢ (tick changes from 1¢ to 0.1¢)
  - Position tracked from real User WS fills only
  - Corridor math + would_improve_pair on real data
  - Cancel previous orders before each new pair (only 1 pair live at a time)

OKX Challenge compliance:
  - Default execution path is polymarket-plugin buy/cancel with --strategy-id
"""

import argparse, asyncio, datetime, json, os, sys, time
from urllib.request import urlopen, Request
from urllib.error import URLError, HTTPError

try:
    import websockets
except ImportError:
    sys.exit("pip3 install websockets")

GAMMA = "https://gamma-api.polymarket.com"
CLOB = "https://clob.polymarket.com"
WS_MARKET = "wss://ws-subscriptions-clob.polymarket.com/ws/market"
WS_USER = "wss://ws-subscriptions-clob.polymarket.com/ws/user"
CREDS_PATH = os.path.expanduser("~/.config/polymarket/creds.json")
STRATEGY_ID = "polymarket-spread-arb"
ORDER_SHARES = 5.0
PLUGIN_BIN = "polymarket-plugin"

COINS = ["btc", "eth", "sol", "xrp", "bnb", "doge", "hype"]
COIN_LONG = {"btc": "bitcoin", "eth": "ethereum", "sol": "solana", "xrp": "xrp",
             "bnb": "bnb", "doge": "dogecoin", "hype": "hype"}
MONTHS = {1: "january", 2: "february", 3: "march", 4: "april", 5: "may", 6: "june",
          7: "july", 8: "august", 9: "september", 10: "october", 11: "november", 12: "december"}
TF_STEP = {"5m": 300, "15m": 900}

# Stop trading when any side hits this price (tick changes 1¢ → 0.1¢)
TICK_CHANGE_THRESHOLD = 0.96


def sf(v, d=0.0):
    try: return float(v)
    except: return d

def log(msg):
    t = datetime.datetime.utcnow().strftime("%H:%M:%S.%f")[:-3]
    sys.stderr.write(f"[{t}] {msg}\n"); sys.stderr.flush()

def api_sync(url):
    try:
        r = Request(url, headers={"Accept": "application/json", "User-Agent": "spread-arb/5.0"})
        with urlopen(r, timeout=5) as resp:
            return json.loads(resp.read().decode())
    except:
        return None

def load_polymarket_creds():
    with open(CREDS_PATH) as f:
        return json.load(f)

def fmt_amount(value):
    return f"{value:.6f}".rstrip("0").rstrip(".")

def find_order_id(value):
    if isinstance(value, dict):
        for key in ("orderID", "order_id", "id"):
            if value.get(key):
                return str(value[key])
        for child in value.values():
            found = find_order_id(child)
            if found:
                return found
    elif isinstance(value, list):
        for child in value:
            found = find_order_id(child)
            if found:
                return found
    return ""

async def run_cmd(args, timeout=60):
    proc = await asyncio.create_subprocess_exec(
        *args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    try:
        stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=timeout)
    except asyncio.TimeoutError:
        proc.kill()
        await proc.wait()
        return {"ok": False, "error": "command timed out", "stdout": "", "stderr": "", "returncode": -1}

    out = stdout.decode(errors="replace").strip()
    err = stderr.decode(errors="replace").strip()
    data = None
    if out:
        try:
            data = json.loads(out)
        except json.JSONDecodeError:
            data = None
    ok = proc.returncode == 0 and not (isinstance(data, dict) and data.get("ok") is False)
    return {
        "ok": ok,
        "data": data,
        "stdout": out,
        "stderr": err,
        "returncode": proc.returncode,
        "error": (data.get("error") if isinstance(data, dict) else None) or err or out,
    }

class PluginExecutor:
    """Write operations through polymarket-plugin for OKX strategy attribution."""

    def __init__(self, strategy_id):
        self.strategy_id = strategy_id

    async def balance(self):
        return await run_cmd([PLUGIN_BIN, "balance"], timeout=30)

    async def cancel_market(self, condition_id):
        return await run_cmd([PLUGIN_BIN, "cancel", "--market", condition_id], timeout=45)

    async def buy(self, token_id, outcome, price, shares, dry_run=False):
        amount = max(price * shares, 0.01)
        cmd = [
            PLUGIN_BIN, "buy",
            "--token-id", str(token_id),
            "--outcome", outcome,
            "--amount", fmt_amount(amount),
            "--price", fmt_amount(price),
            "--order-type", "GTC",
            "--post-only",
            "--round-up",
            "--strategy-id", self.strategy_id,
        ]
        if dry_run:
            cmd.append("--dry-run")
        return await run_cmd(cmd, timeout=75)


# ── SLUGS ──────────────────────────────────────────────────────

def gen_slugs(coin, tf, count=5):
    if tf in TF_STEP:
        step = TF_STEP[tf]; now = int(time.time())
        cur = (now // step) * step  # current slot (floor)
        return [f"{coin}-updown-{tf}-{cur + i * step}" for i in range(count)]
    cl = COIN_LONG[coin]; utc = datetime.datetime.utcnow()
    et = utc - datetime.timedelta(hours=4)
    cur = et.replace(minute=0, second=0, microsecond=0)
    if et.minute > 0 or et.second > 0:
        cur += datetime.timedelta(hours=1)
    out = []
    for i in range(count):
        t = cur + datetime.timedelta(hours=i)
        h = t.hour; ap = "am" if h < 12 else "pm"
        h12 = h if h <= 12 else h - 12
        if h12 == 0: h12 = 12
        out.append(f"{cl}-up-or-down-{MONTHS[t.month]}-{t.day}-{t.year}-{h12}{ap}-et")
    return out

def resolve_market(slug):
    d = api_sync(f"{GAMMA}/markets?slug={slug}")
    if not isinstance(d, list) or not d:
        return None
    m = d[0]; raw = m.get("clobTokenIds", "[]")
    ids = json.loads(raw) if isinstance(raw, str) else raw
    if not isinstance(ids, list) or len(ids) < 2:
        return None
    return {
        "slug": slug, "condition_id": m.get("conditionId", ""),
        "up_token": ids[0], "dn_token": ids[1],
        "end_date": m.get("endDate", ""),
        "question": m.get("question", "")[:120],
        "accepting": m.get("acceptingOrders", True),
    }

def resolve_batch(coin, tf, count=5):
    out = []
    for s in gen_slugs(coin, tf, count):
        m = resolve_market(s)
        if m and m["accepting"]:
            out.append(m)
    return out


# ── LIVE BOOK (Market WebSocket) ───────────────────────────────

class LiveBook:
    """Real-time order book with depth tracking."""

    def __init__(self):
        self.up_bid = self.up_ask = self.dn_bid = self.dn_ask = 0.0
        # Depth: total size available at best bid/ask
        self.up_bid_depth = self.dn_bid_depth = 0.0
        self.up_ask_depth = self.dn_ask_depth = 0.0
        self.up_token = self.dn_token = ""
        self._ws = None; self._connected = False
        self.changed = asyncio.Event()
        self.tick_changed = False  # set when tick_size_change event fires

    @property
    def gap_cents(self):
        return round((1.0 - self.up_bid - self.dn_bid) * 100, 2)

    @property
    def both_sides_live(self):
        """Both sides have bids and asks — book is populated."""
        return self.up_bid > 0 and self.up_ask > 0 and self.dn_bid > 0 and self.dn_ask > 0

    @property
    def any_side_extreme(self):
        """Any side bid >= 96¢ or <= 4¢ — tick is about to change."""
        return (self.up_bid >= TICK_CHANGE_THRESHOLD or self.dn_bid >= TICK_CHANGE_THRESHOLD or
                self.up_bid <= (1 - TICK_CHANGE_THRESHOLD) or self.dn_bid <= (1 - TICK_CHANGE_THRESHOLD))

    async def connect(self, up_tok, dn_tok):
        self.up_token, self.dn_token = up_tok, dn_tok
        self.up_bid = self.up_ask = self.dn_bid = self.dn_ask = 0.0
        self.up_bid_depth = self.dn_bid_depth = 0.0
        self.up_ask_depth = self.dn_ask_depth = 0.0
        self._connected = False; self.tick_changed = False
        try:
            self._ws = await websockets.connect(WS_MARKET, ping_interval=None)
            await self._ws.send(json.dumps({
                "assets_ids": [up_tok, dn_tok], "type": "market",
                "custom_feature_enabled": True,
            }))
            self._connected = True
            log("  Market WS connected")
        except Exception as e:
            log(f"  Market WS fail: {e}")

    async def disconnect(self):
        if self._ws:
            try: await self._ws.close()
            except: pass
        self._connected = False

    async def read_loop(self):
        if not self._ws: return
        try:
            async for raw in self._ws:
                if raw == "PONG": continue
                try: msgs = json.loads(raw)
                except: continue
                if not isinstance(msgs, list): msgs = [msgs]
                for m in msgs: self._proc(m)
        except Exception as e:
            log(f"  Market WS disconnected: {e}")
            self._connected = False

    def _proc(self, msg):
        ev = msg.get("event_type", ""); asset = msg.get("asset_id", "")
        prev = (self.up_bid, self.up_ask, self.dn_bid, self.dn_ask)

        if ev == "book":
            bids, asks = msg.get("bids", []), msg.get("asks", [])
            if bids:
                bb = max(bids, key=lambda b: sf(b.get("price")))
                bb_price = sf(bb.get("price"))
                bb_depth = sf(bb.get("size"))
            else:
                bb_price = bb_depth = 0.0
            if asks:
                ba = min(asks, key=lambda a: sf(a.get("price")))
                ba_price = sf(ba.get("price"))
                ba_depth = sf(ba.get("size"))
            else:
                ba_price = ba_depth = 0.0
            if asset == self.up_token:
                self.up_bid, self.up_ask = bb_price, ba_price
                self.up_bid_depth, self.up_ask_depth = bb_depth, ba_depth
            elif asset == self.dn_token:
                self.dn_bid, self.dn_ask = bb_price, ba_price
                self.dn_bid_depth, self.dn_ask_depth = bb_depth, ba_depth

        elif ev == "price_change":
            for ch in msg.get("price_changes", [msg]):
                bb, ba = sf(ch.get("best_bid")), sf(ch.get("best_ask"))
                if asset == self.up_token:
                    if bb > 0: self.up_bid = bb
                    if ba > 0: self.up_ask = ba
                elif asset == self.dn_token:
                    if bb > 0: self.dn_bid = bb
                    if ba > 0: self.dn_ask = ba

        elif ev == "best_bid_ask":
            bb, ba = sf(msg.get("best_bid")), sf(msg.get("best_ask"))
            if asset == self.up_token:
                if bb > 0: self.up_bid = bb
                if ba > 0: self.up_ask = ba
            elif asset == self.dn_token:
                if bb > 0: self.dn_bid = bb
                if ba > 0: self.dn_ask = ba

        elif ev == "tick_size_change":
            self.tick_changed = True
            log(f"  ⚠ TICK SIZE CHANGED: {msg.get('old_tick_size')} → {msg.get('new_tick_size')}")
            self.changed.set()
            return

        else:
            return

        if (self.up_bid, self.up_ask, self.dn_bid, self.dn_ask) != prev:
            self.changed.set()

    async def ping_loop(self):
        while self._connected and self._ws:
            try: await self._ws.send("PING"); await asyncio.sleep(8)
            except: break


# ── USER STREAM (User WebSocket) ──────────────────────────────

class UserStream:
    """Real-time fills and order updates from User WS channel."""

    def __init__(self, api_key, secret, passphrase):
        self.api_key = api_key
        self.secret = secret
        self.passphrase = passphrase
        self.up_token = self.dn_token = ""
        self._ws = None; self._connected = False

        # Position — updated ONLY from real fill events
        self.f_up = self.f_dn = self.c_up = self.c_dn = 0.0
        self.open_orders = {}
        self.fill_event = asyncio.Event()
        self.seen_trade_ids = set()
        self.matched_trades = {}  # for FAILED rollback

    @property
    def avg_up(self): return self.c_up / self.f_up if self.f_up > 0 else 0.0
    @property
    def avg_dn(self): return self.c_dn / self.f_dn if self.f_dn > 0 else 0.0
    @property
    def pair_cost(self): return self.avg_up + self.avg_dn
    @property
    def filled_usd(self): return (self.f_up * self.avg_up + self.f_dn * self.avg_dn) / 100

    async def connect(self, condition_id):
        self._connected = False
        try:
            self._ws = await websockets.connect(WS_USER, ping_interval=None)
            await self._ws.send(json.dumps({
                "auth": {"apiKey": self.api_key, "secret": self.secret, "passphrase": self.passphrase},
                "markets": [condition_id], "type": "user",
            }))
            self._connected = True
            log("  User WS connected")
        except Exception as e:
            log(f"  User WS fail: {e}")

    async def disconnect(self):
        if self._ws:
            try: await self._ws.close()
            except: pass
        self._connected = False

    async def read_loop(self):
        if not self._ws: return
        try:
            async for raw in self._ws:
                if raw == "PONG": continue
                try: msgs = json.loads(raw)
                except: continue
                if not isinstance(msgs, list): msgs = [msgs]
                for m in msgs: self._proc(m)
        except Exception as e:
            log(f"  User WS disconnected: {e}")
            self._connected = False

    def _proc(self, msg):
        ev = msg.get("event_type", "").lower()
        if not ev:
            t = msg.get("type", "").upper()
            if t in ("TRADE", "MATCHED", "MINED", "CONFIRMED", "RETRYING", "FAILED"):
                ev = "trade"
            elif t in ("PLACEMENT", "UPDATE", "CANCELLATION"):
                ev = "order"
        if ev == "trade": self._on_trade(msg)
        elif ev == "order": self._on_order(msg)

    def _on_trade(self, msg):
        trade_id = msg.get("id", "")
        status = msg.get("status", "").upper() or msg.get("type", "").upper()

        if status == "FAILED":
            if trade_id in self.matched_trades:
                t = self.matched_trades.pop(trade_id)
                self.seen_trade_ids.discard(trade_id)
                if t["asset_id"] == self.up_token:
                    self.f_up -= t["size"]; self.c_up -= t["size"] * t["price_c"]
                    log(f"  ⚠ ROLLBACK UP: {t['size']} shares (FAILED)")
                elif t["asset_id"] == self.dn_token:
                    self.f_dn -= t["size"]; self.c_dn -= t["size"] * t["price_c"]
                    log(f"  ⚠ ROLLBACK DN: {t['size']} shares (FAILED)")
            return

        if status not in ("MATCHED", "CONFIRMED"): return
        if status == "CONFIRMED" and trade_id in self.seen_trade_ids:
            self.matched_trades.pop(trade_id, None)
            return
        if trade_id in self.seen_trade_ids: return
        self.seen_trade_ids.add(trade_id)

        asset_id = str(msg.get("asset_id", ""))
        side = msg.get("side", "").upper()
        price = sf(msg.get("price")); size = sf(msg.get("size"))
        if size <= 0 or price <= 0: return
        price_c = price * 100

        if side == "BUY":
            if asset_id == self.up_token:
                self.c_up += size * price_c; self.f_up += size
                self.matched_trades[trade_id] = {"asset_id": asset_id, "side": side, "price_c": price_c, "size": size}
                log(f"  ✓ FILL UP: {size}sh @ {price:.2f} | total {self.f_up:.0f}sh avg {self.avg_up:.1f}¢")
            elif asset_id == self.dn_token:
                self.c_dn += size * price_c; self.f_dn += size
                self.matched_trades[trade_id] = {"asset_id": asset_id, "side": side, "price_c": price_c, "size": size}
                log(f"  ✓ FILL DN: {size}sh @ {price:.2f} | total {self.f_dn:.0f}sh avg {self.avg_dn:.1f}¢")

        if status == "CONFIRMED":
            self.matched_trades.pop(trade_id, None)
        self.fill_event.set()

    def _on_order(self, msg):
        oid = msg.get("id", ""); otype = msg.get("type", "").upper()
        if otype == "PLACEMENT":
            self.open_orders[oid] = {"asset_id": str(msg.get("asset_id", "")),
                                     "price": msg.get("price", "0"), "original_size": msg.get("original_size", "0"),
                                     "size_matched": msg.get("size_matched", "0")}
        elif otype == "UPDATE" and oid in self.open_orders:
            self.open_orders[oid]["size_matched"] = msg.get("size_matched", "0")
        elif otype == "CANCELLATION":
            self.open_orders.pop(oid, None)

    async def ping_loop(self):
        while self._connected and self._ws:
            try: await self._ws.send("PING"); await asyncio.sleep(8)
            except: break

    def reset(self):
        self.f_up = self.f_dn = self.c_up = self.c_dn = 0.0
        self.open_orders.clear(); self.seen_trade_ids.clear()
        self.matched_trades.clear(); self.fill_event.clear()

# ── MATH ───────────────────────────────────────────────────────

def corridor(a_up, a_dn):
    if a_up <= 0 or a_dn <= 0 or a_up >= 100 or a_dn >= 100: return None, None
    lo = a_dn / (100.0 - a_up); hi = (100.0 - a_dn) / a_up
    return (round(lo, 4), round(hi, 4)) if lo < hi else (None, None)

def calc_pnl(a_up, a_dn, f_up, f_dn):
    c = f_up * a_up + f_dn * a_dn
    return round((100 * f_up - c) / 100, 4), round((100 * f_dn - c) / 100, 4)

def would_improve_pair(a_up, a_dn, f_up, f_dn, new_price_c, side, new_shares):
    if f_up == 0 and f_dn == 0: return True
    na_up, na_dn = a_up, a_dn
    if side == "up":
        if f_up == 0: return True
        na_up = (f_up * a_up + new_shares * new_price_c) / (f_up + new_shares)
    else:
        if f_dn == 0: return True
        na_dn = (f_dn * a_dn + new_shares * new_price_c) / (f_dn + new_shares)
    new_pair = na_up + na_dn
    old_pair = a_up + a_dn if (f_up > 0 and f_dn > 0) else 999
    return new_pair < 100 and new_pair <= old_pair + 0.5


# ── MAIN ───────────────────────────────────────────────────────

async def run(args):
    coin = args.coin.lower(); tf = args.tf
    budget = args.budget; dry_run = args.dry_run
    min_gap = args.min_gap; min_depth = args.min_depth
    slots = args.slots
    strategy_id = args.strategy_id

    if coin not in COINS:
        print(json.dumps({"error": f"Unknown coin. Use: {','.join(COINS)}"})); return

    log(f"=== SPREAD ARB v5: {coin.upper()} {tf} | ${budget} budget | gap≥{min_gap}¢ | depth≥{min_depth}sh | {slots} slots | plugin execution ===")
    if dry_run: log("*** DRY RUN ***")

    executor = PluginExecutor(strategy_id)
    creds = load_polymarket_creds()
    log(f"  Execution: {PLUGIN_BIN} with --strategy-id {strategy_id}")
    if not dry_run:
        bal = await executor.balance()
        if not bal["ok"]:
            log(f"  ⚠ balance check via {PLUGIN_BIN} failed: {bal['error'][:120]}")

    # Resolve markets
    log("Resolving markets...")
    markets = resolve_batch(coin, tf, count=slots)
    if not markets:
        print(json.dumps({"error": "No markets found"})); return
    log(f"Found {len(markets)} markets")

    book = LiveBook()
    user = UserStream(creds["api_key"], creds["secret"], creds["passphrase"])
    user_connected = False
    user_ws_task = user_ping_task = None
    all_results = []

    for mkt in markets:
        slug = mkt["slug"]; condition_id = mkt["condition_id"]
        try: end_ts = datetime.datetime.fromisoformat(mkt["end_date"].replace("Z", "+00:00")).timestamp()
        except: end_ts = time.time() + 3600

        remaining = end_ts - time.time()
        if remaining < 30:
            log(f"Skip {slug} — {int(remaining)}s left"); continue

        log(f"\n{'='*60}")
        log(f"SLOT: {slug}")
        log(f"  {mkt['question']} | {int(remaining)}s left")

        # Reset state
        user.up_token = str(mkt["up_token"]); user.dn_token = str(mkt["dn_token"])
        user.reset()

        # Cancel old orders for this market
        if not dry_run:
            r = await executor.cancel_market(condition_id)
            if not r["ok"]:
                log(f"  ⚠ cancel via {PLUGIN_BIN} failed: {r['error'][:120]}")

        # Connect Market WS
        await book.connect(mkt["up_token"], mkt["dn_token"])
        ws_task = asyncio.create_task(book.read_loop()) if book._connected else None
        ping_task = asyncio.create_task(book.ping_loop()) if book._connected else None

        # Connect User WS
        if not user_connected:
            await user.connect(condition_id)
            if user._connected:
                user_ws_task = asyncio.create_task(user.read_loop())
                user_ping_task = asyncio.create_task(user.ping_loop())
                user_connected = True
        else:
            if user._ws and user._connected:
                await user._ws.send(json.dumps({"markets": [condition_id], "operation": "subscribe"}))

        # ── PHASE 1: Wait for book to populate ──
        log("  Waiting for book...")
        for _ in range(100):  # up to 10s
            if book.both_sides_live: break
            await asyncio.sleep(0.1)

        if not book.both_sides_live:
            # HTTP fallback
            log("  HTTP fallback...")
            for tok, ab, aa in [(mkt["up_token"], "up_bid", "up_ask"), (mkt["dn_token"], "dn_bid", "dn_ask")]:
                d = api_sync(f"{CLOB}/book?token_id={tok}")
                if d:
                    bids, asks = d.get("bids", []), d.get("asks", [])
                    if bids:
                        bb = max(bids, key=lambda b: sf(b["price"]))
                        setattr(book, ab, sf(bb["price"]))
                    if asks:
                        ba = min(asks, key=lambda a: sf(a["price"]))
                        setattr(book, aa, sf(ba["price"]))

        if not book.both_sides_live:
            log(f"  ✗ Book not populated, skipping slot"); continue

        log(f"  Book ready: UP {book.up_bid:.2f}/{book.up_ask:.2f} (depth {book.up_bid_depth:.0f}) | "
            f"DN {book.dn_bid:.2f}/{book.dn_ask:.2f} (depth {book.dn_bid_depth:.0f}) | gap={book.gap_cents}¢")

        # Check tick threshold at start
        if book.any_side_extreme:
            log(f"  ✗ Price already at extreme ({book.up_bid:.2f}/{book.dn_bid:.2f}), skipping"); continue

        # ── PHASE 2: Prepare execution ──
        log(f"  Ready: orders will be submitted through {PLUGIN_BIN} with strategy attribution")

        # ── PHASE 3: Trade for the entire slot ──
        orders_placed = 0; stop_reason = ""
        last_order_time = 0

        while time.time() < end_ts - 10:
            # Wait for book change
            book.changed.clear()
            try:
                await asyncio.wait_for(book.changed.wait(), timeout=3.0)
            except asyncio.TimeoutError:
                if not book._connected:
                    log("  Market WS dead, reconnecting...")
                    await book.connect(mkt["up_token"], mkt["dn_token"])
                    if book._connected:
                        ws_task = asyncio.create_task(book.read_loop())
                        ping_task = asyncio.create_task(book.ping_loop())
                if user_connected and not user._connected:
                    log("  User WS dead, reconnecting...")
                    await user.connect(condition_id)
                    if user._connected:
                        user_ws_task = asyncio.create_task(user.read_loop())
                        user_ping_task = asyncio.create_task(user.ping_loop())
                    else: user_connected = False
                continue

            # Stop conditions
            if book.tick_changed:
                stop_reason = "tick_size_change"; log("  STOP: tick size changed"); break
            if book.any_side_extreme:
                stop_reason = "price_extreme"
                log(f"  STOP: price extreme UP={book.up_bid:.2f} DN={book.dn_bid:.2f}")
                break

            # Need both sides
            if not book.both_sides_live: continue

            # Gap check
            gap_c = book.gap_cents
            if gap_c < min_gap: continue

            # Rate limit
            now = time.time()
            if now - last_order_time < 1.0: continue

            # Budget check from real fills
            if not dry_run and user.filled_usd >= budget:
                stop_reason = "budget_filled"
                log(f"  STOP: budget filled ${user.filled_usd:.2f} >= ${budget}")
                break

            # Prices: bid at best bid, don't cross ask
            p_up = book.up_bid; p_dn = book.dn_bid
            if book.up_ask > 0 and p_up >= book.up_ask: p_up = round(book.up_ask - 0.01, 2)
            if book.dn_ask > 0 and p_dn >= book.dn_ask: p_dn = round(book.dn_ask - 0.01, 2)
            if p_up <= 0 or p_dn <= 0: continue
            if p_up * 100 + p_dn * 100 >= 100: continue

            # Depth check: enough liquidity on the other side to fill us?
            if book.up_bid_depth < min_depth and book.dn_bid_depth < min_depth:
                continue  # both sides too thin

            # Position-based decision from real fills
            want_up = want_dn = True
            if user.f_up > 0 and user.f_dn > 0:
                r = user.f_up / user.f_dn
                lo, hi = corridor(user.avg_up, user.avg_dn)
                if lo and hi:
                    if r > hi: want_up = False
                    elif r < lo: want_dn = False
                if want_up and not would_improve_pair(user.avg_up, user.avg_dn, user.f_up, user.f_dn, p_up * 100, "up", ORDER_SHARES):
                    want_up = False
                if want_dn and not would_improve_pair(user.avg_up, user.avg_dn, user.f_up, user.f_dn, p_dn * 100, "down", ORDER_SHARES):
                    want_dn = False
            elif user.f_up > 0 and user.f_dn == 0:
                want_up = False  # need DN to balance
            elif user.f_dn > 0 and user.f_up == 0:
                want_dn = False  # need UP to balance

            if not want_up and not want_dn: continue

            # Cancel previous orders, place new pair
            if not dry_run:
                r = await executor.cancel_market(condition_id)
                if not r["ok"]:
                    log(f"  ⚠ cancel via {PLUGIN_BIN} failed: {r['error'][:120]}")

            posted = 0
            if want_up:
                if dry_run:
                    log(f"  UP: {ORDER_SHARES:.0f}sh @ {p_up:.2f} via {PLUGIN_BIN} --dry-run")
                    posted += 1
                else:
                    res = await executor.buy(mkt["up_token"], "yes", p_up, ORDER_SHARES)
                    if res["ok"]:
                        posted += 1
                        oid = find_order_id(res["data"]) or "ok"
                        log(f"  UP: {ORDER_SHARES:.0f}sh @ {p_up:.2f} via {PLUGIN_BIN} → {oid[:16]}")
                    elif "not enough balance" in res["error"].lower() or "insufficient" in res["error"].lower():
                        log("  UP: no balance"); stop_reason = "no_balance"; break
                    else:
                        log(f"  UP FAIL via {PLUGIN_BIN}: {res['error'][:120]}")

            if want_dn:
                if dry_run:
                    log(f"  DN: {ORDER_SHARES:.0f}sh @ {p_dn:.2f} via {PLUGIN_BIN} --dry-run")
                    posted += 1
                else:
                    res = await executor.buy(mkt["dn_token"], "no", p_dn, ORDER_SHARES)
                    if res["ok"]:
                        posted += 1
                        oid = find_order_id(res["data"]) or "ok"
                        log(f"  DN: {ORDER_SHARES:.0f}sh @ {p_dn:.2f} via {PLUGIN_BIN} → {oid[:16]}")
                    elif "not enough balance" in res["error"].lower() or "insufficient" in res["error"].lower():
                        log("  DN: no balance"); stop_reason = "no_balance"; break
                    else:
                        log(f"  DN FAIL via {PLUGIN_BIN}: {res['error'][:120]}")

            if posted > 0:
                orders_placed += posted
                last_order_time = time.time()
                pos = f"UP={user.f_up:.0f}@{user.avg_up:.1f}¢ DN={user.f_dn:.0f}@{user.avg_dn:.1f}¢"
                if user.f_up > 0 and user.f_dn > 0:
                    p1, p2 = calc_pnl(user.avg_up, user.avg_dn, user.f_up, user.f_dn)
                    pos += f" pnl=[{p1:+.2f}/{p2:+.2f}]"
                log(f"  gap={gap_c}¢ | {pos} | strategy-id={strategy_id}")

            if stop_reason: break

        if not stop_reason:
            stop_reason = "slot_ended"

        # Cleanup
        await book.disconnect()
        if ws_task: ws_task.cancel()
        if ping_task: ping_task.cancel()
        if not dry_run:
            r = await executor.cancel_market(condition_id)
            if not r["ok"]:
                log(f"  ⚠ cleanup cancel via {PLUGIN_BIN} failed: {r['error'][:120]}")

        # Result
        pnl = calc_pnl(user.avg_up, user.avg_dn, user.f_up, user.f_dn) if user.f_up > 0 and user.f_dn > 0 else (0, 0)
        result = {
            "slug": slug, "stop": stop_reason, "orders_placed": orders_placed,
            "filled_up": round(user.f_up, 2), "avg_up": round(user.avg_up, 2),
            "filled_dn": round(user.f_dn, 2), "avg_dn": round(user.avg_dn, 2),
            "pair_cost": round(user.pair_cost, 2), "filled_usd": round(user.filled_usd, 2),
            "pnl_if_up": pnl[0], "pnl_if_dn": pnl[1],
            "guaranteed": pnl[0] > 0 and pnl[1] > 0,
        }
        all_results.append(result)
        log(f"--- {slug} done: {stop_reason} | {orders_placed} orders | "
            f"UP={user.f_up:.0f}@{user.avg_up:.1f}¢ DN={user.f_dn:.0f}@{user.avg_dn:.1f}¢ ---")

    # Cleanup user WS
    if user_connected:
        await user.disconnect()
        if user_ws_task: user_ws_task.cancel()
        if user_ping_task: user_ping_task.cancel()

    print(json.dumps({"mode": "v5_single_slot", "execution": "plugin",
                       "strategy_id": strategy_id,
                       "coin": coin.upper(), "tf": tf,
                       "dry_run": dry_run, "markets": all_results}, indent=2))
    log(f"=== DONE ===")


# ── ENTRY ──────────────────────────────────────────────────────

def load_env():
    for p in [".env", os.path.expanduser("~/.env")]:
        if os.path.isfile(p):
            with open(p) as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith("#") and "=" in line:
                        k, v = line.split("=", 1)
                        os.environ.setdefault(k.strip(), v.strip().strip("'\""))
            break

def ensure_wallet(account_id=None):
    import subprocess
    env = {**os.environ, "PATH": os.environ.get("PATH", "") + ":" + os.path.expanduser("~/.local/bin")}
    subprocess.run(["onchainos", "wallet", "login"], capture_output=True, text=True, timeout=15, env=env)
    if account_id:
        subprocess.run(["onchainos", "wallet", "switch", account_id], capture_output=True, timeout=10, env=env)

def main():
    p = argparse.ArgumentParser(description="Polymarket Spread Arb v5 (Single-Slot Event-Driven)")
    p.add_argument("cmd", choices=["run"])
    p.add_argument("--coin", required=True, help="btc,eth,sol,xrp,bnb,doge,hype")
    p.add_argument("--tf", required=True, help="5m, 15m, 1h")
    p.add_argument("--budget", type=float, default=50, help="Max USD to deploy per slot")
    p.add_argument("--min-gap", type=float, default=1, help="Min gap cents to trigger")
    p.add_argument("--min-depth", type=float, default=5, help="Min depth (shares) at best bid to trade")
    p.add_argument("--slots", type=int, default=1, help="How many consecutive slots to trade")
    p.add_argument("--dry-run", action="store_true", help="No real orders")
    p.add_argument("--strategy-id", default=STRATEGY_ID, help="OKX strategy ID passed to polymarket-plugin")
    p.add_argument("--account", default=None, help="Wallet account ID")
    args = p.parse_args()
    load_env()
    ensure_wallet(args.account)
    asyncio.run(run(args))

if __name__ == "__main__":
    main()
