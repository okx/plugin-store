import hashlib
import json
from decimal import Decimal
from typing import Any, Dict, Iterable, List, Optional

from .models import target_token


VERDICT_PRIORITY = {"allow": 0, "warn": 1, "block": 2}
POLICY_VERSION = "1.1.0"
MAX_UINT256 = Decimal(2) ** 256 - Decimal(1)


BALANCED_POLICY = {
    "profile": "balanced",
    "policyVersion": POLICY_VERSION,
    "supportedChains": ["xlayer", "solana"],
    "maxTradeUsd": 250,
    "maxWalletPct": 10,
    "slippageWarnPct": 2,
    "slippageBlockPct": 5,
    "priceImpactWarnPct": 3,
    "priceImpactBlockPct": 8,
    "lowLiquidityUsd": 10000,
    "tokenHighSeverity": "warn",
    "tokenMediumSeverity": "warn",
    "scanUnavailableSeverity": "warn",
    "txScanUnavailableSeverity": "warn",
    "simulationUnavailableSeverity": "warn",
    "unlimitedApprovalSeverity": "warn",
    "spenderEoaSeverity": "warn",
    "unknownSpenderSeverity": "warn",
    "highTaxWarnPct": 25,
    "highTaxBlockPct": 50,
    "topHolderWarnPct": 70,
    "topHolderBlockPct": 90,
    "disallowStableNativeOnlyPair": False,
}

STRICT_POLICY = dict(
    BALANCED_POLICY,
    profile="strict",
    maxTradeUsd=100,
    maxWalletPct=5,
    slippageWarnPct=1,
    slippageBlockPct=3,
    priceImpactWarnPct=2,
    priceImpactBlockPct=5,
    lowLiquidityUsd=50000,
    tokenHighSeverity="block",
    scanUnavailableSeverity="block",
    txScanUnavailableSeverity="block",
    simulationUnavailableSeverity="block",
    spenderEoaSeverity="block",
    unknownSpenderSeverity="block",
    highTaxWarnPct=15,
    highTaxBlockPct=35,
    topHolderWarnPct=50,
    topHolderBlockPct=75,
)

COMPETITION_POLICY = dict(
    BALANCED_POLICY,
    profile="competition",
    maxTradeUsd=100,
    maxWalletPct=10,
    slippageWarnPct=1.5,
    slippageBlockPct=4,
    priceImpactWarnPct=2.5,
    priceImpactBlockPct=6,
    lowLiquidityUsd=25000,
    disallowStableNativeOnlyPair=True,
)

DEGEN_SMALL_SIZE_POLICY = dict(
    BALANCED_POLICY,
    profile="degen-small-size",
    maxTradeUsd=25,
    maxWalletPct=3,
    slippageWarnPct=5,
    slippageBlockPct=12,
    priceImpactWarnPct=8,
    priceImpactBlockPct=20,
    lowLiquidityUsd=2000,
    highTaxWarnPct=35,
    highTaxBlockPct=65,
)

POLICIES = {
    "balanced": BALANCED_POLICY,
    "strict": STRICT_POLICY,
    "competition": COMPETITION_POLICY,
    "degen-small-size": DEGEN_SMALL_SIZE_POLICY,
}


def get_policy(profile: str = "balanced") -> Dict[str, Any]:
    normalized = str(profile or "balanced").strip().lower()
    if normalized not in POLICIES:
        return dict(BALANCED_POLICY, requestedProfile=normalized, warning="Unknown profile; using balanced.")
    return dict(POLICIES[normalized])


