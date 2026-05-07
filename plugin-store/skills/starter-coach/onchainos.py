"""
OnchainOS CLI wrapper — all on-chain data reads + swap execution.

All commands verified against `onchainos --help` output (v2.5.0+).
New in v2.5.0: cross_chain_* (bridge) and workflow_* (multi-step research).
CLI binary at ~/.local/bin/onchainos.

Usage:
    from onchainos import OnchainOS
    os = OnchainOS(chain="solana")
    tags = os.get_safety_tags("TokenMintAddress...")
"""
from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass, field
from typing import Any

ONCHAINOS_BIN = "onchainos"

# Chain name -> numeric chain ID (for security token-scan --tokens flag)
CHAIN_IDS: dict[str, str] = {
    "solana": "501",
    "ethereum": "1",
    "base": "8453",
    "bsc": "56",
    "bnb": "56",          # alias for bsc
    "polygon": "137",
    "arbitrum": "42161",
    "optimism": "10",
    "avalanche": "43114",
    "sui": "784",
}


@dataclass
class SafetyTags:
    """Token safety tags aggregated from multiple OnchainOS calls."""
    honeypot: bool = False
    lp_locked_pct: float = 0.0
    lp_lock_days: int = 0
    buy_tax_pct: float = 0.0
    sell_tax_pct: float = 0.0
    liquidity_usd: float = 0.0
    top_holders_pct: float = 0.0
    bundler_ratio_pct: float = 0.0
    dev_holding_pct: float = 0.0
    insider_holding_pct: float = 0.0
    fresh_wallet_ratio_pct: float = 0.0
    smart_money_count: int = 0
    phishing_flagged: bool = False
    whale_max_pct: float = 0.0
    mcap_usd: float = 0.0
    launch_age_hours: float = 0.0


@dataclass
class WalletEvent:
    """On-chain wallet transaction event."""
    wallet: str = ""
    token: str = ""
    side: str = ""          # "buy" | "sell"
    usd_amount: float = 0.0
    timestamp: float = 0.0


@dataclass
class RankingItem:
    """Token in a ranking list."""
    token: str = ""
    symbol: str = ""
    rank: int = 0
    chain: str = ""
    mcap_usd: float = 0.0


@dataclass
class SwapResult:
    """Result of a swap execution."""
    ok: bool = False
    tx_hash: str = ""
    error: str = ""
    from_token: str = ""
    to_token: str = ""
    amount: str = ""


def _run_cli(*args: str, timeout: int = 30) -> dict[str, Any]:
    """Run onchainos CLI command and return parsed JSON output."""
    cmd = [ONCHAINOS_BIN] + list(args)
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        if result.returncode != 0:
            return {"error": result.stderr.strip() or f"exit code {result.returncode}"}
        # Try parsing as JSON; some commands return table format
        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError:
            return {"raw": result.stdout.strip()}
    except FileNotFoundError:
        return {"error": f"onchainos CLI not found at '{ONCHAINOS_BIN}'"}
    except subprocess.TimeoutExpired:
        return {"error": f"onchainos CLI timed out ({timeout}s)"}


