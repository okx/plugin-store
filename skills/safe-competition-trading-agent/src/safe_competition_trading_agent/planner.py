import hashlib
import json
from typing import Any, Dict, Iterable, List

from .candidate_selector import is_real_competition_token, select_candidate
from .models import decimal_to_str, normalize_request
from .risk_bridge import build_firewall_input, risk_gate


AGENT_VERSION = "1.0.0"


def template(name: str = "competition-safe-swap") -> Dict[str, Any]:
    normalized = str(name or "competition-safe-swap").strip().lower().replace("_", "-")
    if normalized in ("competition-safe-swap", "safe-competition-trade"):
        return {
            "intent": "Dry-run a safe competition trade with a 10 USD budget.",
            "chain": "xlayer",
            "goal": "safe-volume",
            "budgetUsd": "10",
            "tokenIn": {"symbol": "USDC"},
            "executionMode": "dry-run",
            "plugins": {
                "wallet": "okx-agentic-wallet",
                "competition": "okx-growth-competition",
                "token": "okx-dex-token",
                "signal": "okx-dex-signal",
                "swap": "okx-dex-swap",
                "composer": "agent-workflow-composer",
                "risk": "agent-risk-firewall",
            },
        }
    if normalized == "rank-optimizer":
        payload = template("competition-safe-swap")
        payload["intent"] = "Find a safer eligible trade that may improve my Agentic Trading competition rank."
        payload["goal"] = "rank-optimizer"
        payload["budgetUsd"] = "25"
        return payload
    return {
        "error": {
            "code": "UNKNOWN_TEMPLATE",
            "message": "Unknown template. Use competition-safe-swap or rank-optimizer.",
        }
    }


def build_plan(payload: Dict[str, Any]) -> Dict[str, Any]:
    request = normalize_request(payload)
    candidate = select_candidate(request)
    steps = _steps(request, candidate)
    plan = {
        "strategyId": "scta_" + _sha256(_stable_json(request))[:16],
        "agentVersion": AGENT_VERSION,
        "status": "planned",
        "intent": request["intent"],
        "chain": request["chain"],
        "goal": request["goal"],
        "budgetUsd": decimal_to_str(request.get("budgetUsd")),
        "executionMode": request["executionMode"],
        "selectedCandidate": candidate,
        "requiredPlugins": _required_plugins(request),
        "steps": steps,
        "safetyGates": _safety_gates(),
        "outputContract": _output_contract(),
        "runbook": _runbook(request),
    }
    plan["validation"] = validate_plan(plan)
    return plan


def dry_run(payload: Dict[str, Any]) -> Dict[str, Any]:
    request = normalize_request(payload)
    candidate = select_candidate(request)
    plan = build_plan(payload)
    firewall_input = build_firewall_input(request, candidate)
    readiness = _dry_run_readiness(request, candidate)
    return {
        "strategyId": plan["strategyId"],
        "agentVersion": AGENT_VERSION,
        "status": readiness["status"],
        "mode": "dry-run",
        "selectedCandidate": candidate,
        "competitionContextStatus": _competition_context_status(request),
        "quoteStatus": "ready" if request.get("quote") else "needs_onchainos_quote",
        "firewallInput": firewall_input,
        "riskGate": risk_gate(request),
        "nextAction": readiness["nextAction"],
        "plan": plan,
    }


def execute(payload: Dict[str, Any]) -> Dict[str, Any]:
    request = normalize_request(payload)
    candidate = select_candidate(request)
    gate = risk_gate(request)
    command = _execute_command(request, candidate)
    if request["executionMode"] != "confirm-before-execute":
        return {
            "status": "blocked",
            "reason": "executionMode must be confirm-before-execute for live execution.",
            "riskGate": gate,
            "executionCommand": command,
        }
    if not request.get("confirmed"):
        return {
            "status": "blocked",
            "reason": "Explicit user confirmation is required before execution.",
            "riskGate": gate,
            "executionCommand": command,
        }
    if not gate["okToExecute"]:
        return {
            "status": "blocked",
            "reason": "Risk gate does not allow execution.",
            "riskGate": gate,
            "executionCommand": command,
        }
    return {
        "status": "ready-to-execute",
        "riskGate": gate,
        "executionCommand": command,
        "mustRunAfter": ["agent-risk-firewall check", "explicit user confirmation"],
        "note": "CLI does not sign or broadcast; host agent must execute with onchainOS Agentic Wallet.",
    }


