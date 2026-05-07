"""
一键发币 v1.0 — One-click token launch.

Usage:
  # From Claude Code or any Python caller — one call does everything:
  from token_launch import quick_launch
  result = await quick_launch("MoonDog", "MDOG", "a good boy", "/path/to/dog.png")

  # With options:
  result = await quick_launch(
      "MoonDog", "MDOG", "a good boy", "https://example.com/dog.png",
      launchpad="pumpfun", buy_amount=0.1,
      twitter="https://twitter.com/moondog",
  )

  # Dashboard mode:
  python3 token_launch.py
"""
from __future__ import annotations

import asyncio
import base64 as b64
import json
import os
import sys
import subprocess
import tempfile
import time
from datetime import datetime, timezone
from http.server import HTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
from threading import Lock, Thread
from typing import Optional

# ── Ensure skill directory is on sys.path (works from any CWD) ────────
_SKILL_DIR = str(Path(__file__).resolve().parent)
if _SKILL_DIR not in sys.path:
    sys.path.insert(0, _SKILL_DIR)

import config as C
import ipfs
from launchpads import get_adapter, LaunchParams, LaunchResult
from launchpads.base import onchainos_bin

# ── State ─────────────────────────────────────────────────────────────
_BASE_DIR   = Path(__file__).parent
_STATE_DIR  = _BASE_DIR / "state"
_LAUNCHES   = _STATE_DIR / "launches.json"
_TEMPLATES  = _STATE_DIR / "templates.json"

_wallet_sol: str = ""
_wallet_bsc: str = ""
_wallet_ready: bool = False
_dashboard_started: bool = False
_launches_lock = Lock()


# ══════════════════════════════════════════════════════════════════════
# Wallet Preflight
# ══════════════════════════════════════════════════════════════════════

def _run_onchainos(*args: str) -> dict:
    """Run onchainos CLI and return parsed JSON output."""
    cmd = [onchainos_bin()]
    cmd.extend(args)
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        if proc.returncode != 0:
            return {"ok": False, "error": proc.stderr.strip() or f"exit code {proc.returncode}"}
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {"ok": False, "error": f"Invalid JSON: {proc.stdout[:200]}"}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def wallet_preflight() -> bool:
    """Check wallet login and resolve addresses."""
    global _wallet_sol, _wallet_bsc, _wallet_ready

    # Check login
    status = _run_onchainos("wallet", "status")
    if not status.get("data", {}).get("loggedIn"):
        print("ERROR: Wallet not logged in.")
        print("  Run: onchainos wallet login <your-email>")
        return False

    email = status["data"].get("email", "unknown")
    print(f"  Wallet logged in: {email}")

    # Resolve addresses from wallet addresses command
    addrs = _run_onchainos("wallet", "addresses")
    addr_data = addrs.get("data", {})
    if isinstance(addr_data, dict):
        sol_list = addr_data.get("solana", [])
        if sol_list:
            _wallet_sol = sol_list[0].get("address", "")
        # EVM address for BSC/ETH — find chainIndex 56 (BSC) or use first EVM
        evm_list = addr_data.get("evm", [])
        for evm in evm_list:
            if evm.get("chainIndex") == "56":
                _wallet_bsc = evm.get("address", "")
                break
        if not _wallet_bsc and evm_list:
            _wallet_bsc = evm_list[0].get("address", "")

    # Use overrides if set
    _wallet_sol = C.WALLET_SOL or _wallet_sol
    _wallet_bsc = C.WALLET_BSC or _wallet_bsc

    if _wallet_sol:
        print(f"  SOL wallet: {_wallet_sol[:6]}...{_wallet_sol[-4:]}")
    if _wallet_bsc:
        print(f"  BSC wallet: {_wallet_bsc[:6]}...{_wallet_bsc[-4:]}")

    if _wallet_sol or _wallet_bsc:
        _wallet_ready = True
        return True
    return False


def get_wallet(chain: str) -> str:
    """Get the resolved wallet address for a chain."""
    if chain == "solana":
        return _wallet_sol
    elif chain == "bsc":
        return _wallet_bsc
    return ""


