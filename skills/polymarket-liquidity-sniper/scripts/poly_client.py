"""
poly_client.py — Unified Polymarket API client
Wraps: py-clob-client (trading), Gamma API (market data), CLOB REST (orderbook)
"""

import os
import logging
import requests
import time
from dotenv import load_dotenv

load_dotenv()
log = logging.getLogger("poly_client")

CLOB_HOST   = "https://clob.polymarket.com"
GAMMA_HOST  = "https://gamma-api.polymarket.com"
DATA_HOST   = "https://data-api.polymarket.com"

TICK_SIZES = {
    "0.1":  lambda p: round(round(p / 0.1)  * 0.1,  2),
    "0.01": lambda p: round(round(p / 0.01) * 0.01, 2),
}


class PolyClient:
    def __init__(self):
        self.private_key    = os.getenv("POLY_PRIVATE_KEY", "")
        self.funder_address = os.getenv("POLY_FUNDER_ADDRESS", "")
        self.sig_type       = int(os.getenv("POLY_SIGNATURE_TYPE", "2"))
        self._clob          = None
        self._api_creds     = None
        self._session       = requests.Session()
        self._session.headers.update({"Accept": "application/json"})

    def get_active_markets(self, limit=100, topic=None):
        params = {
            "active": "true", "closed": "false", "archived": "false",
            "limit": limit, "order": "volume24hr", "ascending": "false",
        }
        if topic:
            params["tag"] = topic
        try:
            resp = self._session.get(f"{GAMMA_HOST}/markets", params=params, timeout=15)
            resp.raise_for_status()
            markets = resp.json()
            log.info(f"Fetched {len(markets)} active markets")
            return markets
        except requests.RequestException as e:
            log.error(f"Gamma API error: {e}")
            return []

    def get_market(self, condition_id):
        try:
            resp = self._session.get(f"{GAMMA_HOST}/markets/{condition_id}", timeout=10)
            resp.raise_for_status()
            return resp.json()
        except Exception as e:
            log.error(f"get_market {condition_id}: {e}")
            return None

    def get_orderbook(self, token_id):
        try:
            resp = self._session.get(f"{CLOB_HOST}/book", params={"token_id": token_id}, timeout=10)
            resp.raise_for_status()
            return resp.json()
        except Exception as e:
            log.warning(f"Orderbook fetch failed for {token_id[:12]}: {e}")
            return None

    def get_midpoint(self, token_id):
        try:
            resp = self._session.get(f"{CLOB_HOST}/midpoint", params={"token_id": token_id}, timeout=8)
            resp.raise_for_status()
            return float(resp.json().get("mid", 0.5))
        except Exception:
            return None

    def get_spread(self, token_id):
        try:
            resp = self._session.get(f"{CLOB_HOST}/spread", params={"token_id": token_id}, timeout=8)
            resp.raise_for_status()
            return float(resp.json().get("spread", 1.0))
        except Exception:
            return None

    def get_tick_size(self, token_id):
        try:
            resp = self._session.get(f"{CLOB_HOST}/tick-size", params={"token_id": token_id}, timeout=8)
            resp.raise_for_status()
            return str(resp.json().get("minimum_tick_size", "0.01"))
        except Exception:
            return "0.01"

    def _get_clob_client(self):
        if self._clob:
            return self._clob
        if not self.private_key:
            raise RuntimeError("POLY_PRIVATE_KEY not set in .env")
        try:
            from py_clob_client.client import ClobClient
            from eth_account import Account
            signer = Account.from_key(self.private_key)
            client = ClobClient(
                host=CLOB_HOST, chain_id=137, key=self.private_key,
                signature_type=self.sig_type, funder=self.funder_address,
            )
            creds = client.create_or_derive_api_key()
            client.set_api_creds(creds)
            self._clob = client
            self._api_creds = creds
            log.info(f"CLOB client ready | address: {signer.address[:10]}")
            return self._clob
        except ImportError:
            raise RuntimeError("py-clob-client not installed. Run: pip install py-clob-client")

    def place_market_order(self, token_id, side, size_usdc, neg_risk=False, dry_run=True):
        book = self.get_orderbook(token_id)
        if not book:
            return {"success": False, "error": "Could not fetch orderbook"}
        if side == "BUY":
            asks = sorted(book.get("asks", []), key=lambda x: float(x.get("price", 1)))
            if not asks:
                return {"success": False, "error": "No asks available"}
            price = float(asks[0]["price"])
        else:
            bids = sorted(book.get("bids", []), key=lambda x: float(x.get("price", 0)), reverse=True)
            if not bids:
                return {"success": False, "error": "No bids available"}
            price = float(bids[0]["price"])
        tick = self.get_tick_size(token_id)
        price = TICK_SIZES.get(tick, TICK_SIZES["0.01"])(price)
        raw_shares = size_usdc / price if price > 0 else 1.0
        shares = max(round(raw_shares, 2), 1.0)
        size_usdc = round(shares * price, 2)
        log.info(f"{'[DRY RUN] ' if dry_run else ''}MARKET ORDER: {side} {shares:.2f} shares @ {price:.4f} | ${size_usdc:.2f} USDC")
        if dry_run:
            return {"success": True, "simulated": True, "side": side, "price": price,
                    "shares": shares, "size_usdc": size_usdc, "order_id": f"SIM-{int(time.time())}", "fill": "immediate (simulated)"}
        try:
            client = self._get_clob_client()
            order_args = {"token_id": token_id, "price": price, "size": shares, "side": side}
            options = {"tick_size": tick, "neg_risk": neg_risk}
            resp = client.create_and_post_order(order_args, options, "FOK")
            order_id = resp.get("orderID", resp.get("order_id", "unknown"))
            return {"success": True, "simulated": False, "order_id": order_id,
                    "price": price, "shares": shares, "size_usdc": size_usdc, "fill": "immediate", "raw": resp}
        except Exception as e:
            log.error(f"Market order failed: {e}")
            return {"success": False, "error": str(e)}

    def place_limit_order(self, token_id, side, price, size_usdc, neg_risk=False, dry_run=True):
        size_usdc = max(round(size_usdc, 2), 1.00)
        tick = self.get_tick_size(token_id)
        price = TICK_SIZES.get(tick, TICK_SIZES["0.01"])(price)
        shares = round(size_usdc / price, 2)
        if dry_run:
            return {"success": True, "simulated": True, "side": side, "price": price,
                    "shares": shares, "size_usdc": size_usdc, "order_id": f"SIM-{int(time.time())}"}
        try:
            client = self._get_clob_client()
            order_args = {"token_id": token_id, "price": price, "size": shares, "side": side}
            options = {"tick_size": tick, "neg_risk": neg_risk}
            resp = client.create_and_post_order(order_args, options, "GTC")
            order_id = resp.get("orderID", resp.get("order_id", "unknown"))
            return {"success": True, "simulated": False, "order_id": order_id,
                    "price": price, "shares": shares, "size_usdc": size_usdc, "raw": resp}
        except Exception as e:
            log.error(f"Limit order failed: {e}")
            return {"success": False, "error": str(e)}

    def cancel_order(self, order_id, dry_run=True):
        if dry_run:
            log.info(f"[DRY RUN] Would cancel order {order_id}")
            return True
        try:
            client = self._get_clob_client()
            client.cancel({"orderID": order_id})
            return True
        except Exception as e:
            log.error(f"Cancel failed {order_id}: {e}")
            return False

    def get_usdc_balance(self):
        if not self.funder_address:
            return 0.0
        try:
            resp = self._session.get(f"{CLOB_HOST}/balance-allowance",
                params={"asset_type": "COLLATERAL", "signature_type": self.sig_type}, timeout=10)
            resp.raise_for_status()
            return float(resp.json().get("balance", 0))
        except Exception as e:
            log.warning(f"Balance check failed: {e}")
            return 0.0

    def get_open_orders(self):
        try:
            client = self._get_clob_client()
            return client.get_orders() or []
        except Exception as e:
            log.warning(f"get_open_orders: {e}")
            return []

    def get_positions(self):
        try:
            client = self._get_clob_client()
            return client.get_positions() or []
        except Exception as e:
            log.warning(f"get_positions: {e}")
            return []

    def extract_token_ids(self, market):
        tokens = market.get("clobTokenIds", [])
        if isinstance(tokens, str):
            import json
            tokens = json.loads(tokens)
        yes_id = tokens[0] if len(tokens) > 0 else ""
        no_id = tokens[1] if len(tokens) > 1 else ""
        return yes_id, no_id

    def is_neg_risk(self, market):
        return bool(market.get("negRisk", False))
