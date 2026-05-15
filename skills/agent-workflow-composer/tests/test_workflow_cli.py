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
        [sys.executable, "-m", "agent_workflow_composer.cli"] + args,
        input=input_text,
        capture_output=True,
        text=True,
        cwd=str(ROOT),
        env=env,
        check=False,
    )


def request():
    return {
        "intent": "Dry-run swap 10 USD of SOL to USDC with risk checks.",
        "workflowType": "swap",
        "chain": "solana",
        "executionMode": "dry-run",
        "riskProfile": "balanced",
    }


def test_plan_reads_stdin_and_outputs_json():
    completed = run_cli(["plan", "--input", "-", "--format", "json"], json.dumps(request()))
    assert completed.returncode == 0
    payload = json.loads(completed.stdout)
    assert payload["workflowId"].startswith("awc_")
    assert payload["validation"]["ok"] is True


def test_validate_returns_nonzero_for_bad_plan():
    bad_plan = {
        "executionMode": "dry-run",
        "requiredPlugins": [],
        "steps": [
            {
                "id": "execute",
                "mode": "execution",
                "command": "onchainos swap execute --from A --to B",
            }
        ],
    }
    completed = run_cli(["validate", "--input", "-", "--format", "json"], json.dumps(bad_plan))
    assert completed.returncode == 1
    payload = json.loads(completed.stdout)
    assert payload["ok"] is False


def test_template_command_outputs_guarded_swap():
    completed = run_cli(["template", "--name", "guarded-swap"])
    assert completed.returncode == 0
    payload = json.loads(completed.stdout)
    assert payload["workflowType"] == "swap"


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