def get_balance(chain: str) -> float:
    """Get native token balance for a chain.

    onchainos wallet balance returns:
    {
      "data": {
        "solAddress": "...",
        "evmAddress": "...",
        "details": [
          { "tokenAssets": [ { "symbol": "SOL", "balance": "0.58", "chainIndex": "501", ... } ] }
        ]
      }
    }
    """
    # Use chain ID filter if possible, fallback to no filter
    chain_id = C.SOL_CHAIN_INDEX if chain == "solana" else C.BSC_CHAIN_INDEX if chain == "bsc" else ""
    native_sym = "SOL" if chain == "solana" else "BNB" if chain == "bsc" else ""

    # onchainos CLI can return 0.0 on first call (flaky), retry once
    for _attempt in range(2):
        result = _run_onchainos("wallet", "balance")
        data = result.get("data", {})
        if isinstance(data, dict):
            for detail in data.get("details", []):
                for asset in detail.get("tokenAssets", []):
                    sym = asset.get("symbol", "").upper()
                    asset_chain = asset.get("chainIndex", "")
                    if sym == native_sym and (not chain_id or asset_chain == chain_id):
                        bal = float(asset.get("balance", 0))
                        if bal > 0:
                            return bal
    return 0.0


# ══════════════════════════════════════════════════════════════════════
# Quick Launch — one call does everything
# ══════════════════════════════════════════════════════════════════════

def _ensure_wallet():
    """Lazy wallet preflight — runs once, caches result."""
    global _wallet_ready
    if _wallet_ready:
        return
    if not wallet_preflight():
        raise RuntimeError(
            "Wallet not logged in. Run: onchainos wallet login <your-email>"
        )
    _wallet_ready = True


def _normalize_image(image: str) -> str:
    """Accept file path, URL, or base64/data-URI. Returns a path or URL
    that ipfs.py can consume directly.

    Supported inputs:
      - "/path/to/image.png"            → returned as-is
      - "https://example.com/dog.png"   → returned as-is (ipfs handles download)
      - "data:image/png;base64,iVBOR…"  → decoded, saved to temp file
      - raw base64 string (>500 chars)  → decoded, saved to temp file
    """
    if not image:
        raise ValueError("Image is required. Provide a file path, URL, or base64 string.")

    # URL — pass through
    if image.startswith("http://") or image.startswith("https://"):
        return image

    # File path — pass through if it exists
    if os.path.exists(image):
        return image

    # Data URI: data:image/png;base64,xxxxx
    if image.startswith("data:"):
        try:
            header, b64data = image.split(",", 1)
            # Extract extension from MIME: data:image/png;base64 → png
            mime = header.split(":")[1].split(";")[0]  # image/png
            ext = mime.split("/")[1] if "/" in mime else "png"
        except (ValueError, IndexError):
            ext = "png"
            b64data = image.split(",")[-1]
        data = b64.b64decode(b64data)
        tmp = tempfile.NamedTemporaryFile(suffix=f".{ext}", delete=False, dir="/tmp")
        tmp.write(data)
        tmp.close()
        return tmp.name

    # Raw base64 (long string, not a file path)
    if len(image) > 500:
        try:
            data = b64.b64decode(image)
            tmp = tempfile.NamedTemporaryFile(suffix=".png", delete=False, dir="/tmp")
            tmp.write(data)
            tmp.close()
            return tmp.name
        except Exception:
            pass  # Not valid base64 — fall through

    # Assume file path — let downstream handle the error
    return image


