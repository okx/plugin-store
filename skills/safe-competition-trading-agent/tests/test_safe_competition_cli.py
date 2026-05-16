import json
import os
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def run_cli(args, input_text=None):
    env = os.environ.copy()
    env["PYTHONPATH"] = str(ROOT / "src")
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    return subprocess.run(
        [sys.executable, "-m", "safe_competition_trading_agent.cli"] + args,
        input=input_text,
        capture_output=True,
        text=True,
        cwd=str(ROOT),
        env=env,
        check=False,
    )


def request():
    return {
        "intent": "Dry-run a safe competition trade.",
        "chain": "xlayer",
        "budgetUsd": "10",
        "executionMode": "dry-run",
    }


def test_plan_reads_stdin_and_outputs_json():
    completed = run_cli(["plan", "--input", "-", "--format", "json"], json.dumps(request()))

    assert completed.returncode == 0
    payload = json.loads(completed.stdout)
    assert payload["strategyId"].startswith("scta_")
    assert payload["validation"]["ok"] is True


def test_validate_returns_nonzero_for_bad_plan():
    bad_plan = {"executionMode": "dry-run", "steps": []}
    completed = run_cli(["validate", "--input", "-", "--format", "json"], json.dumps(bad_plan))

    assert completed.returncode == 1
    payload = json.loads(completed.stdout)
    assert payload["ok"] is False


def test_template_command_outputs_request():
    completed = run_cli(["template", "--name", "rank-optimizer"])

    assert completed.returncode == 0
    payload = json.loads(completed.stdout)
    assert payload["goal"] == "rank-optimizer"


def test_malformed_json_returns_error_json():
    completed = run_cli(["plan", "--input", "-", "--format", "json"], "{bad json")

    assert completed.returncode != 0
    payload = json.loads(completed.stdout)
    assert payload["error"]["code"] == "INVALID_JSON"


def test_self_test_passes():
    completed = run_cli(["self-test"])

    assert completed.returncode == 0
    payload = json.loads(completed.stdout)
    assert payload["status"] == "pass"
