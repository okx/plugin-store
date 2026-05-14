import json
from pathlib import Path

import pytest

from agent_risk_firewall.models import validate_check_input
from agent_risk_firewall.policy import evaluate


FIXTURES = Path(__file__).resolve().parent / "fixtures"


def _load_fixture(name):
    with open(FIXTURES / name, "r", encoding="utf-8") as handle:
        return json.load(handle)


def _payload(operation="buy", with_tx=False):
    payload = {
        "chain": "xlayer",
        "operation": operation,
        "walletAddress": "0x0000000000000000000000000000000000000000",
        "tokenIn": {
            "address": "0x0000000000000000000000000000000000000001",
            "symbol": "USDC",
            "decimals": 6,
        },
        "tokenOut": {
            "address": "0x0000000000000000000000000000000000000002",
            "symbol": "RISK",
            "decimals": 18,
        },
        "amountIn": "10",
        "amountInUsd": "10",
        "quote": {
            "expectedOut": "100",
            "slippagePct": 0.5,
            "priceImpactPct": 0.5,
            "route": "fixture-route",
            "venue": "FixtureDEX",
        },
        "policyProfile": "balanced",
    }
    if with_tx:
        payload["tx"] = {
            "from": "0x0000000000000000000000000000000000000000",
            "to": "0x0000000000000000000000000000000000000003",
            "data": "0x095ea7b3",
            "value": "0",
        }
    return payload


@pytest.mark.parametrize(
    "fixture_name,operation,with_tx,expected_verdict,expected_code,requires_confirmation",
    [
        ("token_high.json", "buy", False, "warn", "TOKEN_HIGH", True),
        ("token_critical.json", "buy", False, "block", "TOKEN_CRITICAL", False),
        ("token_critical.json", "sell", False, "warn", "TOKEN_CRITICAL_SELL", True),
        ("tx_scan_warn.json", "swap", True, "warn", "TX_SCAN_WARN", True),
        ("tx_scan_block.json", "swap", True, "block", "TX_SCAN_BLOCK", False),
        ("simulation_revert.json", "swap", True, "block", "SIMULATION_REVERT", False),
    ],
)
def test_golden_risk_fixtures(
    fixture_name,
    operation,
    with_tx,
    expected_verdict,
    expected_code,
    requires_confirmation,
):
    context, findings = validate_check_input(_payload(operation=operation, with_tx=with_tx))
    result = evaluate(context, _load_fixture(fixture_name), findings)

    assert result["verdict"] == expected_verdict
    assert result["requiresUserConfirmation"] is requires_confirmation
    assert any(reason["code"] == expected_code for reason in result["reasons"])

    if expected_verdict == "block":
        assert "Do not ask the user to sign or broadcast" in result["safeNextStep"]
    elif expected_verdict == "warn":
        assert "ask the user for explicit confirmation" in result["safeNextStep"]
