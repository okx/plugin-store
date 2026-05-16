import argparse
from typing import Optional

from .models import InputError, read_input
from .planner import build_plan, dry_run, execute, template, validate_plan
from .render import dumps_json, error_payload


def main(argv: Optional[list] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.command == "plan":
        return _with_input(args, build_plan, success_code=0)
    if args.command == "dry-run":
        return _with_input(args, dry_run, success_code=0)
    if args.command == "execute":
        return _with_input(args, execute, success_code=0)
    if args.command == "validate":
        return _cmd_validate(args)
    if args.command == "template":
        payload = template(args.name)
        print(dumps_json(payload))
        return 1 if "error" in payload else 0
    if args.command == "self-test":
        return _cmd_self_test()

    parser.print_help()
    return 2


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="safe-competition-trading-agent",
        description="Plan and guard OKX Agentic Wallet competition trades.",
    )
    subparsers = parser.add_subparsers(dest="command")

    for command, help_text in (
        ("plan", "Build a competition-aware trading plan."),
        ("dry-run", "Evaluate a dry-run trade context without execution."),
        ("execute", "Return a guarded execution command only after confirmation and risk allow/warn."),
        ("validate", "Validate a generated plan."),
    ):
        sub = subparsers.add_parser(command, help=help_text)
        sub.add_argument("--input", required=True, help="Path to input JSON, or '-' for stdin.")
        sub.add_argument("--format", default="json", choices=["json"], help="Output format.")

    template_cmd = subparsers.add_parser("template", help="Print a starter request template.")
    template_cmd.add_argument("--name", default="competition-safe-swap", help="competition-safe-swap or rank-optimizer.")
    template_cmd.add_argument("--format", default="json", choices=["json"], help="Output format.")

    subparsers.add_parser("self-test", help="Run local strategy checks.")
    return parser


def _with_input(args: argparse.Namespace, handler, success_code: int) -> int:
    try:
        payload = read_input(args.input)
        print(dumps_json(handler(payload)))
        return success_code
    except OSError as exc:
        print(dumps_json(error_payload("INPUT_READ_FAILED", "Could not read input.", [str(exc)])))
        return 2
    except InputError as exc:
        print(dumps_json(error_payload(exc.code, exc.message, exc.details)))
        return 2


def _cmd_validate(args: argparse.Namespace) -> int:
    try:
        payload = read_input(args.input)
        plan = payload if "steps" in payload else build_plan(payload)
        result = validate_plan(plan)
        print(dumps_json(result))
        return 0 if result.get("ok") else 1
    except OSError as exc:
        print(dumps_json(error_payload("INPUT_READ_FAILED", "Could not read input.", [str(exc)])))
        return 2
    except InputError as exc:
        print(dumps_json(error_payload(exc.code, exc.message, exc.details)))
        return 2


def _cmd_self_test() -> int:
    payload = template("competition-safe-swap")
    plan = build_plan(payload)
    dry = dry_run(
        dict(
            payload,
            tokenOut={"symbol": "MEME", "liquidityUsd": "150000", "riskLevel": "LOW"},
            competition={"active": True, "joined": True, "chainName": "X Layer", "supportedChains": ["xlayer", "solana"]},
            quote={"slippagePct": 1, "priceImpactPct": 1},
            riskVerdict={"verdict": "allow", "reasons": []},
        )
    )
    validation = validate_plan(plan)
    checks = [
        {"case": "build-plan", "ok": plan["validation"]["ok"] is True},
        {"case": "validate-plan", "ok": validation["ok"] is True},
        {"case": "dry-run", "ok": dry["status"] == "ready"},
        {"case": "no-execute-in-dry-run", "ok": "execute_after_confirmation" not in [step["id"] for step in plan["steps"]]},
    ]
    passed = all(item["ok"] for item in checks)
    print(dumps_json({"status": "pass" if passed else "fail", "checks": checks}))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