def validate_plan(plan: Dict[str, Any]) -> Dict[str, Any]:
    errors: List[Dict[str, str]] = []
    warnings: List[Dict[str, str]] = []
    steps = plan.get("steps") if isinstance(plan.get("steps"), list) else []
    ids = [str(step.get("id")) for step in steps if isinstance(step, dict)]

    required = [
        "wallet_preflight",
        "competition_detail",
        "competition_user_status",
        "token_discovery",
        "quote_swap",
        "prepare_unsigned_swap",
        "workflow_composer_plan",
        "risk_firewall_check",
        "user_confirmation_gate",
    ]
    for step_id in required:
        if step_id not in ids:
            errors.append(_issue("MISSING_STEP", "Missing required step: " + step_id))

    if "risk_firewall_check" in ids and "prepare_unsigned_swap" in ids and ids.index("prepare_unsigned_swap") > ids.index("risk_firewall_check"):
        errors.append(_issue("FIREWALL_BEFORE_TX_CONTEXT", "Firewall must run after quote and unsigned transaction context."))
    if "risk_firewall_check" in ids and "competition_user_status" in ids and ids.index("competition_user_status") > ids.index("risk_firewall_check"):
        errors.append(_issue("FIREWALL_BEFORE_COMPETITION_STATUS", "Firewall must run after competition user-status."))
    if "execute_after_confirmation" in ids:
        if ids.index("risk_firewall_check") > ids.index("execute_after_confirmation"):
            errors.append(_issue("EXECUTE_BEFORE_FIREWALL", "Execution appears before risk firewall."))
        if ids.index("user_confirmation_gate") > ids.index("execute_after_confirmation"):
            errors.append(_issue("EXECUTE_BEFORE_CONFIRMATION", "Execution appears before user confirmation gate."))
        execute_step = steps[ids.index("execute_after_confirmation")]
        if execute_step.get("requiresConfirmation") is not True:
            errors.append(_issue("EXECUTE_WITHOUT_CONFIRMATION", "Execution step must require confirmation."))

    for step in steps:
        command = str(step.get("command") or "")
        if "--force" in command:
            errors.append(_issue("FORCE_NOT_ALLOWED", "Generated commands must not include --force."))
    if not _uses_onchainos(steps):
        errors.append(_issue("ONCHAINOS_NOT_USED", "Strategy must use onchainOS as data source and trading tool."))
    if plan.get("executionMode") == "dry-run" and "execute_after_confirmation" in ids:
        errors.append(_issue("DRY_RUN_HAS_EXECUTION", "Dry-run plan must not include execution step."))
    if plan.get("executionMode") == "confirm-before-execute":
        warnings.append(_issue("LIVE_EXECUTION_GUARDED", "Live execution is available only after firewall and explicit confirmation."))

    return {"ok": not errors, "errors": errors, "warnings": warnings}