def evaluate(
    context: Dict[str, Any],
    evidence: Dict[str, Any],
    validation_findings: Optional[List[Dict[str, Any]]] = None,
    profile: str = "balanced",
) -> Dict[str, Any]:
    policy = get_policy(profile)
    evidence_for_output = dict(evidence)
    if context.get("externalEvidence"):
        evidence_for_output["externalEvidence"] = context.get("externalEvidence")

    reasons: List[Dict[str, Any]] = []
    for finding in validation_findings or []:
        _add_reason(reasons, finding)

    _evaluate_profile_rules(context, policy, reasons)
    _evaluate_amount_caps(context, policy, reasons)
    _evaluate_quote(context, policy, reasons)
    _evaluate_token_scan(context, evidence.get("tokenScan") or {}, policy, reasons)
    _evaluate_token_report(evidence.get("tokenReport") or {}, policy, reasons)
    _evaluate_tx_scan(context, evidence.get("txScan") or {}, policy, reasons)
    _evaluate_simulation(context, evidence.get("simulation") or {}, policy, reasons)
    _evaluate_approval(context, policy, reasons)
    _evaluate_external_evidence(context, policy, reasons)

    verdict = _highest_verdict(reasons)
    risk_score = _risk_score(reasons, verdict)
    return {
        "verdict": verdict,
        "riskScore": risk_score,
        "requiresUserConfirmation": verdict == "warn",
        "reasons": _public_reasons(reasons),
        "evidence": evidence_for_output,
        "audit": _audit_trail(context, evidence_for_output, policy, verdict, reasons),
        "safeNextStep": _safe_next_step(verdict),
    }


def _evaluate_profile_rules(context: Dict[str, Any], policy: Dict[str, Any], reasons: List[Dict[str, Any]]) -> None:
    if not policy.get("disallowStableNativeOnlyPair"):
        return
    token_in = context.get("tokenIn") or {}
    token_out = context.get("tokenOut") or {}
    if _is_stable_or_native(token_in) and _is_stable_or_native(token_out):
        _add_reason(
            reasons,
            {
                "code": "COMPETITION_PAIR_NOT_ELIGIBLE",
                "severity": "block",
                "score": 87,
                "message": "Competition profile blocks stablecoin/native-only pairs; use a real token trade.",
            },
        )


def _evaluate_amount_caps(context: Dict[str, Any], policy: Dict[str, Any], reasons: List[Dict[str, Any]]) -> None:
    amount_usd = context.get("amountInUsd")
    wallet_value_usd = context.get("walletValueUsd")
    if isinstance(amount_usd, Decimal):
        if amount_usd > Decimal(str(policy["maxTradeUsd"])):
            _add_reason(
                reasons,
                {
                    "code": "TRADE_CAP_EXCEEDED",
                    "severity": "block",
                    "score": 88,
                    "message": "Trade amount exceeds the active policy per-trade cap.",
                },
            )
        if isinstance(wallet_value_usd, Decimal) and wallet_value_usd > 0:
            pct = (amount_usd / wallet_value_usd) * Decimal("100")
            if pct > Decimal(str(policy["maxWalletPct"])):
                _add_reason(
                    reasons,
                    {
                        "code": "WALLET_EXPOSURE_HIGH",
                        "severity": "block",
                        "score": 86,
                        "message": "Trade amount exceeds 10% of wallet value.",
                    },
                )


def _evaluate_quote(context: Dict[str, Any], policy: Dict[str, Any], reasons: List[Dict[str, Any]]) -> None:
    quote = context.get("quote") or {}
    slippage = _to_decimal(quote.get("slippagePct"))
    price_impact = _to_decimal(quote.get("priceImpactPct"))

    if slippage is not None:
        if slippage > Decimal(str(policy["slippageBlockPct"])):
            _add_reason(
                reasons,
                {
                    "code": "SLIPPAGE_HIGH",
                    "severity": "block",
                    "score": 84,
                    "message": "Quoted slippage is above the active policy block threshold.",
                },
            )
        elif slippage >= Decimal(str(policy["slippageWarnPct"])):
            _add_reason(
                reasons,
                {
                    "code": "SLIPPAGE_ELEVATED",
                    "severity": "warn",
                    "score": 58,
                    "message": "Quoted slippage is elevated.",
                },
            )

    if price_impact is not None:
        if price_impact > Decimal(str(policy["priceImpactBlockPct"])):
            _add_reason(
                reasons,
                {
                    "code": "PRICE_IMPACT_HIGH",
                    "severity": "block",
                    "score": 85,
                    "message": "Quoted price impact is above the active policy block threshold.",
                },
            )
        elif price_impact >= Decimal(str(policy["priceImpactWarnPct"])):
            _add_reason(
                reasons,
                {
                    "code": "PRICE_IMPACT_ELEVATED",
                    "severity": "warn",
                    "score": 57,
                    "message": "Quoted price impact is elevated.",
                },
            )


