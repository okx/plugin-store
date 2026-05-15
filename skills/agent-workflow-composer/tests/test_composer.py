from agent_workflow_composer.composer import build_plan, template, validate_plan, validate_payload
from agent_workflow_composer.models import InputError, normalize_request


def request(execution_mode="dry-run"):
    return {
        "intent": "Dry-run swap 10 USD of SOL to USDC with risk checks.",
        "workflowType": "swap",
        "chain": "solana",
        "tokenIn": {"symbol": "SOL"},
        "tokenOut": {"symbol": "USDC"},
        "amountUsd": "10",
        "executionMode": execution_mode,
        "riskProfile": "balanced",
        "plugins": {
            "wallet": "okx-agentic-wallet",
            "quote": "okx-dex-swap",
            "risk": "agent-risk-firewall",
        },
        "externalEvidencePlugins": ["goplus-security", "birdeye-plugin"],
    }


def test_build_dry_run_plan_has_firewall_and_no_execute():
    plan = build_plan(request())
    step_ids = [step["id"] for step in plan["steps"]]
    commands = " ".join(str(step.get("command", "")) for step in plan["steps"])

    assert plan["validation"]["ok"] is True
    assert "risk_firewall_check" in step_ids
    assert "user_confirmation_gate" in step_ids
    assert "onchainos swap swap" in commands
    assert "onchainos swap execute" not in commands


def test_confirm_before_execute_places_execute_after_firewall_and_gate():
    plan = build_plan(request("confirm-before-execute"))
    step_ids = [step["id"] for step in plan["steps"]]

    assert plan["validation"]["ok"] is True
    assert step_ids.index("risk_firewall_check") < step_ids.index("user_confirmation_gate")
    assert step_ids.index("user_confirmation_gate") < step_ids.index("execute_after_confirmation")


def test_validate_rejects_execute_before_firewall():
    bad_plan = {
        "executionMode": "confirm-before-execute",
        "requiredPlugins": [{"name": "agent-risk-firewall"}],
        "steps": [
            {
                "id": "execute_after_confirmation",
                "mode": "execution",
                "command": "onchainos swap execute --from A --to B",
                "requiresConfirmation": True,
            },
            {"id": "risk_firewall_check", "command": "agent-risk-firewall check --input x --format json"},
            {"id": "user_confirmation_gate"},
        ],
    }
    result = validate_plan(bad_plan)
    assert result["ok"] is False
    assert any(error["code"] == "EXECUTE_BEFORE_FIREWALL" for error in result["errors"])


def test_validate_rejects_force_flag():
    plan = build_plan(request("confirm-before-execute"))
    for step in plan["steps"]:
        if step["id"] == "execute_after_confirmation":
            step["command"] += " --force"
    result = validate_plan(plan)
    assert result["ok"] is False
    assert any(error["code"] == "FORCE_NOT_ALLOWED" for error in result["errors"])


def test_template_competition_trade_uses_competition_profile():
    payload = template("competition-trade")
    assert payload["workflowType"] == "competition-trade"
    assert payload["riskProfile"] == "competition"
    assert payload["plugins"]["competition"] == "okx-growth-competition"
    assert payload["competition"]["eligibleTokenTradeRequired"] is True
    assert "xlayer-alpha-hunter" in payload["plugins"].values()


def test_competition_plan_has_preflight_before_firewall():
    plan = build_plan(template("competition-trade"))
    step_ids = [step["id"] for step in plan["steps"]]

    assert plan["validation"]["ok"] is True
    assert "okx-growth-competition" in [plugin["name"] for plugin in plan["requiredPlugins"]]
    assert step_ids.index("competition_discovery") < step_ids.index("competition_context")
    assert step_ids.index("competition_context") < step_ids.index("risk_firewall_check")
    assert "competitionContext" in plan["steps"][step_ids.index("risk_firewall_check")]["requires"]


def test_validate_rejects_competition_plan_without_context_step():
    plan = build_plan(template("competition-trade"))
    plan["steps"] = [step for step in plan["steps"] if step["id"] != "competition_context"]

    result = validate_plan(plan)

    assert result["ok"] is False
    assert any(error["code"] == "MISSING_COMPETITION_CONTEXT" for error in result["errors"])


def test_validate_rejects_competition_plan_without_competition_profile():
    plan = build_plan(template("competition-trade"))
    plan["riskProfile"] = "balanced"

    result = validate_plan(plan)

    assert result["ok"] is False
    assert any(error["code"] == "COMPETITION_PROFILE_REQUIRED" for error in result["errors"])


def test_validate_payload_accepts_request():
    result = validate_payload(request())
    assert result["ok"] is True


def test_normalize_request_rejects_missing_intent():
    try:
        normalize_request({"workflowType": "swap"})
    except InputError as exc:
        assert exc.code == "INVALID_INPUT"
    else:
        raise AssertionError("Expected InputError")
