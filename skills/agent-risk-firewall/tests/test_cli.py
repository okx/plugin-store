import json
import os
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def run_cli(args, input_text=None):
    env = os.environ.copy()
    env["PYTHONPATH"] = str(ROOT / "src")
    env["AGENT_RISK_FIREWALL_DISABLE_ONCHAINOS"] = "1"
    return subprocess.run(
        [sys.executable, "-m", "agent_risk_firewall.cli"] + args,
        input=input_text,
        capture_output=True,
        text=True,
        cwd=str(ROOT),
        env=env,
        check=False,
    )


def valid_payload():
    return {
        "chain": "xlayer",
        "operation": "swap",
        "walletAddress": "0x0000000000000000000000000000000000000000",
        "tokenIn": {
            "address": "0x0000000000000000000000000000000000000001",
            "symbol": "USDC",
            "decimals": 6,
        },
        "tokenOut": {
            "address": "0x0000000000000000000000000000000000000002",
            "symbol": "TOKEN",
            "decimals": 18,
        },
        "amountIn": "10",
        "amountInUsd": 10,
        "quote": {"expectedOut": "100", "slippagePct": 1, "priceImpactPct": 1},
    }


def test_check_reads_file_and_outputs_json(tmp_path):
    request = tmp_path / "request.json"
    request.write_text(json.dumps(valid_payload()), encoding="utf-8")
    completed = run_cli(["check", "--input", str(request), "--format", "json"])
    assert completed.returncode == 0
    payload = json.loads(completed.stdout)
    assert payload["verdict"] == "warn"
    assert "riskScore" in payload
    assert "reasons" in payload
    assert any(reason["code"] == "SCAN_UNAVAILABLE" for reason in payload["reasons"])


def test_check_reads_stdin():
    completed = run_cli(["check", "--input", "-", "--format", "json"], json.dumps(valid_payload()))
    assert completed.returncode == 0
    payload = json.loads(completed.stdout)
    assert payload["verdict"] == "warn"


def test_malformed_input_returns_error_json():
    completed = run_cli(["check", "--input", "-", "--format", "json"], "{bad json")
    assert completed.returncode != 0
    payload = json.loads(completed.stdout)
    assert payload["error"]["code"] == "INVALID_JSON"


def test_policy_command_outputs_balanced_profile():
    completed = run_cli(["policy", "--profile", "balanced"])
    assert completed.returncode == 0
    payload = json.loads(completed.stdout)
    assert payload["profile"] == "balanced"
    assert payload["maxTradeUsd"] == 250


def test_self_test_passes():
    completed = run_cli(["self-test"])
    assert completed.returncode == 0
    payload = json.loads(completed.stdout)
    assert payload["status"] == "pass"