def _evaluate_token_scan(
    context: Dict[str, Any],
    token_scan: Dict[str, Any],
    policy: Dict[str, Any],
    reasons: List[Dict[str, Any]],
) -> None:
    token = target_token(context)
    token_needs_scan = bool(isinstance(token, dict) and token.get("address"))
    status = token_scan.get("status")
    data = token_scan.get("data") if isinstance(token_scan.get("data"), dict) else token_scan
    risk_level = str(_dig(data, ["riskLevel", "risk_level", "level"]) or "").upper()
    operation = context.get("operation")

    if token_needs_scan and status in ("unavailable", "timeout", "error"):
        severity = str(policy.get("scanUnavailableSeverity", "warn"))
        _add_reason(
            reasons,
            {
                "code": "SCAN_UNAVAILABLE" if status != "timeout" else "SCAN_TIMEOUT",
                "severity": severity,
                "score": 62 if severity == "warn" else 82,
                "message": "Token security scan did not complete; verification is incomplete.",
            },
        )
        return

    if not risk_level:
        return

    if risk_level == "CRITICAL":
        if operation == "sell":
            _add_reason(
                reasons,
                {
                    "code": "TOKEN_CRITICAL_SELL",
                    "severity": "warn",
                    "score": 70,
                    "message": "Target token has CRITICAL risk; selling may be an exit but requires confirmation.",
                },
            )
        else:
            _add_reason(
                reasons,
                {
                    "code": "TOKEN_CRITICAL",
                    "severity": "block",
                    "score": 96,
                    "message": "Target token has CRITICAL risk.",
                },
            )
    elif risk_level == "HIGH":
        severity = str(policy.get("tokenHighSeverity", "warn"))
        _add_reason(
            reasons,
            {
                "code": "TOKEN_HIGH",
                "severity": severity,
                "score": 68 if severity == "warn" else 86,
                "message": "Target token has HIGH risk.",
            },
        )
    elif risk_level == "MEDIUM":
        severity = str(policy.get("tokenMediumSeverity", "warn"))
        _add_reason(
            reasons,
            {
                "code": "TOKEN_MEDIUM",
                "severity": severity,
                "score": 55 if severity == "warn" else 82,
                "message": "Target token has MEDIUM risk.",
            },
        )


def _evaluate_token_report(token_report: Dict[str, Any], policy: Dict[str, Any], reasons: List[Dict[str, Any]]) -> None:
    if token_report.get("status") != "ok":
        return
    liquidity = _find_numeric(token_report.get("data"), ("liquidityUsd", "liquidityUSD", "liquidity", "totalLiquidity"))
    if liquidity is not None and liquidity < Decimal(str(policy["lowLiquidityUsd"])):
        _add_reason(
            reasons,
            {
                "code": "LIQUIDITY_LOW",
                "severity": "warn",
                "score": 56,
                "message": "Token liquidity appears low for a retail swap.",
            },
        )


def _evaluate_tx_scan(
    context: Dict[str, Any],
    tx_scan: Dict[str, Any],
    policy: Dict[str, Any],
    reasons: List[Dict[str, Any]],
) -> None:
    if not _has_tx(context):
        return
    status = tx_scan.get("status")
    data = tx_scan.get("data") if isinstance(tx_scan.get("data"), dict) else tx_scan
    action = str(_dig(data, ["action", "riskAction", "verdict"]) or "").lower()

    if status in ("unavailable", "timeout", "error"):
        severity = str(policy.get("txScanUnavailableSeverity", "warn"))
        _add_reason(
            reasons,
            {
                "code": "TX_SCAN_UNAVAILABLE" if status != "timeout" else "TX_SCAN_TIMEOUT",
                "severity": severity,
                "score": 64 if severity == "warn" else 84,
                "message": "Transaction security scan did not complete; verification is incomplete.",
            },
        )
        return

    if action == "block":
        _add_reason(
            reasons,
            {
                "code": "TX_SCAN_BLOCK",
                "severity": "block",
                "score": 98,
                "message": "OKX transaction scan returned a block action.",
            },
        )
    elif action == "warn":
        _add_reason(
            reasons,
            {
                "code": "TX_SCAN_WARN",
                "severity": "warn",
                "score": 72,
                "message": "OKX transaction scan returned a warning.",
            },
        )


