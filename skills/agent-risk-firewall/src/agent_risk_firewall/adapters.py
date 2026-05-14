import json
import os
import shutil
import subprocess
from typing import Any, Dict, List, Optional

from .models import target_token


class OnchainOSAdapter:
    def __init__(self, timeout_seconds: int = 20):
        self.timeout_seconds = timeout_seconds
        self.binary = shutil.which("onchainos")

    def is_available(self) -> bool:
        if os.environ.get("AGENT_RISK_FIREWALL_DISABLE_ONCHAINOS") == "1":
            return False
        return bool(self.binary)

    def collect(self, context: Dict[str, Any]) -> Dict[str, Any]:
        evidence: Dict[str, Any] = {}
        token = target_token(context)
        token_address = token.get("address") if isinstance(token, dict) else None
        tx = context.get("tx") or {}

        if token_address:
            evidence["tokenScan"] = self.token_scan(context["chain"], token_address)
            evidence["tokenReport"] = self.token_report(context["chain"], token_address)
        else:
            evidence["tokenScan"] = {"status": "skipped", "reason": "No target token address."}

        if self._has_tx_context(tx):
            evidence["txScan"] = self.tx_scan(context)
            evidence["simulation"] = self.simulate(context)
        else:
            evidence["txScan"] = {"status": "skipped", "reason": "No transaction context."}
            evidence["simulation"] = {"status": "skipped", "reason": "No transaction context."}

        return evidence

    def token_scan(self, chain: str, address: str) -> Dict[str, Any]:
        chain_id = _chain_id(chain)
        return self._run_json(["security", "token-scan", "--tokens", f"{chain_id}:{address}"])

    def token_report(self, chain: str, address: str) -> Dict[str, Any]:
        return self._run_json(["token", "liquidity", "--chain", chain, "--address", address])

    def tx_scan(self, context: Dict[str, Any]) -> Dict[str, Any]:
        tx = context.get("tx") or {}
        args = ["security", "tx-scan", "--chain", context["chain"]]
        self._append_tx_scan_args(args, context["chain"], tx)
        return self._run_json(args)

    def simulate(self, context: Dict[str, Any]) -> Dict[str, Any]:
        tx = context.get("tx") or {}
        args = ["gateway", "simulate", "--chain", context["chain"]]
        self._append_simulate_args(args, tx)
        return self._run_json(args)

    def _run_json(self, args: List[str]) -> Dict[str, Any]:
        if not self.is_available():
            return {
                "status": "unavailable",
                "code": "ONCHAINOS_UNAVAILABLE",
                "message": "The onchainos CLI is not installed or is disabled.",
                "command": ["onchainos"] + args,
            }

        command = [self.binary or "onchainos"] + args
        try:
            completed = subprocess.run(
                command,
                check=False,
                capture_output=True,
                text=True,
                timeout=self.timeout_seconds,
            )
        except subprocess.TimeoutExpired:
            return {
                "status": "timeout",
                "code": "SCAN_TIMEOUT",
                "message": "OnchainOS command timed out.",
                "command": ["onchainos"] + args,
            }
        except OSError as exc:
            return {
                "status": "error",
                "code": "ONCHAINOS_ERROR",
                "message": str(exc),
                "command": ["onchainos"] + args,
            }

        parsed = _parse_output(completed.stdout)
        if completed.returncode != 0:
            return {
                "status": "error",
                "code": "ONCHAINOS_COMMAND_FAILED",
                "returnCode": completed.returncode,
                "message": (completed.stderr or completed.stdout or "").strip(),
                "command": ["onchainos"] + args,
                "data": parsed,
            }

        return {
            "status": "ok",
            "command": ["onchainos"] + args,
            "data": parsed,
        }

    @staticmethod
    def _has_tx_context(tx: Dict[str, Any]) -> bool:
        return bool(tx.get("to") or tx.get("data") or tx.get("signaturePayload") or tx.get("signedTx"))

    @staticmethod
    def _append_tx_scan_args(args: List[str], chain: str, tx: Dict[str, Any]) -> None:
        if chain == "solana":
            mapping = (
                ("from", "--from"),
                ("encoding", "--encoding"),
                ("signaturePayload", "--transactions"),
            )
        else:
            mapping = (
                ("from", "--from"),
                ("to", "--to"),
                ("data", "--data"),
                ("value", "--value"),
                ("gas", "--gas"),
                ("gasPrice", "--gas-price"),
            )
        for source, flag in mapping:
            value = tx.get(source)
            if value is not None and value != "":
                args.extend([flag, str(value)])

    @staticmethod
    def _append_simulate_args(args: List[str], tx: Dict[str, Any]) -> None:
        mapping = (
            ("from", "--from"),
            ("to", "--to"),
            ("data", "--data"),
            ("value", "--amount"),
        )
        for source, flag in mapping:
            value = tx.get(source)
            if value is not None and value != "":
                args.extend([flag, str(value)])


def _chain_id(chain: str) -> str:
    if chain == "xlayer":
        return "196"
    if chain == "solana":
        return "501"
    return chain


def _parse_output(output: str) -> Any:
    text = (output or "").strip()
    if not text:
        return {}
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass

    start = text.find("{")
    end = text.rfind("}")
    if start != -1 and end != -1 and end > start:
        try:
            return json.loads(text[start : end + 1])
        except json.JSONDecodeError:
            pass
    return {"raw": text}
