"""OKX onchainos CLI wrapper + HTTP helpers for multi-chain spot system."""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import urllib.request
import urllib.parse

import config

_ONCHAINOS = os.path.expanduser("~/.local/bin/onchainos")
_OKX_BASE = "https://www.okx.com"

WALLET_ADDRESS = None  # set by wallet_preflight()


# ── CLI wrapper ──────────────────────────────────────────────────────

def _onchainos(*args, timeout: int = 30) -> dict:
    """Call onchainos CLI and parse JSON output."""
    cmd = [_ONCHAINOS] + list(args)
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        raise RuntimeError(f"onchainos timeout ({timeout}s): {' '.join(args[:3])}")
    if result.returncode != 0:
        err = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"onchainos error (rc={result.returncode}): {err[:200]}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        raise RuntimeError(f"onchainos invalid JSON: {result.stdout[:200]}")


def _cli_data(resp: dict):
    """Extract .data from onchainos JSON response."""
    return resp.get("data", [])


# ── HTTP helper (public endpoints) ──────────────────────────────────

def _get(path: str, params: dict = None) -> dict:
    """GET from OKX public REST API."""
    url = _OKX_BASE + path
    if params:
        url += "?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, headers={"User-Agent": "spot-multi/1.0"})
    with urllib.request.urlopen(req, timeout=15) as resp:
        return json.loads(resp.read().decode())


# ── Candle data ─────────────────────────────────────────────────────

def kline(token: str, bar: str = config.BAR_SIZE, limit: int = 299,
          chain_index: str | None = None) -> list:
    """Fetch recent candles via CLI. Returns list of candle dicts.
    chain_index overrides config.pair() chain (used for BTC overlay from different chain).
    """
    chain = chain_index or config.pair()["chain_index"]
    r = _onchainos("market", "kline",
                   "--chain", chain,
                   "--address", token,
                   "--bar", bar,
                   "--limit", str(limit))
    return _cli_data(r)


def kline_history(token: str, bar: str = config.BAR_SIZE,
                  limit: int = 100, after: int = 0,
                  chain_index: str | None = None) -> list:
    """Fetch historical candles via OKX public REST API with pagination.
    `after` is a timestamp in ms — returns candles BEFORE this time.
    Returns list of [ts, o, h, l, c, vol] lists, newest-first.
    """
    chain = chain_index or config.pair()["chain_index"]
    params = {
        "chainIndex": chain,
        "tokenContractAddress": token,
        "bar": bar,
        "limit": str(limit),
    }
    if after:
        params["after"] = str(after)
    resp = _get("/api/v5/dex/market/candles", params)
    data = resp.get("data", [])
    return data if isinstance(data, list) else []


# ── Swap execution (Solana: 2-step, EVM: 1-step) ────────────────────

def swap_quote(from_token: str, to_token: str, amount: str) -> dict:
    """Get swap quote via CLI. amount in base units."""
    chain = config.pair()["chain_index"]
    r = _onchainos("swap", "quote",
                   "--chain", chain,
                   "--from", from_token,
                   "--to", to_token,
                   "--amount", amount,
                   timeout=15)
    data = _cli_data(r)
    return data[0] if isinstance(data, list) and data else data if isinstance(data, dict) else {}


def swap_execute(from_token: str, to_token: str, amount: str,
                 slippage: str = "0.005") -> dict:
    """Execute swap via CLI (Solana path). Returns tx data with unsigned transaction."""
    chain = config.pair()["chain_index"]
    r = _onchainos("swap", "swap",
                   "--chain", chain,
                   "--from", from_token,
                   "--to", to_token,
                   "--amount", amount,
                   "--slippage", slippage,
                   "--wallet-address", WALLET_ADDRESS,
                   timeout=30)
    data = _cli_data(r)
    return data[0] if isinstance(data, list) and data else data if isinstance(data, dict) else {}