def _evaluate_simulation(
    context: Dict[str, Any],
    simulation: Dict[str, Any],
    policy: Dict[str, Any],
    reasons: List[Dict[str, Any]],
) -> None:
    if not _has_tx(context):
        return
    status = simulation.get("status")
    data = simulation.get("data") if isinstance(simulation.get("data"), dict) else simulation

    if status in ("unavailable", "timeout", "error"):
        severity = str(policy.get("simulationUnavailableSeverity", "warn"))
        _add_reason(
            reasons,
            {
                "code": "SIMULATION_UNAVAILABLE" if status != "timeout" else "SIMULATION_TIMEOUT",
                "severity": severity,
                "score": 63 if severity == "warn" else 83,
                "message": "Transaction simulation did not complete; verification is incomplete.",
            },
        )
        return

    if _simulation_reverted(data):
        _add_reason(
            reasons,
            {
                "code": "SIMULATION_REVERT",
                "severity": "block",
                "score": 94,
                "message": "Transaction simulation indicates the transaction may revert or fail.",
            },
        )


def _evaluate_approval(context: Dict[str, Any], policy: Dict[str, Any], reasons: List[Dict[str, Any]]) -> None:
    if context.get("operation") != "approval":
        return

    approval = context.get("approval") or {}
    tx = context.get("tx") or {}
    spender = approval.get("spender") or tx.get("spender")
    spender_type = str(approval.get("spenderType") or tx.get("spenderType") or "").strip().lower()
    amount = approval.get("amount") or approval.get("allowance") or tx.get("allowance") or tx.get("amount")
    allowlist = approval.get("allowedSpenders") or approval.get("allowlist") or []
    denylist = approval.get("blockedSpenders") or approval.get("denylist") or []

    if not spender:
        _add_reason(
            reasons,
            {
                "code": "APPROVAL_SPENDER_MISSING",
                "severity": str(policy.get("unknownSpenderSeverity", "warn")),
                "score": 73,
                "message": "Approval spender is missing or could not be identified.",
            },
        )
    else:
        spender_lc = str(spender).lower()
        if _contains_address(denylist, spender_lc):
            _add_reason(
                reasons,
                {
                    "code": "APPROVAL_SPENDER_BLOCKED",
                    "severity": "block",
                    "score": 97,
                    "message": "Approval spender is explicitly blocked by policy input.",
                },
            )
        if allowlist and not _contains_address(allowlist, spender_lc):
            _add_reason(
                reasons,
                {
                    "code": "APPROVAL_SPENDER_NOT_ALLOWLISTED",
                    "severity": "warn",
                    "score": 74,
                    "message": "Approval spender is not in the provided allowlist.",
                },
            )

    if spender_type in ("eoa", "externally-owned", "externally_owned", "wallet"):
        severity = str(policy.get("spenderEoaSeverity", "warn"))
        _add_reason(
            reasons,
            {
                "code": "APPROVAL_SPENDER_EOA",
                "severity": severity,
                "score": 76 if severity == "warn" else 90,
                "message": "Approval spender appears to be an externally owned account.",
            },
        )

    if _is_unlimited_approval(approval, amount):
        severity = str(policy.get("unlimitedApprovalSeverity", "warn"))
        _add_reason(
            reasons,
            {
                "code": "APPROVAL_UNLIMITED",
                "severity": severity,
                "score": 75 if severity == "warn" else 89,
                "message": "Approval grants an unlimited or effectively unlimited allowance.",
            },
        )