async def quick_launch(
    name: str,
    symbol: str,
    description: str = "",
    image: str = "",
    launchpad: str = "pumpfun",
    buy_amount: float = 0.0,
    website: str = "",
    twitter: str = "",
    telegram: str = "",
    slippage_bps: int = 1000,
    **extras,
) -> LaunchResult:
    """One-call token launch. Handles wallet, image, IPFS, signing, broadcast.

    Args:
        name:        Token name (e.g. "MoonDog")
        symbol:      Token ticker (e.g. "MDOG")
        description: Token description
        image:       File path, URL, or base64/data-URI
        launchpad:   "pumpfun" | "bags" | "letsbonk" | "moonit" | "fourmeme" | "flap"
        buy_amount:  Bundled buy in native token (0 = create only)
        website:     Optional website URL
        twitter:     Optional Twitter URL
        telegram:    Optional Telegram URL
        slippage_bps: Slippage tolerance in basis points (default 1000 = 10%)
        **extras:    Launchpad-specific options (priority_fee, pool, buy_tax, etc.)

    Returns:
        LaunchResult with success, token_address, tx_hash, explorer_url, etc.

    Example:
        result = await quick_launch("MoonDog", "MDOG", "a good boy", "/tmp/dog.png")
        result = await quick_launch("MoonDog", "MDOG", "a good boy", "https://i.imgur.com/dog.png",
                                     buy_amount=0.1, twitter="https://twitter.com/moondog")
    """
    # ── 0. Input validation ───────────────────────────────────────────
    name = name.strip()
    symbol = symbol.strip().upper()
    if not name or len(name) > 32:
        return LaunchResult(success=False, error="Token name must be 1–32 characters.")
    if not symbol or len(symbol) > 10 or not symbol.replace("-", "").replace("_", "").isalnum():
        return LaunchResult(success=False, error="Symbol must be 1–10 alphanumeric characters.")
    if launchpad not in C.LAUNCHPAD_CHAIN:
        return LaunchResult(success=False, error=f"Unknown launchpad '{launchpad}'. Choose from: {list(C.LAUNCHPAD_CHAIN)}")
    if buy_amount < 0:
        return LaunchResult(success=False, error="buy_amount cannot be negative.")
    if not 0 <= slippage_bps <= 10000:
        return LaunchResult(success=False, error="slippage_bps must be 0–10000 (0–100%).")

    # ── 1. Auto-start dashboard ────────────────────────────────────────
    global _dashboard_started
    if not _dashboard_started:
        _start_dashboard()
        _dashboard_started = True

    # ── 2. Wallet (lazy init) ──────────────────────────────────────────
    _ensure_wallet()

    adapter = get_adapter(launchpad)
    chain = adapter.chain
    wallet = get_wallet(chain)
    native = "SOL" if chain == "solana" else "BNB"

    # ── 3. Wallet + balance checks (skipped in DRY_RUN) ──────────────
    balance = 0.0
    if C.DRY_RUN:
        wallet = wallet or f"DRY_RUN_{chain.upper()}_WALLET"
    else:
        if not wallet:
            return LaunchResult(
                success=False,
                error=f"No {chain} wallet found. Check onchainos wallet addresses.",
            )

        balance = get_balance(chain)
        buffer = C.MIN_BALANCE_BUFFER.get(chain, 0.02)
        needed = buy_amount + buffer

        if balance < needed:
            return LaunchResult(
                success=False,
                error=f"Insufficient balance: {balance:.4f} {native} < {needed:.4f} {native} needed",
            )

    # ── 4. Normalize image ─────────────────────────────────────────────
    try:
        image_path = _normalize_image(image) if image else ""
    except Exception as e:
        if not C.DRY_RUN:
            return LaunchResult(success=False, error=f"Image error: {e}")
        image_path = ""

    # ── 5. Build params ────────────────────────────────────────────────
    params = LaunchParams(
        name=name,
        symbol=symbol,
        description=description,
        image_path=image_path,
        website=website,
        twitter=twitter,
        telegram=telegram,
        launchpad=launchpad,
        buy_amount=buy_amount,
        slippage_bps=slippage_bps,
        mev_protection=C.MEV_PROTECTION,
        wallet_address=wallet,
        extras=dict(extras) if extras else {},
    )

    # ── 6. Confirmation display ────────────────────────────────────────
    est_cost = adapter.estimate_cost(params)
    mode = "DRY RUN" if C.DRY_RUN else "LIVE"
    lp_name = C.LAUNCHPAD_DISPLAY.get(launchpad, launchpad)

    print(f"\n{'─' * 54}")
    print(f"  Token Launch — {mode}")
    print(f"{'─' * 54}")
    print(f"  Name:      {name} ({symbol})")
    print(f"  Desc:      {description[:60]}{'…' if len(description) > 60 else ''}")
    print(f"  Launchpad: {lp_name} ({chain})")
    print(f"  Wallet:    {wallet[:8]}…{wallet[-4:]}")
    if not C.DRY_RUN:
        print(f"  Balance:   {balance:.4f} {native}")
    print(f"  Buy:       {buy_amount} {native}" if buy_amount > 0 else "  Buy:       create only")
    print(f"  Est. cost: ~{est_cost:.4f} {native}")
    if website:  print(f"  Website:   {website}")
    if twitter:  print(f"  Twitter:   {twitter}")
    if telegram: print(f"  Telegram:  {telegram}")
    print(f"{'─' * 54}\n")

    # ── 7. Confirmation gate (always enforced in LIVE mode) ────────────
    # Live mode always requires explicit user confirmation to prevent
    # accidental on-chain TX. DRY_RUN bypasses this gate.
    if C.CONFIRM_REQUIRED and not C.DRY_RUN:
        print("  ⚠  LIVE MODE — token creation is IRREVERSIBLE.")
        print('  Type "confirm" to proceed, anything else to abort.')
        try:
            answer = input("  > ").strip().lower()
        except EOFError:
            answer = ""
        if answer != "confirm":
            return LaunchResult(success=False, error="Launch cancelled by user.")

    # ── 8. Execute (pass cached balance to avoid double network call) ─
    return await execute_launch(params, _balance=balance)