def sign_and_broadcast(unsigned_tx: str, to_addr: str) -> str:
    """Sign via TEE + broadcast (Solana path). Returns txHash."""
    chain = config.pair()["chain_index"]
    r = _onchainos("wallet", "contract-call",
                   "--chain", chain,
                   "--to", to_addr,
                   "--unsigned-tx", unsigned_tx,
                   timeout=60)
    data = _cli_data(r)
    if isinstance(data, list) and data:
        data = data[0]
    return data.get("txHash", "") if isinstance(data, dict) else ""


def swap_onestep(from_token: str, to_token: str, amount: str,
                 slippage: str = "0.005") -> dict:
    """One-step swap for EVM chains. Handles ERC-20 approval + swap execution.
    Returns dict with txHash and status.
    """
    chain = config.pair()["chain_index"]
    # Get swap data
    r = _onchainos("swap", "swap",
                   "--chain", chain,
                   "--from", from_token,
                   "--to", to_token,
                   "--amount", amount,
                   "--slippage", slippage,
                   "--wallet-address", WALLET_ADDRESS,
                   timeout=30)
    data = _cli_data(r)
    swap_data = data[0] if isinstance(data, list) and data else data if isinstance(data, dict) else {}

    # Check if approval is needed
    approve_data = swap_data.get("approve", swap_data.get("approveData"))
    if approve_data and isinstance(approve_data, dict):
        approve_to = approve_data.get("to", approve_data.get("spenderAddress", ""))
        approve_tx = approve_data.get("data", approve_data.get("callData", ""))
        if approve_to and approve_tx:
            try:
                _onchainos("wallet", "contract-call",
                           "--chain", chain,
                           "--to", approve_to,
                           "--unsigned-tx", approve_tx,
                           timeout=60)
                time.sleep(5)  # wait for approval to confirm
            except Exception as e:
                return {"txHash": "", "status": "APPROVE_FAILED", "error": str(e)}

    # Execute the swap
    unsigned_tx = swap_data.get("tx", swap_data.get("callData", swap_data.get("data", "")))
    to_addr = swap_data.get("to", swap_data.get("routerAddress", ""))
    if not unsigned_tx:
        return {"txHash": "", "status": "NO_TX_DATA", "keys": list(swap_data.keys())}

    try:
        tx_r = _onchainos("wallet", "contract-call",
                          "--chain", chain,
                          "--to", to_addr,
                          "--unsigned-tx", unsigned_tx,
                          timeout=60)
        tx_data = _cli_data(tx_r)
        if isinstance(tx_data, list) and tx_data:
            tx_data = tx_data[0]
        tx_hash = tx_data.get("txHash", "") if isinstance(tx_data, dict) else ""
        return {"txHash": tx_hash, "status": "SUBMITTED"}
    except Exception as e:
        return {"txHash": "", "status": "SWAP_FAILED", "error": str(e)}


def tx_status(tx_hash: str, polls: int = 20, interval: float = 3.0) -> str:
    """Poll tx confirmation. Returns SUCCESS/FAILED/TIMEOUT."""
    chain = config.pair()["chain_index"]
    for _ in range(polls):
        time.sleep(interval)
        try:
            r = _onchainos("wallet", "history",
                           "--tx-hash", tx_hash,
                           "--chain", chain,
                           "--address", WALLET_ADDRESS)
            data = _cli_data(r)
            item = data[0] if isinstance(data, list) and data else (
                data if isinstance(data, dict) else {})
            status = str(item.get("txStatus", "0"))
            if status == "1":
                return "SUCCESS"
            if status == "2":
                return "FAILED"
        except Exception:
            pass
    return "TIMEOUT"


# ── Wallet helpers ──────────────────────────────────────────────────