def _evaluate_external_evidence(context: Dict[str, Any], policy: Dict[str, Any], reasons: List[Dict[str, Any]]) -> None:
    external = context.get("externalEvidence") or {}
    if not isinstance(external, dict):
        return
    _evaluate_goplus_evidence(external.get("goplus") or {}, policy, reasons)
    _evaluate_birdeye_evidence(external.get("birdeye") or {}, policy, reasons)
    _evaluate_rootdata_evidence(external.get("rootdata") or {}, reasons)


def _evaluate_goplus_evidence(goplus: Any, policy: Dict[str, Any], reasons: List[Dict[str, Any]]) -> None:
    if not isinstance(goplus, dict):
        return
    risk_level = str(_dig(goplus, ["riskLevel", "risk_level", "level"]) or "").upper()
    if risk_level == "CRITICAL":
        _add_reason(
            reasons,
            {
                "code": "EXTERNAL_GOPLUS_CRITICAL",
                "severity": "block",
                "score": 96,
                "message": "GoPlus external evidence reports CRITICAL token risk.",
            },
        )
    elif risk_level == "HIGH":
        severity = str(policy.get("tokenHighSeverity", "warn"))
        _add_reason(
            reasons,
            {
                "code": "EXTERNAL_GOPLUS_HIGH",
                "severity": severity,
                "score": 69 if severity == "warn" else 86,
                "message": "GoPlus external evidence reports HIGH token risk.",
            },
        )

    if any(_truthy(_dig(goplus, [key])) for key in ("is_honeypot", "isHoneypot", "honeypot")):
        _add_reason(
            reasons,
            {
                "code": "EXTERNAL_GOPLUS_HONEYPOT",
                "severity": "block",
                "score": 98,
                "message": "GoPlus external evidence indicates honeypot risk.",
            },
        )
    if any(_truthy(_dig(goplus, [key])) for key in ("is_blacklisted", "blacklisted", "is_malicious", "malicious")):
        _add_reason(
            reasons,
            {
                "code": "EXTERNAL_GOPLUS_BLOCKLISTED",
                "severity": "block",
                "score": 98,
                "message": "GoPlus external evidence indicates a malicious or blocklisted token/address.",
            },
        )

    buy_tax = _find_numeric(goplus, ("buy_tax", "buyTax", "buyTaxes"))
    sell_tax = _find_numeric(goplus, ("sell_tax", "sellTax", "sellTaxes"))
    max_tax = max([tax for tax in (buy_tax, sell_tax) if tax is not None], default=None)
    if max_tax is not None:
        if max_tax > Decimal(str(policy["highTaxBlockPct"])):
            _add_reason(
                reasons,
                {
                    "code": "EXTERNAL_TAX_HIGH",
                    "severity": "block",
                    "score": 88,
                    "message": "External evidence reports token tax above the active policy block threshold.",
                },
            )
        elif max_tax >= Decimal(str(policy["highTaxWarnPct"])):
            _add_reason(
                reasons,
                {
                    "code": "EXTERNAL_TAX_ELEVATED",
                    "severity": "warn",
                    "score": 67,
                    "message": "External evidence reports elevated token tax.",
                },
            )


def _evaluate_birdeye_evidence(birdeye: Any, policy: Dict[str, Any], reasons: List[Dict[str, Any]]) -> None:
    if not isinstance(birdeye, dict):
        return
    liquidity = _find_numeric(birdeye, ("liquidityUsd", "liquidityUSD", "liquidity", "totalLiquidity"))
    if liquidity is not None and liquidity < Decimal(str(policy["lowLiquidityUsd"])):
        _add_reason(
            reasons,
            {
                "code": "EXTERNAL_LIQUIDITY_LOW",
                "severity": "warn",
                "score": 57,
                "message": "External Birdeye evidence reports low liquidity.",
            },
        )

    top_holder_pct = _find_numeric(
        birdeye,
        ("top10HolderPercent", "top10HolderPct", "topHolderPercent", "holderConcentrationPct"),
    )
    if top_holder_pct is not None:
        if top_holder_pct > Decimal(str(policy["topHolderBlockPct"])):
            _add_reason(
                reasons,
                {
                    "code": "EXTERNAL_HOLDER_CONCENTRATION_HIGH",
                    "severity": "block",
                    "score": 87,
                    "message": "External evidence reports very high holder concentration.",
                },
            )
        elif top_holder_pct >= Decimal(str(policy["topHolderWarnPct"])):
            _add_reason(
                reasons,
                {
                    "code": "EXTERNAL_HOLDER_CONCENTRATION_ELEVATED",
                    "severity": "warn",
                    "score": 66,
                    "message": "External evidence reports elevated holder concentration.",
                },
            )


