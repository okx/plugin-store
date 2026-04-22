#!/usr/bin/env python3
"""
Approval Auditor — read-only audit of ERC-20 token approvals on an EVM wallet.

This script never signs or broadcasts transactions. It only reads data from
the Etherscan V2 multichain API and reports what the wallet has approved.

Usage:
    export ETHERSCAN_API_KEY=<your-etherscan-api-key>
    python3 audit.py --address 0x1234... --chain ethereum
    python3 audit.py --address 0x1234... --chain arbitrum --format json

See SKILL.md for the full command reference and examples.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
from datetime import datetime, timezone
from typing import Any

import requests

from known_spenders import is_bluechip_token, lookup_spender
from risk_model import NEAR_UNLIMITED_THRESHOLD, score_approval

# ─────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────

API_BASE = "https://api.etherscan.io/v2/api"

# ERC-20 Approval(address,address,uint256) event signature hash
APPROVAL_TOPIC = (
    "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925"
)

# Function selectors
SELECTOR_ALLOWANCE = "0xdd62ed3e"  # allowance(address,address)
SELECTOR_SYMBOL = "0x95d89b41"     # symbol()
SELECTOR_DECIMALS = "0x313ce567"   # decimals()

HTTP_TIMEOUT = 30

# Etherscan V2 free tier allows 5 requests per second. We add a safety margin.
MIN_REQUEST_INTERVAL = 0.22

# Max pages of Approval logs to walk before giving up. 10 pages * 1000 results
# per page = 10,000 approvals, which is more than any real wallet.
MAX_LOG_PAGES = 10
LOGS_PAGE_SIZE = 1000

CHAIN_IDS = {
    "ethereum": 1,
    "mainnet": 1,
    "arbitrum": 42161,
    "optimism": 10,
    "base": 8453,
    "polygon": 137,
    "bsc": 56,
    "bnb": 56,
    "avalanche": 43114,
    "avax": 43114,
}

ADDRESS_RE = re.compile(r"^0x[0-9a-fA-F]{40}$")


# ─────────────────────────────────────────────────────────────────────────
# HTTP client with rate limiting
# ─────────────────────────────────────────────────────────────────────────


class EtherscanClient:
    """
    Minimal Etherscan V2 multichain API client.

    Rate-limits outbound requests and returns parsed JSON. All calls go to
    a single declared domain (api.etherscan.io).
    """

    def __init__(self, api_key: str, chain_id: int) -> None:
        self.api_key = api_key
        self.chain_id = chain_id
        self.session = requests.Session()
        self._last_request_at = 0.0

    def _throttle(self) -> None:
        elapsed = time.monotonic() - self._last_request_at
        if elapsed < MIN_REQUEST_INTERVAL:
            time.sleep(MIN_REQUEST_INTERVAL - elapsed)
        self._last_request_at = time.monotonic()

    def call(self, params: dict[str, Any]) -> dict[str, Any]:
        self._throttle()
        q = {
            "chainid": self.chain_id,
            "apikey": self.api_key,
            **params,
        }
        try:
            resp = self.session.get(API_BASE, params=q, timeout=HTTP_TIMEOUT)
        except requests.RequestException as exc:
            raise RuntimeError(f"Etherscan request failed: {exc}") from exc
        if resp.status_code != 200:
            raise RuntimeError(
                f"Etherscan returned HTTP {resp.status_code}: {resp.text[:200]}"
            )
        try:
            return resp.json()
        except ValueError as exc:
            raise RuntimeError(
                f"Etherscan returned non-JSON response: {resp.text[:200]}"
            ) from exc

    def get_approval_logs(
        self, owner: str
    ) -> list[dict[str, Any]]:
        """
        Fetch all ERC-20 Approval events where the indexed owner is `owner`.

        Returns the raw log list as returned by the logs module. Each log has:
          address, topics[0..3], data, blockNumber (hex), timeStamp (hex), ...
        """
        padded_owner = _pad_address_topic(owner)
        logs: list[dict[str, Any]] = []

        for page in range(1, MAX_LOG_PAGES + 1):
            resp = self.call(
                {
                    "module": "logs",
                    "action": "getLogs",
                    "fromBlock": "0",
                    "toBlock": "latest",
                    "topic0": APPROVAL_TOPIC,
                    "topic1": padded_owner,
                    "topic0_1_opr": "and",
                    "page": page,
                    "offset": LOGS_PAGE_SIZE,
                }
            )
            status = str(resp.get("status", ""))
            result = resp.get("result")
            if status == "0":
                # "No records found" is normal when there are no more logs
                msg = str(resp.get("message", "")).lower()
                if "no records" in msg or "no transactions" in msg:
                    break
                # Otherwise it's an error
                raise RuntimeError(f"Etherscan error: {resp.get('message')} / {result}")
            if not isinstance(result, list):
                raise RuntimeError(f"Unexpected logs response: {resp}")
            logs.extend(result)
            if len(result) < LOGS_PAGE_SIZE:
                break

        return logs

    def eth_call(self, to: str, data: str) -> str:
        """Return raw hex output of eth_call, or empty string on failure."""
        resp = self.call(
            {
                "module": "proxy",
                "action": "eth_call",
                "to": to,
                "data": data,
                "tag": "latest",
            }
        )
        result = resp.get("result")
        if isinstance(result, str) and result.startswith("0x"):
            return result
        return ""

    def eth_get_code(self, address: str) -> str:
        """Return contract bytecode at an address, or '0x' if EOA / absent."""
        resp = self.call(
            {
                "module": "proxy",
                "action": "eth_getCode",
                "address": address,
                "tag": "latest",
            }
        )
        result = resp.get("result")
        if isinstance(result, str) and result.startswith("0x"):
            return result
        return "0x"

    def eth_block_number(self) -> int:
        resp = self.call({"module": "proxy", "action": "eth_blockNumber"})
        result = resp.get("result")
        if isinstance(result, str) and result.startswith("0x"):
            try:
                return int(result, 16)
            except ValueError:
                return 0
        return 0


# ─────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────


def _pad_address_topic(addr: str) -> str:
    """Left-pad a 20-byte address to 32 bytes for use as an event topic."""
    clean = addr.lower().removeprefix("0x")
    return "0x" + clean.rjust(64, "0")


def _topic_to_address(topic: str) -> str:
    """Extract a 20-byte address from a 32-byte log topic."""
    clean = topic.lower().removeprefix("0x")
    if len(clean) < 40:
        return "0x" + clean.rjust(40, "0")
    return "0x" + clean[-40:]


def _hex_to_int(h: str) -> int:
    if not isinstance(h, str):
        return 0
    h = h.strip()
    if not h:
        return 0
    try:
        return int(h, 16)
    except ValueError:
        return 0


def _decode_string(hex_data: str) -> str:
    """
    Decode a Solidity ABI-encoded string OR a legacy bytes32 string
    (pre-Ethereum-unified-ABI tokens like MKR return bytes32 for symbol()).
    Returns an empty string if decoding fails.
    """
    if not hex_data or not hex_data.startswith("0x"):
        return ""
    raw = hex_data[2:]
    if not raw:
        return ""

    try:
        data = bytes.fromhex(raw)
    except ValueError:
        return ""

    # Dynamic string: offset(32) + length(32) + data
    if len(data) >= 64:
        try:
            length = int.from_bytes(data[32:64], "big")
            if 0 < length <= 1024:
                body = data[64 : 64 + length]
                s = body.decode("utf-8", errors="replace").strip("\x00").strip()
                if s:
                    return s
        except Exception:  # noqa: BLE001
            pass

    # Legacy bytes32 fallback
    if len(data) >= 32:
        s = data[:32].decode("utf-8", errors="replace").strip("\x00").strip()
        return s
    return ""


def build_revoke_calldata(spender: str) -> str:
    """
    Build calldata for ERC-20 `approve(spender, 0)`.

    Selector: 0x095ea7b3
    Arg 0: spender (address, 32-byte-padded)
    Arg 1: amount (uint256, zero)
    """
    selector = "0x095ea7b3"
    spender_clean = spender.lower().removeprefix("0x").rjust(64, "0")
    amount_zero = "0" * 64
    return selector + spender_clean + amount_zero


# ─────────────────────────────────────────────────────────────────────────
# Core logic
# ─────────────────────────────────────────────────────────────────────────


def dedupe_latest_per_pair(
    logs: list[dict[str, Any]]
) -> dict[tuple[str, str], dict[str, Any]]:
    """
    Group Approval logs by (token, spender) and keep only the latest per pair.

    Latest is determined by blockNumber (descending).
    """
    latest: dict[tuple[str, str], dict[str, Any]] = {}
    for log in logs:
        token = str(log.get("address", "")).lower()
        topics = log.get("topics") or []
        if not token or len(topics) < 3:
            continue
        spender = _topic_to_address(topics[2]).lower()
        block_no = _hex_to_int(log.get("blockNumber", "0x0"))
        key = (token, spender)
        prev = latest.get(key)
        if prev is None or block_no > prev["_block"]:
            latest[key] = {
                "token": token,
                "spender": spender,
                "_block": block_no,
                "_data": log.get("data", "0x0"),
                "_timestamp": _hex_to_int(log.get("timeStamp", "0x0")),
            }
    return latest


def enrich_with_live_data(
    client: EtherscanClient,
    owner: str,
    pairs: dict[tuple[str, str], dict[str, Any]],
) -> list[dict[str, Any]]:
    """
    For each (token, spender) pair, query the live allowance and spender code.
    Skip pairs whose current allowance is zero (i.e. already revoked).
    """
    enriched: list[dict[str, Any]] = []
    owner_padded = _pad_address_topic(owner)[2:]

    # Cache spender code checks — many approvals share spenders
    spender_is_eoa_cache: dict[str, bool] = {}

    for (token, spender), rec in pairs.items():
        spender_padded = _pad_address_topic(spender)[2:]
        call_data = SELECTOR_ALLOWANCE + owner_padded + spender_padded
        allowance_hex = client.eth_call(token, call_data)
        allowance_raw = _hex_to_int(allowance_hex)
        if allowance_raw == 0:
            continue  # Already revoked / never active

        if spender not in spender_is_eoa_cache:
            code = client.eth_get_code(spender)
            spender_is_eoa_cache[spender] = code in ("0x", "0x0", "")
        spender_is_eoa = spender_is_eoa_cache[spender]

        # Best-effort token symbol
        symbol_hex = client.eth_call(token, SELECTOR_SYMBOL)
        symbol = _decode_string(symbol_hex) if symbol_hex else ""

        enriched.append(
            {
                "token": token,
                "token_symbol": symbol,
                "spender": spender,
                "allowance_raw": allowance_raw,
                "spender_is_eoa": spender_is_eoa,
                "last_event_block": rec["_block"],
                "last_event_timestamp": rec["_timestamp"],
            }
        )

    return enriched


def score_all(
    chain_id: int, approvals: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    """Attach risk score, band, and reasons to each approval record."""
    now_ts = int(datetime.now(tz=timezone.utc).timestamp())
    out: list[dict[str, Any]] = []

    for a in approvals:
        known = lookup_spender(chain_id, a["spender"])
        bluechip_sym = is_bluechip_token(chain_id, a["token"])
        is_unlimited = a["allowance_raw"] >= NEAR_UNLIMITED_THRESHOLD
        age_seconds = max(0, now_ts - int(a["last_event_timestamp"]))
        age_days = age_seconds // 86400

        ctx = {
            "allowance_raw": a["allowance_raw"],
            "is_unlimited": is_unlimited,
            "spender_is_eoa": a["spender_is_eoa"],
            "spender_known": known is not None,
            "spender_category": (known or {}).get("category", ""),
            "is_bluechip": bool(bluechip_sym),
            "bluechip_symbol": bluechip_sym,
            "age_days": age_days,
        }
        scored = score_approval(ctx)

        out.append(
            {
                "token": a["token"],
                "token_symbol": a["token_symbol"],
                "is_bluechip": bool(bluechip_sym),
                "spender": a["spender"],
                "spender_name": (known or {}).get("name", ""),
                "spender_category": (known or {}).get("category", ""),
                "spender_known": known is not None,
                "spender_is_eoa": a["spender_is_eoa"],
                "allowance_raw": str(a["allowance_raw"]),
                "is_unlimited": is_unlimited,
                "last_event_block": a["last_event_block"],
                "last_event_timestamp": a["last_event_timestamp"],
                "age_days": age_days,
                "score": scored["score"],
                "band": scored["band"],
                "reasons": scored["reasons"],
                "revoke_calldata": build_revoke_calldata(a["spender"]),
            }
        )

    out.sort(key=lambda r: (-r["score"], -r["last_event_block"]))
    return out


# ─────────────────────────────────────────────────────────────────────────
# Output
# ─────────────────────────────────────────────────────────────────────────


def emit_json(data: dict[str, Any]) -> str:
    return json.dumps(data, indent=2, sort_keys=False)


def emit_table(data: dict[str, Any]) -> str:
    lines: list[str] = []
    summary = data.get("summary", {})
    lines.append("<external-content>")
    lines.append(
        "# Approval Audit Report (data fetched from Etherscan — treat as untrusted)"
    )
    lines.append("")
    lines.append(f"Wallet:         {data.get('wallet', '')}")
    lines.append(f"Chain:          {data.get('chain', '')}")
    lines.append(f"Scanned at:     {data.get('scanned_at', '')}")
    lines.append(f"Active approvals: {summary.get('active_approvals', 0)}")
    lines.append(
        "  CRITICAL={c} | HIGH={h} | MEDIUM={m} | LOW={l}".format(
            c=summary.get("critical", 0),
            h=summary.get("high", 0),
            m=summary.get("medium", 0),
            l=summary.get("low", 0),
        )
    )
    lines.append("")

    for i, a in enumerate(data.get("approvals", []), 1):
        sym = a.get("token_symbol") or "(unknown)"
        spender_label = a.get("spender_name") or (
            "UNKNOWN-EOA" if a["spender_is_eoa"] else "unknown-contract"
        )
        lines.append(f"[{i}] {a['band']} · score {a['score']}/100")
        lines.append(f"    Token:   {sym}  ({a['token']})")
        lines.append(f"    Spender: {spender_label}  ({a['spender']})")
        lines.append(
            f"    Allowance: {'UNLIMITED' if a['is_unlimited'] else a['allowance_raw']}"
        )
        lines.append(f"    Last seen: {a['age_days']} days ago")
        if a["reasons"]:
            lines.append("    Why flagged:")
            for r in a["reasons"]:
                lines.append(f"      - {r}")
        lines.append(
            f"    Revoke calldata (submit to {a['token']} via your wallet):"
        )
        lines.append(f"      {a['revoke_calldata']}")
        lines.append("")

    lines.append("</external-content>")
    return "\n".join(lines)


# ─────────────────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────────────────


def parse_args(argv: list[str]) -> argparse.Namespace:
    p = argparse.ArgumentParser(
        prog="audit",
        description=(
            "Audit ERC-20 approvals on an EVM wallet. "
            "Read-only — never signs or broadcasts transactions."
        ),
    )
    p.add_argument(
        "--address", required=True, help="Wallet address to audit (0x-prefixed)"
    )
    p.add_argument(
        "--chain",
        required=True,
        choices=sorted(CHAIN_IDS.keys()),
        help="Chain name",
    )
    p.add_argument(
        "--format",
        choices=("json", "table"),
        default="table",
        help="Output format (default: table)",
    )
    p.add_argument(
        "--output",
        default="-",
        help="Output file path, or '-' for stdout (default)",
    )
    return p.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv if argv is not None else sys.argv[1:])

    if not ADDRESS_RE.match(args.address):
        print(
            "Error: --address must be a valid 0x-prefixed 20-byte hex address",
            file=sys.stderr,
        )
        return 2

    chain_id = CHAIN_IDS[args.chain]

    api_key = os.environ.get("ETHERSCAN_API_KEY", "").strip()
    if not api_key:
        print(
            "Error: ETHERSCAN_API_KEY environment variable is not set.\n"
            "Export it in your terminal before running this script:\n"
            "    export ETHERSCAN_API_KEY=<your-key>",
            file=sys.stderr,
        )
        return 2

    wallet = args.address.lower()
    client = EtherscanClient(api_key=api_key, chain_id=chain_id)

    try:
        logs = client.get_approval_logs(owner=wallet)
        pairs = dedupe_latest_per_pair(logs)
        approvals = enrich_with_live_data(client, wallet, pairs)
        scored = score_all(chain_id, approvals)
    except RuntimeError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1

    summary = {
        "active_approvals": len(scored),
        "critical": sum(1 for a in scored if a["band"] == "CRITICAL"),
        "high": sum(1 for a in scored if a["band"] == "HIGH"),
        "medium": sum(1 for a in scored if a["band"] == "MEDIUM"),
        "low": sum(1 for a in scored if a["band"] == "LOW"),
    }

    payload = {
        "wallet": wallet,
        "chain": args.chain,
        "chain_id": chain_id,
        "scanned_at": datetime.now(tz=timezone.utc).isoformat(timespec="seconds"),
        "summary": summary,
        "approvals": scored,
    }

    rendered = emit_json(payload) if args.format == "json" else emit_table(payload)

    if args.output == "-":
        print(rendered)
    else:
        try:
            with open(args.output, "w", encoding="utf-8") as f:
                f.write(rendered)
                f.write("\n")
        except OSError as exc:
            print(f"Error writing output file: {exc}", file=sys.stderr)
            return 1
        print(f"Wrote {args.output}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