class OnchainOS:
    def __init__(self, chain: str = "solana"):
        self.chain = chain
        self.chain_id = CHAIN_IDS.get(chain, "501")

    # ── Token Safety ──────────────────────────────────────────────

    def security_scan(self, token: str) -> dict[str, Any]:
        """
        onchainos security token-scan --tokens "chainId:contractAddress"
        Returns raw security scan data.
        """
        token_arg = f"{self.chain_id}:{token}"
        return _run_cli("security", "token-scan", "--tokens", token_arg)

    def get_advanced_info(self, token: str) -> dict[str, Any]:
        """
        onchainos token advanced-info --address <token> --chain <chain>
        Returns risk indicators, creator info, dev stats, holder concentration.
        """
        return _run_cli("token", "advanced-info", "--address", token, "--chain", self.chain)

    def get_price_info(self, token: str) -> dict[str, Any]:
        """
        onchainos token price-info --address <token> --chain <chain>
        Returns price, market cap, liquidity, volume, 24h change.
        """
        return _run_cli("token", "price-info", "--address", token, "--chain", self.chain)

    def get_basic_info(self, token: str) -> dict[str, Any]:
        """
        onchainos token info --address <token> --chain <chain>
        Returns name, symbol, decimals, logo.
        """
        return _run_cli("token", "info", "--address", token, "--chain", self.chain)

    def get_safety_tags(self, token: str) -> SafetyTags:
        """
        Aggregate safety data from security scan + advanced info + holders.
        Combines multiple CLI calls into a single SafetyTags object.
        """
        tags = SafetyTags()

        # 1. Security scan (honeypot, taxes, phishing)
        sec = self.security_scan(token)
        if "error" not in sec:
            tags.honeypot = bool(sec.get("honeypot", False))
            tags.buy_tax_pct = float(sec.get("buyTax", sec.get("buy_tax_pct", 0)) or 0)
            tags.sell_tax_pct = float(sec.get("sellTax", sec.get("sell_tax_pct", 0)) or 0)
            tags.phishing_flagged = bool(sec.get("isPhishing", sec.get("phishing_flagged", False)))

        # 2. Advanced info (mcap, liquidity, dev, holders, launch age)
        adv = self.get_advanced_info(token)
        if "error" not in adv:
            tags.mcap_usd = float(adv.get("marketCap", adv.get("mcap_usd", 0)) or 0)
            tags.liquidity_usd = float(adv.get("liquidity", adv.get("liquidity_usd", 0)) or 0)
            tags.dev_holding_pct = float(adv.get("devHoldPercent", adv.get("dev_holding_pct", 0)) or 0)
            tags.top_holders_pct = float(adv.get("top10HoldPercent", adv.get("top_holders_pct", 0)) or 0)
            tags.lp_locked_pct = float(adv.get("lpLockedPercent", adv.get("lp_locked_pct", 0)) or 0)
            tags.launch_age_hours = float(adv.get("launchAgeHours", adv.get("launch_age_hours", 0)) or 0)

        # 3. Bundle info (bundler ratio)
        bundle = self.get_bundle_info(token)
        if "error" not in bundle:
            tags.bundler_ratio_pct = float(bundle.get("bundlePercent", bundle.get("bundler_ratio_pct", 0)) or 0)

        # 4. Smart money holders count
        sm_holders = self.get_holders(token, tag_filter=3)  # 3 = Smart Money
        if isinstance(sm_holders, list):
            tags.smart_money_count = len(sm_holders)

        # 5. Fresh wallet ratio
        fresh = self.get_holders(token, tag_filter=5)  # 5 = Fresh Wallet
        all_holders = self.get_holders(token)
        if isinstance(fresh, list) and isinstance(all_holders, list) and all_holders:
            tags.fresh_wallet_ratio_pct = len(fresh) / len(all_holders) * 100

        # 6. Insider holding
        insiders = self.get_holders(token, tag_filter=6)  # 6 = Insider
        if isinstance(insiders, list):
            # Sum their holding percentages if available
            tags.insider_holding_pct = sum(
                float(h.get("holdPercent", 0) or 0) for h in insiders
                if isinstance(h, dict)
            )

        # 7. Whale max (single largest non-LP)
        whales = self.get_holders(token, tag_filter=4)  # 4 = Whale
        if isinstance(whales, list) and whales:
            max_pct = max(
                (float(h.get("holdPercent", 0) or 0) for h in whales if isinstance(h, dict)),
                default=0,
            )
            tags.whale_max_pct = max_pct

        return tags

    # ── Holders ───────────────────────────────────────────────────

    def get_holders(self, token: str, tag_filter: int | None = None) -> list[dict] | dict:
        """
        onchainos token holders --address <token> --chain <chain> [--tag-filter <n>]
        Tag filters: 1=KOL, 2=Developer, 3=Smart Money, 4=Whale,
                     5=Fresh Wallet, 6=Insider, 7=Sniper, 8=Phishing, 9=Bundler
        """
        args = ["token", "holders", "--address", token, "--chain", self.chain]
        if tag_filter is not None:
            args.extend(["--tag-filter", str(tag_filter)])
        data = _run_cli(*args)
        if "error" in data:
            return data
        # Return the holders list if present, else the raw data
        return data.get("holders", data.get("data", []))

    def get_smart_money_holders(self, token: str) -> list[str]:
        """Return wallet addresses of smart-money holders."""
        holders = self.get_holders(token, tag_filter=3)
        if isinstance(holders, list):
            return [
                h.get("wallet", h.get("address", ""))
                for h in holders if isinstance(h, dict)
            ]
        return []

    # ── Dev / Memepump ────────────────────────────────────────────

    def get_dev_info(self, token: str) -> dict[str, Any]:
        """
        onchainos memepump token-dev-info --address <token> --chain <chain>
        Returns dev wallet, launch history, rug history.
        """
        return _run_cli("memepump", "token-dev-info", "--address", token, "--chain", self.chain)

    def get_bundle_info(self, token: str) -> dict[str, Any]:
        """
        onchainos memepump token-bundle-info --address <token> --chain <chain>
        Returns bundler/sniper info.
        """
        return _run_cli("memepump", "token-bundle-info", "--address", token, "--chain", self.chain)

    def get_dev_wallet(self, token: str) -> str | None:
        """Return the deployer wallet address."""
        data = self.get_dev_info(token)
        if "error" in data:
            return None
        return data.get("devWallet", data.get("dev_wallet", data.get("creator")))

    # ── Wallet Tracking ───────────────────────────────────────────

    def track_wallets(self, wallets: list[str], trade_type: int = 0) -> list[WalletEvent]:
        """
        onchainos tracker activities --tracker-type multi_address
            --wallet-address "addr1,addr2" --chain <chain>
            [--trade-type 0|1|2]
        trade_type: 0=all, 1=buy, 2=sell
        """
        wallet_csv = ",".join(wallets[:20])  # max 20
        args = [
            "tracker", "activities",
            "--tracker-type", "multi_address",
            "--wallet-address", wallet_csv,
            "--chain", self.chain,
        ]
        if trade_type != 0:
            args.extend(["--trade-type", str(trade_type)])
        data = _run_cli(*args)
        if "error" in data:
            return []
        return self._parse_events(data)

    def track_smart_money(self, trade_type: int = 0) -> list[WalletEvent]:
        """
        onchainos tracker activities --tracker-type smart_money --chain <chain>
        """
        args = [
            "tracker", "activities",
            "--tracker-type", "smart_money",
            "--chain", self.chain,
        ]
        if trade_type != 0:
            args.extend(["--trade-type", str(trade_type)])
        data = _run_cli(*args)
        if "error" in data:
            return []
        return self._parse_events(data)

    def _parse_events(self, data: dict) -> list[WalletEvent]:
        events = []
        for tx in data.get("activities", data.get("transactions", data.get("data", []))):
            if not isinstance(tx, dict):
                continue
            events.append(WalletEvent(
                wallet=tx.get("wallet", tx.get("walletAddress", "")),
                token=tx.get("token", tx.get("tokenAddress", "")),
                side=tx.get("side", tx.get("tradeType", "")),
                usd_amount=float(tx.get("usd_amount", tx.get("amount", 0)) or 0),
                timestamp=float(tx.get("timestamp", tx.get("txTime", 0)) or 0),
            ))
        return events

    # ── Rankings / Hot Tokens ─────────────────────────────────────

    def get_hot_tokens(
        self,
        ranking_type: int = 4,
        top_n: int = 50,
        time_frame: int = 4,
        **filters: Any,
    ) -> list[RankingItem]:
        """
        onchainos token hot-tokens --chain <chain> --ranking-type <n>
        ranking_type: 4=Trending (token score), 5=X-mentioned
        time_frame: 1=5min, 2=1h, 3=4h, 4=24h
        filters: market_cap_min, market_cap_max, liquidity_min, liquidity_max, etc.
        """
        args = [
            "token", "hot-tokens",
            "--chain", self.chain,
            "--ranking-type", str(ranking_type),
            "--time-frame", str(time_frame),
        ]
        # Map filter kwargs to CLI flags
        flag_map = {
            "market_cap_min": "--market-cap-min",
            "market_cap_max": "--market-cap-max",
            "liquidity_min": "--liquidity-min",
            "liquidity_max": "--liquidity-max",
            "volume_min": "--volume-min",
            "volume_max": "--volume-max",
            "holders_min": "--holders-min",
            "project_id": "--project-id",
        }
        for key, flag in flag_map.items():
            if key in filters:
                args.extend([flag, str(filters[key])])

        data = _run_cli(*args)
        if "error" in data:
            return []
        items = []
        for i, tok in enumerate(data.get("tokens", data.get("data", [])), 1):
            if not isinstance(tok, dict):
                continue
            if i > top_n:
                break
            items.append(RankingItem(
                token=tok.get("tokenAddress", tok.get("token", "")),
                symbol=tok.get("symbol", ""),
                rank=i,
                chain=self.chain,
                mcap_usd=float(tok.get("marketCap", tok.get("mcap_usd", 0)) or 0),
            ))
        return items

    def subscribe_ranking(self, list_name: str, top_n: int = 50) -> list[RankingItem]:
        """
        Map spec list_name to the correct OnchainOS command.
        Routes based on what production skills actually use:
          new/bonding  → memepump tokens (pump.fun launches)
          trending     → token trending --sort-by trending
          gainers      → token trending --sort-by gainers
          volume       → token trending --sort-by volume
          hot          → token hot-tokens
        """
        if list_name in ("new", "bonding"):
            raw = self.get_memepump_tokens(stage="bonding")
            items = []
            for i, tok in enumerate(raw[:top_n], 1):
                if not isinstance(tok, dict):
                    continue
                items.append(RankingItem(
                    token=tok.get("tokenAddress", tok.get("token", tok.get("contractAddress", ""))),
                    symbol=tok.get("symbol", ""),
                    rank=i,
                    chain=self.chain,
                    mcap_usd=float(tok.get("marketCap", tok.get("mcap_usd", 0)) or 0),
                ))
            return items

        if list_name in ("trending", "gainers", "volume"):
            tf_map   = {"trending": "1h", "gainers": "1h", "volume": "1h"}
            sort_map = {"trending": "trending", "gainers": "gainers", "volume": "volume"}
            raw = self.get_token_trending(
                sort_by=sort_map[list_name],
                time_frame=tf_map[list_name],
            )
            items = []
            for i, tok in enumerate(raw[:top_n], 1):
                if not isinstance(tok, dict):
                    continue
                items.append(RankingItem(
                    token=tok.get("tokenAddress", tok.get("token", "")),
                    symbol=tok.get("symbol", ""),
                    rank=i,
                    chain=self.chain,
                    mcap_usd=float(tok.get("marketCap", 0) or 0),
                ))
            return items

        # Fallback: hot-tokens
        return self.get_hot_tokens(ranking_type=4, top_n=top_n)

    # ── Market Data ───────────────────────────────────────────────

    def get_candles(
        self, token: str, bar: str = "1H", limit: int = 100
    ) -> list[dict[str, Any]]:
        """
        onchainos market kline --address <token> --chain <chain> --bar <bar> --limit <n>
        bar: 1s, 1m, 5m, 15m, 30m, 1H, 4H, 1D, 1W
        limit: max 299
        Returns list of {ts, open, high, low, close, volume}.
        """
        data = _run_cli(
            "market", "kline",
            "--address", token,
            "--chain", self.chain,
            "--bar", bar,
            "--limit", str(min(limit, 299)),
        )
        if "error" in data:
            return []
        candles = data.get("candles", data.get("data", []))
        result = []
        for c in candles:
            if not isinstance(c, dict):
                continue
            result.append({
                "ts": float(c.get("ts", c.get("time", 0)) or 0),
                "open": float(c.get("open", c.get("o", 0)) or 0),
                "high": float(c.get("high", c.get("h", 0)) or 0),
                "low": float(c.get("low", c.get("l", 0)) or 0),
                "close": float(c.get("close", c.get("c", 0)) or 0),
                "volume": float(c.get("volume", c.get("vol", 0)) or 0),
            })
        return result

    # ── Signals ───────────────────────────────────────────────────

    def get_signals(
        self,
        wallet_type: int = 1,
        token: str | None = None,
        min_amount_usd: float | None = None,
    ) -> list[dict[str, Any]]:
        """
        onchainos signal list --chain <chain> --wallet-type <n>
        wallet_type: 1=Smart Money, 2=KOL, 3=Whales
        """
        args = [
            "signal", "list",
            "--chain", self.chain,
            "--wallet-type", str(wallet_type),
        ]
        if token:
            args.extend(["--token-address", token])
        if min_amount_usd:
            args.extend(["--min-amount-usd", str(int(min_amount_usd))])
        data = _run_cli(*args)
        if "error" in data:
            return []
        return data.get("signals", data.get("data", []))

    # ── Token Trades ──────────────────────────────────────────────

    def get_token_trades(
        self, token: str, limit: int = 200, tag_filter: int | None = None
    ) -> list[dict[str, Any]]:
        """
        onchainos token trades --address <token> --chain <chain> --limit <n>
        tag_filter: 1=KOL, 2=Dev, 3=Smart Money, 4=Whale, etc.
        """
        args = [
            "token", "trades",
            "--address", token,
            "--chain", self.chain,
            "--limit", str(limit),
        ]
        if tag_filter is not None:
            args.extend(["--tag-filter", str(tag_filter)])
        data = _run_cli(*args)
        if "error" in data:
            return []
        return data.get("trades", data.get("data", []))

    # ── Swap ──────────────────────────────────────────────────────

    def swap_quote(
        self, from_token: str, to_token: str, readable_amount: str
    ) -> dict[str, Any]:
        """
        onchainos swap quote --from <addr> --to <addr> --readable-amount <amt> --chain <chain>
        Read-only price estimate.
        """
        return _run_cli(
            "swap", "quote",
            "--from", from_token,
            "--to", to_token,
            "--readable-amount", readable_amount,
            "--chain", self.chain,
        )

    def swap_execute(
        self,
        from_token: str,
        to_token: str,
        readable_amount: str,
        wallet: str,
        slippage: float | None = None,
        mev_protection: bool = False,
    ) -> SwapResult:
        """
        onchainos swap execute --from <addr> --to <addr> --readable-amount <amt>
            --chain <chain> --wallet <wallet> [--slippage <n>] [--mev-protection]
        Full execution: quote -> approve -> swap -> sign -> broadcast.
        """
        args = [
            "swap", "execute",
            "--from", from_token,
            "--to", to_token,
            "--readable-amount", readable_amount,
            "--chain", self.chain,
            "--wallet", wallet,
            "--biz-type", "dex",
            "--strategy", "starter-coach",
        ]
        if slippage is not None:
            args.extend(["--slippage", str(slippage)])
        if mev_protection:
            args.append("--mev-protection")

        data = _run_cli(*args, timeout=60)
        if "error" in data:
            return SwapResult(ok=False, error=data["error"])
        return SwapResult(
            ok=True,
            tx_hash=data.get("txHash", data.get("tx_hash", "")),
            from_token=from_token,
            to_token=to_token,
            amount=readable_amount,
        )

    # ── Wallet / Position Monitoring ──────────────────────────────

    def wallet_status(self) -> dict[str, Any]:
        """
        onchainos wallet status
        Returns login state, active account, wallet addresses.
        Normalizes: loggedIn surfaced to top-level regardless of nesting.
        """
        result = _run_cli("wallet", "status")
        # CLI returns {"ok": true, "data": {"loggedIn": true, ...}}
        # Normalize: promote loggedIn to top level for convenience
        if "loggedIn" not in result and "data" in result and isinstance(result["data"], dict):
            result["loggedIn"] = result["data"].get("loggedIn", False)
        return result

    def wallet_addresses(self) -> dict[str, Any]:
        """
        onchainos wallet addresses --chain <chainId>
        Returns wallet addresses grouped by chain category.
        """
        return _run_cli("wallet", "addresses", "--chain", self.chain_id)

    def get_wallet_address(self) -> str | None:
        """Resolve the bot's own wallet address for this chain."""
        raw = self.wallet_addresses()
        if "error" in raw:
            return None
        # Response: {"ok": true, "data": {"solana": [{"address": "..."}], "evm": [], ...}}
        inner = raw.get("data", raw)
        if isinstance(inner, dict):
            # Chain-keyed: look for solana / evm / xlayer lists
            chain_keys = ["solana", "evm", "xlayer", "addresses"]
            for ck in chain_keys:
                lst = inner.get(ck)
                if isinstance(lst, list) and lst:
                    first = lst[0]
                    addr = first.get("address") or first.get("wallet") if isinstance(first, dict) else first
                    if addr:
                        return str(addr)
            # Flat address string
            for key in ("address", "wallet"):
                val = inner.get(key)
                if isinstance(val, str) and val:
                    return val
        return None

    def get_all_balances(self, force: bool = False) -> list[dict[str, Any]]:
        """
        onchainos wallet balance --chain <chainId> [--force]
        Returns all token balances for the active wallet on this chain.
        Each item: {tokenAddress, symbol, balance, balanceUsd, ...}
        """
        args = ["wallet", "balance", "--chain", self.chain_id]
        if force:
            args.append("--force")
        data = _run_cli(*args)
        if "error" in data:
            return []
        # Normalize response
        balances = data.get("balances", data.get("data", data.get("assets", [])))
        if isinstance(balances, list):
            return balances
        return []

    def get_token_balance(self, token_address: str, force: bool = False) -> float:
        """
        onchainos wallet balance --chain <chainId> --token-address <addr>
        Returns balance amount for a specific token. 0.0 if not held.
        """
        args = [
            "wallet", "balance",
            "--chain", self.chain_id,
            "--token-address", token_address,
        ]
        if force:
            args.append("--force")
        data = _run_cli(*args)
        if "error" in data:
            return 0.0
        # Parse balance from response
        bal = data.get("balance", data.get("amount", 0))
        if isinstance(bal, (int, float)):
            return float(bal)
        if isinstance(bal, str):
            try:
                return float(bal)
            except ValueError:
                return 0.0
        # May be nested in a list
        balances = data.get("balances", data.get("data", []))
        if isinstance(balances, list) and balances:
            first = balances[0] if isinstance(balances[0], dict) else {}
            return float(first.get("balance", first.get("amount", 0)) or 0)
        return 0.0

    def get_wallet_history(
        self, limit: int = 20, begin_ms: int | None = None
    ) -> list[dict[str, Any]]:
        """
        onchainos wallet history --chain <chainId> --limit <n>
        Returns recent transactions for the active wallet.
        """
        args = ["wallet", "history", "--chain", self.chain_id, "--limit", str(limit)]
        if begin_ms is not None:
            args.extend(["--begin", str(begin_ms)])
        data = _run_cli(*args)
        if "error" in data:
            return []
        return data.get("orders", data.get("transactions", data.get("data", [])))

    def get_tx_detail(self, tx_hash: str, address: str) -> dict[str, Any]:
        """
        onchainos wallet history --tx-hash <hash> --address <addr> --chain <chainId>
        Returns detail for a specific transaction (confirm swap completed).
        """
        return _run_cli(
            "wallet", "history",
            "--tx-hash", tx_hash,
            "--address", address,
            "--chain", self.chain_id,
        )

    def wallet_contract_call(
        self,
        to: str,
        unsigned_tx: str,
    ) -> dict[str, Any]:
        """
        onchainos wallet contract-call --chain <chainId> --to <addr> --unsigned-tx <data>
        TEE-sign and broadcast a transaction (Agentic Wallet path).
        Used by scan_live, AI Whale, and any skill needing non-swap on-chain actions.
        Returns {txHash, status, ...}
        """
        return _run_cli(
            "wallet", "contract-call",
            "--chain", self.chain_id,
            "--to", to,
            "--unsigned-tx", unsigned_tx,
            "--biz-type", "dex",
            "--strategy", "starter-coach",
        )

    # ── Portfolio ─────────────────────────────────────────────────

    def get_portfolio_balances(self) -> list[dict[str, Any]]:
        """
        onchainos portfolio all-balances --chain <chainId>
        All token balances in the active wallet on this chain.
        """
        data = _run_cli("portfolio", "all-balances", "--chain", self.chain_id)
        if "error" in data:
            return []
        return data.get("balances", data.get("data", []))

    def get_portfolio_token_balances(
        self, address: str | None = None
    ) -> list[dict[str, Any]]:
        """
        onchainos portfolio token-balances --chain <chainId> [--address <addr>]
        Token balances by address (defaults to logged-in wallet).
        """
        args = ["portfolio", "token-balances", "--chain", self.chain_id]
        if address:
            args.extend(["--address", address])
        data = _run_cli(*args)
        if "error" in data:
            return []
        return data.get("balances", data.get("data", []))

    def get_portfolio_token_pnl(self, wallet: str, token: str) -> dict[str, Any]:
        """
        onchainos market portfolio-token-pnl --chain <chainId>
            --address <wallet> --token <tokenAddr>
        Returns realized/unrealized PnL for a specific held token.
        """
        return _run_cli(
            "market", "portfolio-token-pnl",
            "--chain", self.chain_id,
            "--address", wallet,
            "--token", token,
        )

    # ── Token extended ────────────────────────────────────────────

    def get_token_liquidity(self, token: str) -> dict[str, Any]:
        """
        onchainos token liquidity --chain <chain> --address <token>
        Returns LP pool data: pool address, reserve amounts, LP locked %.
        """
        return _run_cli("token", "liquidity", "--address", token, "--chain", self.chain)

    def get_token_trending(
        self,
        sort_by: str = "trending",
        time_frame: str = "1h",
        chain: str | None = None,
        **filters: Any,
    ) -> list[dict[str, Any]]:
        """
        onchainos token trending --chain <chain> --sort-by <sort> --time-frame <tf>
        sort_by: trending | volume | gainers | new
        time_frame: 5min | 1h | 4h | 24h
        filters: market_cap_min, market_cap_max, liquidity_min, holders_min, etc.
        Used by Ranking Sniper and Meme Trenching skills.
        """
        args = [
            "token", "trending",
            "--chain", chain or self.chain,
            "--sort-by", sort_by,
            "--time-frame", time_frame,
        ]
        flag_map = {
            "market_cap_min":  "--market-cap-min",
            "market_cap_max":  "--market-cap-max",
            "liquidity_min":   "--liquidity-min",
            "holders_min":     "--holders-min",
            "volume_min":      "--volume-min",
        }
        for key, flag in flag_map.items():
            if key in filters:
                args.extend([flag, str(filters[key])])
        data = _run_cli(*args)
        if "error" in data:
            return []
        return data.get("tokens", data.get("data", []))

    def get_batch_prices(self, token_chain_pairs: list[tuple[str, str]]) -> dict[str, Any]:
        """
        onchainos market prices --tokens "chainId:addr,chainId:addr,..."
        Batch price fetch — efficient for monitoring multiple positions.
        token_chain_pairs: [(token_address, chain_name), ...]
        """
        tokens_arg = ",".join(
            f"{CHAIN_IDS.get(chain, chain)}:{addr}"
            for addr, chain in token_chain_pairs
        )
        return _run_cli("market", "prices", "--tokens", tokens_arg)

    # ── Memepump extended ─────────────────────────────────────────

    def get_memepump_tokens(
        self,
        stage: str = "all",
        **filters: Any,
    ) -> list[dict[str, Any]]:
        """
        onchainos memepump tokens --chain <chain> --stage <stage> [filters...]
        stage: all | bonding | graduated
        filters: max_market_cap, min_holders, max_bundlers_percent, max_dev_holdings_percent,
                 max_top10_holdings_percent, max_insiders_percent, max_snipers_percent,
                 max_fresh_wallets_percent, protocol_id_list
        Used by scan_live.py and AI Whale for pump.fun token scanning.
        """
        args = [
            "memepump", "tokens",
            "--chain", self.chain,
            "--stage", stage,
        ]
        flag_map = {
            "max_market_cap":           "--max-market-cap",
            "min_holders":              "--min-holders",
            "max_bundlers_percent":     "--max-bundlers-percent",
            "max_dev_holdings_percent": "--max-dev-holdings-percent",
            "max_top10_holdings_percent": "--max-top10-holdings-percent",
            "max_insiders_percent":     "--max-insiders-percent",
            "max_snipers_percent":      "--max-snipers-percent",
            "max_fresh_wallets_percent": "--max-fresh-wallets-percent",
            "protocol_id_list":         "--protocol-id-list",
        }
        for key, flag in flag_map.items():
            if key in filters:
                args.extend([flag, str(filters[key])])
        data = _run_cli(*args)
        if "error" in data:
            return []
        return data.get("tokens", data.get("data", []))

    def get_memepump_token_details(
        self, token: str, wallet: str | None = None
    ) -> dict[str, Any]:
        """
        onchainos memepump token-details --chain <chain> --address <token>
            [--wallet-address <wallet>]
        Full token details: bonding curve progress, holder breakdown, social links.
        """
        args = ["memepump", "token-details", "--chain", self.chain, "--address", token]
        if wallet:
            args.extend(["--wallet-address", wallet])
        return _run_cli(*args)

    def get_aped_wallets(self, token: str) -> list[dict[str, Any]]:
        """
        onchainos memepump aped-wallet --chain <chain> --address <token>
        Returns wallets that co-invested (same group / coordinated buyers).
        Useful for detecting wash trading clusters.
        """
        data = _run_cli("memepump", "aped-wallet", "--chain", self.chain, "--address", token)
        if "error" in data:
            return []
        return data.get("wallets", data.get("data", []))

    def get_similar_tokens(self, token: str) -> list[dict[str, Any]]:
        """
        onchainos memepump similar-tokens --chain <chain> --address <token>
        Returns other tokens launched by the same developer.
        Used to check dev's rug history across projects.
        """
        data = _run_cli("memepump", "similar-tokens", "--chain", self.chain, "--address", token)
        if "error" in data:
            return []
        return data.get("tokens", data.get("data", []))

    # ── Tracker extended ──────────────────────────────────────────

    def track_kol(self, trade_type: int = 0) -> list[WalletEvent]:
        """
        onchainos tracker activities --tracker-type kol --chain <chain>
        Track KOL (Key Opinion Leader) wallet activity.
        trade_type: 0=all, 1=buy, 2=sell
        """
        args = [
            "tracker", "activities",
            "--tracker-type", "kol",
            "--chain", self.chain,
        ]
        if trade_type != 0:
            args.extend(["--trade-type", str(trade_type)])
        data = _run_cli(*args)
        if "error" in data:
            return []
        return self._parse_events(data)

    def track_with_filters(
        self,
        tracker_type: str = "smart_money",
        trade_type: int = 0,
        min_volume: float | None = None,
        max_volume: float | None = None,
        min_holders: int | None = None,
        min_market_cap: float | None = None,
        max_market_cap: float | None = None,
        min_liquidity: float | None = None,
        wallet_addresses: list[str] | None = None,
    ) -> list[WalletEvent]:
        """
        onchainos tracker activities with full filter set.
        tracker_type: smart_money | kol | multi_address
        Covers all filter params used across AI Whale and Wallet Tracker skills.
        """
        args = [
            "tracker", "activities",
            "--tracker-type", tracker_type,
            "--chain", self.chain,
        ]
        if trade_type != 0:
            args.extend(["--trade-type", str(trade_type)])
        if min_volume is not None:
            args.extend(["--min-volume", str(int(min_volume))])
        if max_volume is not None:
            args.extend(["--max-volume", str(int(max_volume))])
        if min_holders is not None:
            args.extend(["--min-holders", str(min_holders)])
        if min_market_cap is not None:
            args.extend(["--min-market-cap", str(int(min_market_cap))])
        if max_market_cap is not None:
            args.extend(["--max-market-cap", str(int(max_market_cap))])
        if min_liquidity is not None:
            args.extend(["--min-liquidity", str(int(min_liquidity))])
        if wallet_addresses:
            args.extend(["--wallet-address", ",".join(wallet_addresses[:20])])
        data = _run_cli(*args)
        if "error" in data:
            return []
        return self._parse_events(data)

    # ── Cross-chain bridge (v2.5.0+) ──────────────────────────────────

    def cross_chain_chains(self) -> dict[str, Any]:
        """
        onchainos cross-chain chains
        Returns supported chain pairs for cross-chain bridging.
        """
        return _run_cli("cross-chain", "chains")

    def cross_chain_quote(
        self,
        from_token: str,
        to_token: str,
        from_chain: str,
        to_chain: str,
        amount: str,
        receive_address: str | None = None,
        sort: int = 0,
    ) -> dict[str, Any]:
        """
        onchainos cross-chain quote --from <> --to <> --from-chain <> --to-chain <> --readable-amount <>
        Get a cross-chain bridge quote (read-only, no signing).
        sort: 0=optimal, 1=fastest, 2=max output
        """
        args = [
            "cross-chain", "quote",
            "--from", from_token,
            "--to", to_token,
            "--from-chain", from_chain,
            "--to-chain", to_chain,
            "--readable-amount", str(amount),
            "--sort", str(sort),
        ]
        if receive_address:
            args.extend(["--receive-address", receive_address])
        return _run_cli(*args)

    def cross_chain_probe(
        self,
        from_chain: str,
        to_chain: str,
        amount: str = "100",
    ) -> dict[str, Any]:
        """
        onchainos cross-chain probe --from-chain <> --to-chain <>
        Probe which common tokens (USDC/USDT/native) can be bridged between two chains.
        """
        return _run_cli(
            "cross-chain", "probe",
            "--from-chain", from_chain,
            "--to-chain", to_chain,
            "--readable-amount", str(amount),
        )

    def cross_chain_status(self, order_id: str) -> dict[str, Any]:
        """
        onchainos cross-chain status --order-id <>
        Query the status of a cross-chain bridge order.
        """
        return _run_cli("cross-chain", "status", "--order-id", order_id)

    # ── Workflow (v2.5.0+) ────────────────────────────────────────────

    def workflow_token_research(
        self,
        address: str | None = None,
        query: str | None = None,
        chain: str | None = None,
    ) -> dict[str, Any]:
        """
        onchainos workflow token-research --address <> | --query <>
        Full token due diligence: price, security, holders, signals, launchpad.
        Provide either address (contract) or query (symbol/name search).
        """
        args = ["workflow", "token-research"]
        if address:
            args.extend(["--address", address])
        elif query:
            args.extend(["--query", query])
        if chain:
            args.extend(["--chain", chain])
        return _run_cli(*args, timeout=60)

    def workflow_smart_money(self, chain: str | None = None) -> dict[str, Any]:
        """
        onchainos workflow smart-money
        Aggregate smart money signals by token with per-token due diligence.
        """
        args = ["workflow", "smart-money"]
        if chain:
            args.extend(["--chain", chain])
        return _run_cli(*args, timeout=60)

    def workflow_new_tokens(
        self,
        chain: str | None = None,
        stage: str = "MIGRATED",
    ) -> dict[str, Any]:
        """
        onchainos workflow new-tokens [--stage MIGRATED|MIGRATING]
        New token screening: launchpad scan + safety enrichment for top 10.
        """
        args = ["workflow", "new-tokens", "--stage", stage]
        if chain:
            args.extend(["--chain", chain])
        return _run_cli(*args, timeout=60)

    def workflow_wallet_analysis(
        self,
        address: str,
        chain: str | None = None,
    ) -> dict[str, Any]:
        """
        onchainos workflow wallet-analysis --address <>
        7d/30d wallet performance, trading behaviour, recent activity.
        """
        args = ["workflow", "wallet-analysis", "--address", address]
        if chain:
            args.extend(["--chain", chain])
        return _run_cli(*args, timeout=60)

    def workflow_portfolio(
        self,
        address: str,
        chain: str | None = None,
        chains: str | None = None,
    ) -> dict[str, Any]:
        """
        onchainos workflow portfolio --address <>
        Portfolio check: balances, total value, 30d PnL overview.
        chains: comma-separated list of chains (defaults to all supported).
        """
        args = ["workflow", "portfolio", "--address", address]
        if chain:
            args.extend(["--chain", chain])
        if chains:
            args.extend(["--chains", chains])
        return _run_cli(*args, timeout=60)