def _steps(request: Dict[str, Any], candidate: Dict[str, Any]) -> List[Dict[str, Any]]:
    plugins = request["plugins"]
    steps: List[Dict[str, Any]] = [
        {
            "id": "parse_intent",
            "title": "Parse competition trading intent",
            "plugin": "safe-competition-trading-agent",
            "mode": "local",
            "produces": ["chain", "budgetUsd", "goal", "executionMode"],
        },
        {
            "id": "wallet_preflight",
            "title": "Check Agentic Wallet session and addresses",
            "plugin": plugins["wallet"],
            "mode": "read-only",
            "command": "onchainos wallet status",
            "produces": ["walletStatus", "evmWallet", "solWallet"],
        },
        {
            "id": "competition_discovery",
            "title": "Discover active Agentic Trading competitions",
            "plugin": plugins["competition"],
            "mode": "read-only",
            "command": "onchainos competition list --status 0",
            "produces": ["availableCompetitions"],
            "mustNot": ["show internal activityId values to the user"],
        },
        {
            "id": "competition_detail",
            "title": "Fetch selected competition rules",
            "plugin": plugins["competition"],
            "mode": "read-only",
            "command": "onchainos competition detail --activity-id <activityId>",
            "requires": ["selectedCompetition"],
            "produces": ["competitionDetail"],
            "mustNot": ["show internal activityId values to the user"],
        },
        {
            "id": "competition_user_status",
            "title": "Check registration status for selected competition",
            "plugin": plugins["competition"],
            "mode": "read-only",
            "command": "onchainos competition user-status --activity-id <activityId> --evm-wallet <evmWallet> --sol-wallet <solWallet>",
            "requires": ["competitionDetail", "evmWallet", "solWallet"],
            "produces": ["competitionUserStatus"],
        },
        {
            "id": "token_discovery",
            "title": "Find eligible real-token candidates from onchainOS data",
            "plugin": plugins["token"],
            "mode": "read-only",
            "command": "onchainos token hot-tokens --chain <chain>",
            "requires": ["competitionDetail"],
            "produces": ["tokenCandidates"],
        },
        {
            "id": "candidate_selection",
            "title": "Select safer competition token candidate",
            "plugin": "safe-competition-trading-agent",
            "mode": "local",
            "requires": ["tokenCandidates"],
            "produces": ["selectedCandidate"],
            "selectedCandidate": candidate,
        },
        {
            "id": "quote_swap",
            "title": "Get onchainOS swap quote",
            "plugin": plugins["swap"],
            "mode": "read-only",
            "command": "onchainos swap quote --from <tokenIn> --to <selectedCandidate> --readable-amount <amount> --chain <chain>",
            "requires": ["selectedCandidate", "walletStatus"],
            "produces": ["quote"],
        },
        {
            "id": "prepare_unsigned_swap",
            "title": "Prepare unsigned transaction context only",
            "plugin": plugins["swap"],
            "mode": "pre-execution",
            "command": "onchainos swap swap --from <tokenIn> --to <selectedCandidate> --readable-amount <amount> --chain <chain> --wallet <walletAddress>",
            "requires": ["quote", "walletAddress"],
            "produces": ["unsignedTransaction"],
            "mustNot": ["sign", "broadcast", "execute"],
        },
        {
            "id": "workflow_composer_plan",
            "title": "Validate plugin workflow order",
            "plugin": plugins["composer"],
            "mode": "local",
            "command": "agent-workflow-composer plan --input <composerRequest.json> --format json",
            "requires": ["competitionDetail", "quote", "unsignedTransaction"],
            "produces": ["workflowPlan"],
        },
        {
            "id": "risk_firewall_check",
            "title": "Run Agent Risk Firewall competition policy",
            "plugin": plugins["risk"],
            "mode": "risk-gate",
            "command": "agent-risk-firewall check --input <firewallInput.json> --format json",
            "requires": ["competitionDetail", "competitionUserStatus", "quote", "unsignedTransaction"],
            "produces": ["riskVerdict", "riskScore", "riskReasons", "firewallAudit"],
        },
        {
            "id": "user_confirmation_gate",
            "title": "Require explicit user confirmation",
            "plugin": "safe-competition-trading-agent",
            "mode": "confirmation",
            "requires": ["riskVerdict"],
            "agentInstruction": "If block, stop. If warn, show reasons and require explicit user confirmation. If allow, continue only if user already requested execution.",
            "produces": ["explicitUserConfirmation"],
        },
    ]
    if request["executionMode"] == "confirm-before-execute":
        steps.append(
            {
                "id": "execute_after_confirmation",
                "title": "Execute guarded swap through Agentic Wallet",
                "plugin": plugins["swap"],
                "mode": "execution",
                "command": _execute_command(request, candidate),
                "requires": ["riskVerdict", "explicitUserConfirmation"],
                "requiresConfirmation": True,
                "condition": "riskVerdict is allow, or riskVerdict is warn and explicitUserConfirmation is true",
                "mustNot": ["--force", "execute after block verdict"],
            }
        )
    return steps