# ══════════════════════════════════════════════════════════════════════
# Launch Execution (internal pipeline)
# ══════════════════════════════════════════════════════════════════════

async def execute_launch(params: LaunchParams, _balance: float = -1) -> LaunchResult:
    """Full launch pipeline: IPFS upload → adapter launch → save record.

    Args:
        params:   Launch parameters (wallet_address may already be set by quick_launch)
        _balance: Pre-fetched balance from quick_launch (avoids double network call).
                  Pass -1 to fetch fresh.
    """
    adapter = get_adapter(params.launchpad)
    chain = adapter.chain

    # ── DRY_RUN short-circuit: skip balance check and IPFS upload ─────
    if C.DRY_RUN:
        print(f"\n  [DRY RUN] Simulating {params.name} ({params.symbol}) on {adapter.display_name}...")
        result = await adapter.launch(params)
        _save_launch_record(params, result)
        return result

    # ── Resolve wallet (skip if already set by quick_launch) ──────────
    if not params.wallet_address:
        params.wallet_address = get_wallet(chain)
    if not params.wallet_address:
        return LaunchResult(success=False, error=f"No wallet found for {chain}")

    # ── Check balance (skip if pre-verified by quick_launch) ──────────
    balance = _balance if _balance >= 0 else get_balance(chain)
    estimated_cost = adapter.estimate_cost(params)
    buffer = C.MIN_BALANCE_BUFFER.get(chain, 0.02)

    if balance < params.buy_amount + buffer:
        return LaunchResult(
            success=False,
            error=f"Insufficient balance: {balance:.4f} < {params.buy_amount + buffer:.4f} needed",
        )

    native = "SOL" if chain == "solana" else "BNB"
    print(f"\n  Launching {params.name} ({params.symbol}) on {adapter.display_name}...")
    print(f"  Wallet: {params.wallet_address[:8]}…{params.wallet_address[-4:]} | {balance:.4f} {native}")

    # ── Upload image + metadata to IPFS ─────────────────────────────────
    print("  Step 1/3: Uploading image + metadata to IPFS...")
    try:
        ipfs_result = ipfs.upload_all(
            image_path=params.image_path,
            name=params.name,
            symbol=params.symbol,
            description=params.description,
            website=params.website,
            twitter=params.twitter,
            telegram=params.telegram,
        )
        params.image_cid = ipfs_result["image_cid"]
        params.metadata_cid = ipfs_result["metadata_cid"]
        params.metadata_uri = ipfs_result["metadata_uri"]
    except Exception as e:
        return LaunchResult(success=False, error=f"IPFS upload failed: {e}")

    # ── Execute launch ────────────────────────────────────────────────
    print("  Step 2/3: Executing launch...")
    result = await adapter.launch(params)

    # ── Save record ───────────────────────────────────────────────────
    print("  Step 3/3: Saving launch record...")
    _save_launch_record(params, result)

    # ── Lark notification (fire-and-forget) ──────────────────────────
    webhook = C.LARK_WEBHOOK or os.environ.get("LARK_WEBHOOK", "")
    if webhook and result.success:
        asyncio.create_task(_send_lark_notification(webhook, params, result))

    # ── Post-launch monitor (background thread — non-blocking) ──────
    if result.success and result.token_address and not C.DRY_RUN:
        def _bg_monitor():
            try:
                import post_launch
                post_launch.snapshot(
                    result.token_address,
                    chain=C.SOL_CHAIN_INDEX if chain == "solana" else C.BSC_CHAIN_INDEX,
                    launchpad=params.launchpad,
                    wallet=params.wallet_address,
                )
                print(f"\n  Run live monitor:")
                print(f"  python3 post_launch.py {result.token_address} --refresh 10")
            except Exception as e:
                print(f"  [Monitor] Failed to show stats: {e}")

        Thread(target=_bg_monitor, daemon=True).start()
        print("  Post-launch stats loading in background...")

    return result


