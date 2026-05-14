import argparse
import sys
from typing import Any, Dict, Optional

from .adapters import OnchainOSAdapter
from .models import InputError, read_input, validate_check_input
from .policy import evaluate, get_policy
from .render import dumps_json, error_payload


def main(argv: Optional[list] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.command == "check":
        return _cmd_check(args)
    if args.command == "policy":
        return _cmd_policy(args)
    if args.command == "self-test":
        return _cmd_self_test()

    parser.print_help()
    return 2


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="agent-risk-firewall",
        description="Pre-trade risk firewall for Agentic Wallet swaps on X Layer and Solana.",
    )
    subparsers = parser.add_subparsers(dest="command")

    check = subparsers.add_parser("check", help="Evaluate a proposed trade or approval.")
    check.add_argument("--input", required=True, help="Path to input JSON, or '-' for stdin.")
    check.add_argument("--format", default="json", choices=["json"], help="Output format.")

    policy = subparsers.add_parser("policy", help="Print the active policy.")
    policy.add_argument(
        "--profile",
        default="balanced",
        help="Policy profile: balanced, strict, competition, or degen-small-size.",
    )

    subparsers.add_parser("self-test", help="Run local policy checks without live assets.")
    return parser


def _cmd_check(args: argparse.Namespace) -> int:
    try:
        payload = read_input(args.input)
        context, validation_findings = validate_check_input(payload)
    except OSError as exc:
        print(dumps_json(error_payload("INPUT_READ_FAILED", "Could not read input.", [str(exc)])))
        return 2
    except InputError as exc:
        print(dumps_json(error_payload(exc.code, exc.message, exc.details)))
        return 2

    adapter = OnchainOSAdapter()
    evidence = adapter.collect(context)
    result = evaluate(context, evidence, validation_findings, context.get("policyProfile", "balanced"))
    print(dumps_json(result))
    return 0


def _cmd_policy(args: argparse.Namespace) -> int:
    print(dumps_json(get_policy(args.profile)))
    return 0


def _cmd_self_test() -> int:
    cases = [_fixture_allow(), _fixture_warn(), _fixture_block()]
    results = []
    for name, context, evidence in cases:
        result = evaluate(context, evidence, [], "balanced")
        results.append({"case": name, "verdict": result["verdict"], "riskScore": result["riskScore"]})

    passed = (
        results[0]["verdict"] == "allow"
        and results[1]["verdict"] == "warn"
        and results[2]["verdict"] == "block"
    )
    print(dumps_json({"status": "pass" if passed else "fail", "results": results}))
    return 0 if passed else 1


def _base_context() -> Dict[str, Any]:
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
        "amountInUsd": None,
        "walletValueUsd": None,
        "quote": {"slippagePct": 0.5, "priceImpactPct": 0.4},
        "tx": {},
        "policyProfile": "balanced",
    }


def _fixture_allow():
    return (
        "allow-low-risk",
        _base_context(),
        {
            "tokenScan": {"status": "ok", "data": {"riskLevel": "LOW"}},
            "tokenReport": {"status": "ok", "data": {"liquidityUsd": 500000}},
            "txScan": {"status": "skipped"},
            "simulation": {"status": "skipped"},
        },
    )


def _fixture_warn():
    context = _base_context()
    context["quote"] = {"slippagePct": 4, "priceImpactPct": 1}
    return (
        "warn-slippage",
        context,
        {
            "tokenScan": {"status": "ok", "data": {"riskLevel": "LOW"}},
            "tokenReport": {"status": "ok", "data": {"liquidityUsd": 500000}},
            "txScan": {"status": "skipped"},
            "simulation": {"status": "skipped"},
        },
    )


def _fixture_block():
    context = _base_context()
    return (
        "block-critical-token",
        context,
        {
            "tokenScan": {"status": "ok", "data": {"riskLevel": "CRITICAL"}},
            "tokenReport": {"status": "ok", "data": {"liquidityUsd": 500000}},
            "txScan": {"status": "skipped"},
            "simulation": {"status": "skipped"},
        },
    )


if __name__ == "__main__":
    raise SystemExit(main())