def _dry_run_readiness(request: Dict[str, Any], candidate: Dict[str, Any]) -> Dict[str, str]:
    if not request.get("competition"):
        return {"status": "needs_competition_context", "nextAction": "Run onchainOS competition detail and user-status."}
    if not is_real_competition_token(candidate):
        return {"status": "needs_token_candidate", "nextAction": "Query onchainOS token data and select a real token candidate."}
    if not request.get("quote"):
        return {"status": "needs_quote", "nextAction": "Run onchainOS swap quote and prepare unsigned tx context."}
    gate = risk_gate(request)
    if gate["state"] == "risk_verdict_missing":
        return {"status": "needs_firewall", "nextAction": "Run agent-risk-firewall check with competition policy."}
    if gate["state"] == "blocked":
        return {"status": "blocked", "nextAction": "Cancel or revise the trade."}
    if gate["state"] == "needs_explicit_confirmation":
        return {"status": "needs_confirmation", "nextAction": "Show risk reasons and ask user to confirm explicitly."}
    return {"status": "ready", "nextAction": "Use confirm-before-execute only if the user explicitly wants live execution."}


def _competition_context_status(request: Dict[str, Any]) -> str:
    competition = request.get("competition") or {}
    if not competition:
        return "missing"
    if competition.get("active") is False:
        return "inactive"
    if competition.get("joined") is False:
        return "not_joined"
    return "ready"


def _execute_command(request: Dict[str, Any], candidate: Dict[str, Any]) -> str:
    token_in = (request.get("tokenIn") or {}).get("symbol") or "<tokenIn>"
    token_out = candidate.get("address") or candidate.get("symbol") or "<selectedCandidate>"
    amount = decimal_to_str(request.get("budgetUsd")) or "<amount>"
    return (
        "onchainos swap execute --from {from_token} --to {to_token} "
        "--readable-amount {amount} --chain {chain} --wallet <walletAddress>"
    ).format(from_token=token_in, to_token=token_out, amount=amount, chain=request["chain"])


def _required_plugins(request: Dict[str, Any]) -> List[Dict[str, str]]:
    purposes = {
        "wallet": "Agentic Wallet session and addresses",
        "competition": "competition list, detail, join status, rank context",
        "token": "onchain token discovery and token metadata",
        "swap": "onchainOS quote, unsigned transaction, and guarded execution",
        "composer": "workflow order validation",
        "risk": "pre-sign risk verdict and audit trail",
    }
    return [
        {"name": request["plugins"][key], "purpose": purposes[key]}
        for key in ("wallet", "competition", "token", "swap", "composer", "risk")
    ]


def _safety_gates() -> List[Dict[str, Any]]:
    return [
        {
            "id": "competition-eligibility",
            "rule": "Do not execute until competition detail and user-status are known.",
        },
        {
            "id": "risk-firewall",
            "rule": "Do not execute on block; require explicit confirmation on warn.",
        },
        {
            "id": "user-confirmation",
            "rule": "Live execution requires confirmed=true and a prior user instruction to execute.",
        },
    ]


def _output_contract() -> Dict[str, Any]:
    return {
        "sections": [
            "Competition Trade Plan",
            "Candidate",
            "Quote",
            "Risk Verdict",
            "Next Step",
        ],
        "neverShow": ["activityId", "accountId", "privateKey", "seedPhrase", "mnemonic"],
    }


def _runbook(request: Dict[str, Any]) -> List[str]:
    lines = [
        "Use onchainOS as the primary source for competition, token, quote, and execution data.",
        "Treat Solana plus the competition primary chain as supported until backend exposes full multi-chain support.",
        "Keep internal competition ids in tool context only; identify competitions to users by name.",
        "Never execute before agent-risk-firewall check.",
        "Never use --force.",
    ]
    if request["executionMode"] == "dry-run":
        lines.append("Dry-run mode must not execute a transaction.")
    return lines


def _uses_onchainos(steps: Iterable[Dict[str, Any]]) -> bool:
    commands = " ".join(str(step.get("command") or "") for step in steps)
    return "onchainos competition" in commands and "onchainos swap" in commands


def _issue(code: str, message: str) -> Dict[str, str]:
    return {"code": code, "message": message}


def _stable_json(payload: Any) -> str:
    return json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False, default=str)


def _sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()