def _save_launch_record(params: LaunchParams, result: LaunchResult):
    """Append launch record to state/launches.json (thread-safe, atomic)."""
    _STATE_DIR.mkdir(exist_ok=True)

    record = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "launchpad": params.launchpad,
        "chain": C.LAUNCHPAD_CHAIN.get(params.launchpad, "unknown"),
        "name": params.name,
        "symbol": params.symbol,
        "description": params.description,
        "image_cid": params.image_cid,
        "metadata_cid": params.metadata_cid,
        "metadata_uri": params.metadata_uri,
        "website": params.website,
        "twitter": params.twitter,
        "telegram": params.telegram,
        "buy_amount": params.buy_amount,
        "wallet": params.wallet_address,
        "token_address": result.token_address,
        "tx_hash": result.tx_hash,
        "explorer_url": result.explorer_url,
        "trade_page_url": result.trade_page_url,
        "success": result.success,
        "error": result.error,
    }

    with _launches_lock:
        records = []
        if _LAUNCHES.exists():
            try:
                records = json.loads(_LAUNCHES.read_text())
            except json.JSONDecodeError:
                records = []
        records.append(record)

        # Atomic write: write to temp file, then rename
        tmp_path = _LAUNCHES.with_suffix(".json.tmp")
        tmp_path.write_text(json.dumps(records, indent=2, ensure_ascii=False))
        tmp_path.rename(_LAUNCHES)

    print(f"  [State] Launch record saved ({len(records)} total)")


async def _send_lark_notification(webhook: str, params: LaunchParams, result: LaunchResult):
    """Send launch notification to Lark webhook (async-safe)."""
    try:
        import httpx
        msg = {
            "msg_type": "text",
            "content": {
                "text": (
                    f"Token Launched!\n"
                    f"Name: {params.name} ({params.symbol})\n"
                    f"Launchpad: {C.LAUNCHPAD_DISPLAY.get(params.launchpad, params.launchpad)}\n"
                    f"Token: {result.token_address}\n"
                    f"TX: {result.explorer_url}\n"
                    f"Trade: {result.trade_page_url}\n"
                    f"Buy: {params.buy_amount} {'SOL' if 'sol' in params.launchpad else 'BNB'}"
                )
            }
        }
        async with httpx.AsyncClient() as client:
            await client.post(webhook, json=msg, timeout=10)
    except Exception as e:
        print(f"  [Lark] Notification failed: {e}")


# ══════════════════════════════════════════════════════════════════════
# Dashboard
# ══════════════════════════════════════════════════════════════════════