def _evaluate_rootdata_evidence(rootdata: Any, reasons: List[Dict[str, Any]]) -> None:
    if not isinstance(rootdata, dict):
        return
    risk_level = str(_dig(rootdata, ["riskLevel", "risk_level", "level"]) or "").upper()
    tags = _lowered_values(_dig(rootdata, ["tags", "labels", "riskTags"]) or [])
    bad_tags = {"scam", "rug", "rugpull", "malicious", "phishing", "exploit", "hack"}
    if risk_level == "CRITICAL" or any(tag in bad_tags for tag in tags):
        _add_reason(
            reasons,
            {
                "code": "EXTERNAL_ROOTDATA_CRITICAL",
                "severity": "block",
                "score": 92,
                "message": "RootData external evidence reports critical project risk.",
            },
        )
    elif risk_level == "HIGH":
        _add_reason(
            reasons,
            {
                "code": "EXTERNAL_ROOTDATA_HIGH",
                "severity": "warn",
                "score": 67,
                "message": "RootData external evidence reports high project risk.",
            },
        )


def _simulation_reverted(data: Any) -> bool:
    if isinstance(data, dict):
        explicit = _dig(data, ["success", "succeeded", "isSuccess"])
        if explicit is False:
            return True
        status = str(_dig(data, ["status", "state", "result"]) or "").lower()
        if status in ("failed", "fail", "reverted", "revert", "error"):
            return True
        if _dig(data, ["revertReason", "revert_reason", "error", "errorMessage"]):
            return True
        for value in data.values():
            if _simulation_reverted(value):
                return True
    elif isinstance(data, list):
        return any(_simulation_reverted(item) for item in data)
    elif isinstance(data, str):
        lowered = data.lower()
        return "revert" in lowered or "execution failed" in lowered
    return False


def _has_tx(context: Dict[str, Any]) -> bool:
    tx = context.get("tx") or {}
    return bool(tx.get("to") or tx.get("data") or tx.get("signedTx") or tx.get("signaturePayload"))


def _highest_verdict(reasons: Iterable[Dict[str, Any]]) -> str:
    verdict = "allow"
    for reason in reasons:
        severity = reason.get("severity", "allow")
        if VERDICT_PRIORITY.get(severity, 0) > VERDICT_PRIORITY[verdict]:
            verdict = severity
    return verdict


def _risk_score(reasons: List[Dict[str, Any]], verdict: str) -> int:
    if not reasons:
        return 5
    score = max(int(reason.get("score", 0)) for reason in reasons)
    if verdict == "block":
        return max(score, 80)
    if verdict == "warn":
        return max(score, 50)
    return min(score, 20)


def _safe_next_step(verdict: str) -> str:
    if verdict == "block":
        return "Cancel the operation. Do not ask the user to sign or broadcast this transaction."
    if verdict == "warn":
        return "Show the warning reasons and ask the user for explicit confirmation before signing."
    return "Proceed with the normal signing or broadcast flow if the user already requested it."


def _public_reasons(reasons: List[Dict[str, Any]]) -> List[Dict[str, str]]:
    return [
        {
            "code": str(reason.get("code")),
            "severity": str(reason.get("severity")),
            "message": str(reason.get("message")),
        }
        for reason in reasons
    ]


def _add_reason(reasons: List[Dict[str, Any]], reason: Dict[str, Any]) -> None:
    code = reason.get("code")
    if any(existing.get("code") == code and existing.get("message") == reason.get("message") for existing in reasons):
        return
    reasons.append(reason)


