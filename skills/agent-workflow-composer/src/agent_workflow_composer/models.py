import json
import sys
from decimal import Decimal, InvalidOperation
from typing import Any, Dict, List, Optional


SUPPORTED_WORKFLOW_TYPES = {
    "swap",
    "approval",
    "competition-trade",
    "custom",
}
SUPPORTED_EXECUTION_MODES = {
    "dry-run",
    "confirm-before-execute",
}
SUPPORTED_RISK_PROFILES = {
    "balanced",
    "strict",
    "competition",
    "degen-small-size",
}


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
    workflow_type = _normalize_choice(payload.get("workflowType") or payload.get("type"), "swap")
    execution_mode = _normalize_choice(payload.get("executionMode"), "dry-run")
    risk_profile = _normalize_choice(payload.get("riskProfile") or payload.get("policyProfile"), "balanced")

    errors: List[str] = []
    if not intent:
        errors.append("Missing required field: intent.")
    if workflow_type not in SUPPORTED_WORKFLOW_TYPES:
        errors.append("Unsupported workflowType. Use swap, approval, competition-trade, or custom.")
    if execution_mode not in SUPPORTED_EXECUTION_MODES:
        errors.append("Unsupported executionMode. Use dry-run or confirm-before-execute.")
    if risk_profile not in SUPPORTED_RISK_PROFILES:
        errors.append("Unsupported riskProfile. Use balanced, strict, competition, or degen-small-size.")
    if errors:
        raise InputError("INVALID_INPUT", "Input failed required-field validation.", errors)

    return {
        "intent": intent,
        "workflowType": workflow_type,
        "chain": _normalize_text(payload.get("chain")),
        "tokenIn": _normalize_token(payload.get("tokenIn")),
        "tokenOut": _normalize_token(payload.get("tokenOut")),
        "amountUsd": optional_decimal(payload.get("amountUsd")),
        "executionMode": execution_mode,
        "riskProfile": risk_profile,
        "plugins": payload.get("plugins") if isinstance(payload.get("plugins"), dict) else {},
        "constraints": payload.get("constraints") if isinstance(payload.get("constraints"), dict) else {},
        "competition": payload.get("competition") if isinstance(payload.get("competition"), dict) else {},
        "externalEvidencePlugins": _normalize_list(payload.get("externalEvidencePlugins")),
    }


def optional_decimal(value: Any) -> Optional[Decimal]:
    if value is None or value == "":
        return None
    try:
        return Decimal(str(value))
    except (InvalidOperation, ValueError):
        return None


def _normalize_choice(value: Any, default: str) -> str:
    return str(value or default).strip().lower().replace("_", "-")


def _normalize_text(value: Any) -> Optional[str]:
    if value is None:
        return None
    text = str(value).strip()
    return text or None


def _normalize_token(value: Any) -> Optional[Dict[str, Any]]:
    if value is None:
        return None
    if isinstance(value, str):
        return {"symbol": value.strip()}
    if isinstance(value, dict):
        return dict(value)
    return {"raw": value}


def _normalize_list(value: Any) -> List[str]:
    if value is None:
        return []
    if isinstance(value, str):
        return [part.strip() for part in value.split(",") if part.strip()]
    if isinstance(value, list):
        return [str(part).strip() for part in value if str(part).strip()]
    return []