def _start_dashboard():
    """Start the web dashboard on DASHBOARD_PORT."""
    dashboard_html = _BASE_DIR / "dashboard.html"
    if not dashboard_html.exists():
        print(f"  Dashboard HTML not found, skipping")
        return

    class Handler(SimpleHTTPRequestHandler):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, directory=str(_BASE_DIR), **kwargs)

        def do_GET(self):
            if self.path == "/" or self.path == "/index.html":
                self.path = "/dashboard.html"
            elif self.path.endswith((".py", ".json", ".yaml", ".txt", ".md", ".gitignore")):
                # Block access to source code and config files
                self.send_error(403, "Forbidden")
                return
            elif self.path == "/api/launches":
                self._json_response(_load_launches())
                return
            elif self.path == "/api/wallet":
                self._json_response({
                    "sol": {"address": _wallet_sol, "balance": get_balance("solana") if _wallet_sol else 0},
                    "bsc": {"address": _wallet_bsc, "balance": get_balance("bsc") if _wallet_bsc else 0},
                    "mode": "DRY RUN" if C.DRY_RUN else "LIVE",
                })
                return
            elif self.path.startswith("/api/token-stats?"):
                self._handle_token_stats()
                return
            return super().do_GET()

        def _handle_token_stats(self):
            """Fetch live bonding curve + price data for a token."""
            from urllib.parse import urlparse, parse_qs
            qs = parse_qs(urlparse(self.path).query)
            addr = qs.get("address", [""])[0]
            chain = qs.get("chain", ["501"])[0]
            if not addr or addr.startswith("DRY_RUN"):
                self._json_response({"bonding_pct": None, "price_usd": None, "mcap_usd": None, "holders": None})
                return
            try:
                result = _run_onchainos("memepump", "token-details", "--chain", chain, "--address", addr)
                data = result.get("data", {})
                bp = data.get("bondingPercent", "")
                mkt = data.get("market", {})
                tags = data.get("tags", {})
                self._json_response({
                    "bonding_pct": float(bp) if bp else None,
                    "price_usd": float(mkt.get("priceUsd", "0") or "0") or None,
                    "mcap_usd": float(mkt.get("marketCapUsd", "0") or "0") or None,
                    "holders": int(tags.get("totalHolders", "0") or "0"),
                    "volume_1h": float(mkt.get("volumeUsd1h", "0") or "0") or None,
                    "buy_count": int(mkt.get("buyTxCount1h", "0") or "0"),
                    "sell_count": int(mkt.get("sellTxCount1h", "0") or "0"),
                })
            except Exception:
                self._json_response({"bonding_pct": None, "price_usd": None, "mcap_usd": None, "holders": None})

        def _json_response(self, data):
            body = json.dumps(data, ensure_ascii=False).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, fmt, *args):
            pass  # Suppress request logs

    try:
        server = HTTPServer(("127.0.0.1", C.DASHBOARD_PORT), Handler)
        t = Thread(target=server.serve_forever, daemon=True)
        t.start()
        print(f"  Dashboard → http://localhost:{C.DASHBOARD_PORT}")
    except OSError as e:
        print(f"  Dashboard failed to start: {e}")


def _load_launches() -> list:
    """Load launch history from disk."""
    if _LAUNCHES.exists():
        try:
            return json.loads(_LAUNCHES.read_text())
        except json.JSONDecodeError:
            return []
    return []


# ══════════════════════════════════════════════════════════════════════
# CLI Entry Point
# ══════════════════════════════════════════════════════════════════════

def print_banner():
    mode = "DRY RUN" if C.DRY_RUN else "LIVE"
    print("=" * 60)
    print(f"  一键发币 Token Launch v1.0  [{mode}]")
    print(f"  Launchpads: pump.fun | Bags | LetsBonk | Moonit | Four.Meme | Flap")
    print(f"  Dashboard: http://localhost:{C.DASHBOARD_PORT}")
    print("=" * 60)


def main():
    global _dashboard_started
    print_banner()

    # Wallet preflight
    if not wallet_preflight():
        sys.exit(1)

    # Start dashboard
    _start_dashboard()
    _dashboard_started = True

    print("\n  Ready. One-call launch:")
    print("    from token_launch import quick_launch")
    print('    await quick_launch("Name", "SYM", "desc", "/path/to/image.png")')
    print(f"\n  Dashboard: http://localhost:{C.DASHBOARD_PORT}")
    print("  Press Ctrl+C to stop.\n")

    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        print("\n  Shutting down.")


if __name__ == "__main__":
    main()
