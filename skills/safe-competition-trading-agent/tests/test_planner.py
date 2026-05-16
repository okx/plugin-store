from safe_competition_trading_agent.planner import build_plan, dry_run, execute, template, validate_plan


def request(execution_mode="dry-run"):
    payload = template("competition-safe-swap")
    payload["executionMode"] = execution_mode
    payload["tokenOut"] = {"symbol": "MEME", "liquidityUsd": "150000", "riskLevel": "LOW"}
    return payload


def test_build_plan_uses_onchainos_competition_and_swap():
    plan = build_plan(request())
    commands = " ".join(str(step.get("command") or "") for step in plan["steps"])
    step_ids = [step["id"] for step in plan["steps"]]

    assert plan["validation"]["ok"] is True
    assert "onchainos competition list" in commands
    assert "onchainos swap quote" in commands
    assert "agent-risk-firewall check" in commands
    assert "execute_after_confirmation" not in step_ids


def test_confirm_before_execute_places_execution_after_firewall_and_confirmation():
    plan = build_plan(request("confirm-before-execute"))
    step_ids = [step["id"] for step in plan["steps"]]

    assert plan["validation"]["ok"] is True
    assert step_ids.index("risk_firewall_check") < step_ids.index("user_confirmation_gate")
    assert step_ids.index("user_confirmation_gate") < step_ids.index("execute_after_confirmation")


def test_dry_run_reports_missing_competition_context_first():
    result = dry_run(request())

    assert result["status"] == "needs_competition_context"
    assert result["firewallInput"]["policyProfile"] == "competition"


def test_dry_run_ready_with_quote_competition_and_allow_verdict():
    payload = request()
    payload["competition"] = {"active": True, "joined": True, "chainName": "X Layer", "supportedChains": ["xlayer", "solana"]}
    payload["quote"] = {"slippagePct": 1, "priceImpactPct": 1}
    payload["riskVerdict"] = {"verdict": "allow", "reasons": []}

    result = dry_run(payload)

    assert result["status"] == "ready"
    assert result["riskGate"]["state"] == "ready_for_confirmation"


def test_execute_requires_confirmation_and_firewall_verdict():
    payload = request("confirm-before-execute")
    payload["competition"] = {"active": True, "joined": True}
    payload["quote"] = {"slippagePct": 1}
    payload["riskVerdict"] = {"verdict": "allow"}

    blocked = execute(payload)
    assert blocked["status"] == "blocked"

    payload["confirmed"] = True
    ready = execute(payload)
    assert ready["status"] == "ready-to-execute"
    assert "onchainos swap execute" in ready["executionCommand"]


def test_execute_blocks_firewall_block_even_when_confirmed():
    payload = request("confirm-before-execute")
    payload["confirmed"] = True
    payload["riskVerdict"] = {"verdict": "block", "reasons": [{"code": "TOKEN_CRITICAL"}]}

    result = execute(payload)

    assert result["status"] == "blocked"
    assert result["riskGate"]["state"] == "blocked"


def test_validate_rejects_execute_before_firewall():
    plan = build_plan(request("confirm-before-execute"))
    steps = plan["steps"]
    execute_step = [step for step in steps if step["id"] == "execute_after_confirmation"][0]
    plan["steps"] = [execute_step] + [step for step in steps if step["id"] != "execute_after_confirmation"]

    result = validate_plan(plan)

    assert result["ok"] is False
    assert any(error["code"] == "EXECUTE_BEFORE_FIREWALL" for error in result["errors"])
