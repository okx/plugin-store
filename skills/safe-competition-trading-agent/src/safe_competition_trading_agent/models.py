import json
import sys
from decimal import Decimal, InvalidOperation
from typing import Any, Dict, List, Optional


SUPPORTED_CHAINS = {"xlayer", "solana"}
SUPPORTED_EXECUTION_MODES = {"dry-run", "confirm-before-execute"}
SUPPORTED_GOALS = {"safe-volume", "rank-optimizer", "eligible-token", "custom"}


class InputError(Exception):
    def __init__(self, code: str, message: str, details: Optional[List[str]] = None):
        super().__init__(message)
        self.code = code
        self.message = message
        self.details = details or []


def read_input(path: str) -> Dict[str, Any]:
    if path == "-":
        raw = sys.stdin.read()
    else:
        with open(path, "r", encoding="utf-8") as handle:
            raw = handle.read()
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise InputError("INVALID_JSON", "Input is not valid JSON.", [str(exc)])
    if not isinstance(payload, dict):
        raise InputError("INVALID_INPUT", "Input JSON must be an object.")
    return payload


def normalize_request(payload: Dict[str, Any]) -> Dict[str, Any]:
    intent = str(payload.get("intent") or "").strip()
    chain = normalize_chain(payload.get("chain") or "xlayer")
    execution_mode = _choice(payload.get("executionMode"), "dry-run")
    goal = _choice(payload.get("goal"), "safe-volume")
    budget_usd = optional_decimal(payload.get("budgetUsd", payload.get("amountUsd")))

    errors: List[str] = []
    if not intent:
        errors.append("Missing required field: intent.")
    if chain not in SUPPORTED_CHAINS:
        errors.append("Unsupported chain. Use xlayer or solana.")
    if execution_mode not in SUPPORTED_EXECUTION_MODES:
        errors.append("Unsupported executionMode. Use dry-run or confirm-before-execute.")
    if goal not in SUPPORTED_GOALS:
        errors.append("Unsupported goal. Use safe-volume, rank-optimizer, eligible-token, or custom.")
    if budget_usd is not None and budget_usd <= 0:
        errors.append("budgetUsd must be greater than zero when provided.")
    if errors:
        raise InputError("INVALID_INPUT", "Input failed required-field validation.", errors)

    plugins = payload.get("plugins") if isinstance(payload.get("plugins"), dict) else {}
    return {
        "intent": intent,
        "chain": chain,
        "goal": goal,
        "budgetUsd": budget_usd,
        "tokenIn": normalize_token(payload.get("tokenIn")) or {"symbol": "USDC"},
        "tokenOut": normalize_token(payload.get("tokenOut")),
        "executionMode": execution_mode,
        "confirmed": bool(payload.get("confirmed")),
        "wallet": _object(payload.get("wallet")),
        "competition": _object(payload.get("competition")),
        "candidates": _list_of_objects(payload.get("candidates")),
        "quote": _object(payload.get("quote")),
        "unsignedTx": _object(payload.get("unsignedTx") or payload.get("tx")),
        "riskVerdict": _object(payload.get("riskVerdict")),
        "constraints": _object(payload.get("constraints")),
        "plugins": {
            "wallet": plugins.get("wallet") or "okx-agentic-wallet",
            "competition": plugins.get("competition") or "okx-growth-competition",
            "token": plugins.get("token") or "okx-dex-token",
            "signal": plugins.get("signal") or "okx-dex-signal",
            "swap": plugins.get("swap") or "okx-dex-swap",
            "composer": plugins.get("composer") or "agent-workflow-composer",
            "risk": plugins.get("risk") or "agent-risk-firewall",
        },
    }


def normalize_chain(value: Any) -> str:
    text = str(value or "").strip().lower().replace("_", "-")
    aliases = {
        "x-layer": "xlayer",
        "x layer": "xlayer",
        "196": "xlayer",
        "sol": "solana",
        "501": "solana",
    }
    return aliases.get(text, text)


def normalize_token(value: Any) -> Optional[Dict[str, Any]]:
    if value is None:
        return None
    if isinstance(value, str):
        symbol = value.strip()
        return {"symbol": symbol} if symbol else None
    if isinstance(value, dict):
        token = dict(value)
        if "symbol" in token and isinstance(token["symbol"], str):
            token["symbol"] = token["symbol"].strip()
        return token
    return {"raw": value}


def optional_decimal(value: Any) -> Optional[Decimal]:
    if value is None or value == "":
        return None
    try:
        return Decimal(str(value))
    except (InvalidOperation, ValueError):
        return None


def decimal_to_str(value: Any) -> Any:
    if isinstance(value, Decimal):
        return str(value)
    return value


def _choice(value: Any, default: str) -> str:
    return str(value or default).strip().lower().replace("_", "-")


def _object(value: Any) -> Dict[str, Any]:
    return value if isinstance(value, dict) else {}


def _list_of_objects(value: Any) -> List[Dict[str, Any]]:
    if not isinstance(value, list):
        return []
    return [item for item in value if isinstance(item, dict)]
