from typing import Any, Dict, List

from .models import decimal_to_str


def build_firewall_input(request: Dict[str, Any], candidate: Dict[str, Any]) -> Dict[str, Any]:
    wallet = request.get("wallet") or {}
    wallet_address = wallet.get("address") or wallet.get("walletAddress") or "<walletAddress>"
    return {
        "chain": request["chain"],
        "operation": "swap",
        "walletAddress": wallet_address,
        "tokenIn": request.get("tokenIn") or {"symbol": "USDC"},
        "tokenOut": _token_for_firewall(candidate),
        "amountIn": decimal_to_str(request.get("budgetUsd")),
        "amountInUsd": decimal_to_str(request.get("budgetUsd")),
        "quote": request.get("quote") or {},
        "tx": request.get("unsignedTx") or {},
        "competition": request.get("competition") or {},
        "policyProfile": "competition",
    }


def risk_gate(request: Dict[str, Any]) -> Dict[str, Any]:
    verdict = request.get("riskVerdict") or {}
    value = str(verdict.get("verdict") or "unknown").lower()
    reasons = verdict.get("reasons") if isinstance(verdict.get("reasons"), list) else []
    confirmed = bool(request.get("confirmed"))

    if value == "block":
        return {
            "okToExecute": False,
            "state": "blocked",
            "reasons": _reason_codes(reasons) or ["Firewall returned block."],
        }
    if value == "warn":
        return {
            "okToExecute": confirmed,
            "state": "confirmed_warn" if confirmed else "needs_explicit_confirmation",
            "reasons": _reason_codes(reasons) or ["Firewall returned warn."],
        }
    if value == "allow":
        return {
            "okToExecute": confirmed,
            "state": "confirmed_allow" if confirmed else "ready_for_confirmation",
            "reasons": [],
        }
    return {
        "okToExecute": False,
        "state": "risk_verdict_missing",
        "reasons": ["Run agent-risk-firewall check before execution."],
    }


def _token_for_firewall(candidate: Dict[str, Any]) -> Dict[str, Any]:
    token = dict(candidate)
    token.pop("selectionReasons", None)
    token.pop("selectionScore", None)
    return token


def _reason_codes(reasons: List[Any]) -> List[str]:
    output = []
    for reason in reasons:
        if isinstance(reason, dict):
            output.append(str(reason.get("code") or reason.get("message") or reason))
        else:
            output.append(str(reason))
    return output