def _audit_trail(
    context: Dict[str, Any],
    evidence: Dict[str, Any],
    policy: Dict[str, Any],
    verdict: str,
    reasons: List[Dict[str, Any]],
) -> Dict[str, str]:
    evidence_hash = _sha256(_stable_json(evidence))
    decision_payload = {
        "context": _audit_context(context),
        "evidenceHash": evidence_hash,
        "policyProfile": policy.get("profile"),
        "policyVersion": policy.get("policyVersion", POLICY_VERSION),
        "verdict": verdict,
        "reasonCodes": [str(reason.get("code")) for reason in reasons],
    }
    return {
        "decisionId": "arf_" + _sha256(_stable_json(decision_payload))[:16],
        "policyProfile": str(policy.get("profile")),
        "policyVersion": str(policy.get("policyVersion", POLICY_VERSION)),
        "evidenceHash": evidence_hash,
    }


def _audit_context(context: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "chain": context.get("chain"),
        "operation": context.get("operation"),
        "walletAddress": context.get("walletAddress"),
        "tokenIn": context.get("tokenIn"),
        "tokenOut": context.get("tokenOut"),
        "amountIn": context.get("amountIn"),
        "amountInUsd": context.get("amountInUsd"),
        "walletValueUsd": context.get("walletValueUsd"),
        "quote": context.get("quote"),
        "tx": context.get("tx"),
        "approval": context.get("approval"),
        "externalEvidence": context.get("externalEvidence"),
    }


def _stable_json(payload: Any) -> str:
    return json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False, default=str)


def _sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def _is_unlimited_approval(approval: Dict[str, Any], amount: Any) -> bool:
    explicit = approval.get("isUnlimited") or approval.get("unlimited") or approval.get("isUnlimitedApproval")
    if _truthy(explicit):
        return True
    if isinstance(amount, str) and amount.strip().lower() in ("unlimited", "infinite", "max", "max_uint256"):
        return True
    numeric = _to_decimal(amount)
    if numeric is None:
        return False
    return numeric >= MAX_UINT256


def _contains_address(values: Any, target_lower: str) -> bool:
    if isinstance(values, str):
        values = [values]
    if not isinstance(values, list):
        return False
    return any(str(value).lower() == target_lower for value in values)


def _is_stable_or_native(token: Dict[str, Any]) -> bool:
    symbol = str(token.get("symbol") or "").strip().upper()
    address = str(token.get("address") or "").strip()
    native_addresses = {
        "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "11111111111111111111111111111111",
        "So11111111111111111111111111111111111111112",
    }
    stable_symbols = {
        "USDC",
        "USDT",
        "DAI",
        "USDE",
        "USDS",
        "USDG",
        "PYUSD",
        "FDUSD",
        "TUSD",
        "FRAX",
        "XLAYER_USDT",
        "USD₮0",
    }
    native_symbols = {"OKB", "WOKB", "SOL", "WSOL", "ETH", "WETH"}
    return address in native_addresses or symbol in stable_symbols or symbol in native_symbols


def _truthy(value: Any) -> bool:
    if isinstance(value, bool):
        return value
    if value is None:
        return False
    return str(value).strip().lower() in ("1", "true", "yes", "y", "risk", "risky", "high", "critical")


def _lowered_values(values: Any) -> List[str]:
    if isinstance(values, str):
        return [values.strip().lower()]
    if isinstance(values, list):
        return [str(value).strip().lower() for value in values]
    return []


def _to_decimal(value: Any) -> Optional[Decimal]:
    if value is None or value == "":
        return None
    try:
        return Decimal(str(value))
    except Exception:
        return None


def _dig(data: Any, keys: Iterable[str]) -> Any:
    if isinstance(data, dict):
        for key in keys:
            if key in data:
                return data[key]
        for value in data.values():
            found = _dig(value, keys)
            if found is not None:
                return found
    elif isinstance(data, list):
        for value in data:
            found = _dig(value, keys)
            if found is not None:
                return found
    return None


def _find_numeric(data: Any, keys: Iterable[str]) -> Optional[Decimal]:
    found = _dig(data, keys)
    if found is None:
        return None
    return _to_decimal(found)