def wallet_preflight() -> str:
    """Check wallet login and return address for the active chain. Exits on failure."""
    global WALLET_ADDRESS
    r = _onchainos("wallet", "status")
    data = _cli_data(r)
    if not data.get("loggedIn"):
        print("FATAL: Agentic Wallet not logged in. Run: onchainos wallet login <email>")
        sys.exit(1)

    p = config.pair()
    chain = p["chain_index"]
    family = p["chain_family"]

    r2 = _onchainos("wallet", "addresses", "--chain", chain)
    data2 = _cli_data(r2)
    addr = ""

    if family == "solana":
        if isinstance(data2, dict):
            sol_list = data2.get("solana", [])
            if sol_list and isinstance(sol_list[0], dict):
                addr = sol_list[0].get("address", "")
            if not addr:
                addr = data2.get("solAddress", data2.get("address", ""))
        elif isinstance(data2, list) and data2:
            item = data2[0] if isinstance(data2[0], dict) else {}
            addr = item.get("address", "")
    else:  # evm
        if isinstance(data2, dict):
            evm_list = data2.get("evm", data2.get("ethereum", []))
            if evm_list and isinstance(evm_list[0], dict):
                addr = evm_list[0].get("address", "")
            if not addr:
                addr = data2.get("evmAddress", data2.get("address", ""))
        elif isinstance(data2, list) and data2:
            item = data2[0] if isinstance(data2[0], dict) else {}
            addr = item.get("address", "")

    if not addr:
        print(f"FATAL: Could not resolve {family} address from Agentic Wallet")
        sys.exit(1)
    WALLET_ADDRESS = addr
    return addr


_bal_cache = {"base": None, "base_ts": 0, "quote": None, "quote_ts": 0}
_BAL_TTL = 30


def base_balance() -> float | None:
    """Cached base token balance (30s TTL)."""
    now = time.time()
    if _bal_cache["base"] is not None and now - _bal_cache["base_ts"] < _BAL_TTL:
        return _bal_cache["base"]

    p = config.pair()
    chain = p["chain_index"]
    symbol = p["base_symbol"].upper()

    try:
        r = _onchainos("wallet", "balance", "--chain", chain, timeout=10)
        data = _cli_data(r)
        assets = []
        if isinstance(data, dict):
            for detail in data.get("details", []):
                if isinstance(detail, dict):
                    assets.extend(detail.get("tokenAssets", []))
            if not assets:
                assets = [data]
        elif isinstance(data, list):
            assets = data
        for item in assets:
            if isinstance(item, dict) and item.get("symbol", "").upper() == symbol:
                bal = float(item.get("balance", item.get("amount", 0)))
                _bal_cache["base"] = round(bal, 6)
                _bal_cache["base_ts"] = now
                return _bal_cache["base"]
    except Exception:
        pass
    return _bal_cache["base"]


def quote_balance() -> float | None:
    """Cached USDC/quote balance (30s TTL)."""
    now = time.time()
    if _bal_cache["quote"] is not None and now - _bal_cache["quote_ts"] < _BAL_TTL:
        return _bal_cache["quote"]

    p = config.pair()
    chain = p["chain_index"]
    quote_mint = p["quote_mint"]

    try:
        r = _onchainos("portfolio", "token-balances",
                       "--address", WALLET_ADDRESS,
                       "--tokens", f"{chain}:{quote_mint}",
                       timeout=15)
        data = _cli_data(r)
        items = data if isinstance(data, list) else [data] if isinstance(data, dict) else []
        for item in items:
            token_assets = item.get("tokenAssets", []) if isinstance(item, dict) else []
            for t in token_assets:
                addr = t.get("tokenContractAddress", t.get("tokenAddress", ""))
                # Case-insensitive compare for EVM hex addresses
                if addr.lower() == quote_mint.lower():
                    bal = float(t.get("balance", t.get("holdingAmount", 0)))
                    _bal_cache["quote"] = round(bal, 4)
                    _bal_cache["quote_ts"] = now
                    return _bal_cache["quote"]
    except Exception:
        pass
    return _bal_cache["quote"]
