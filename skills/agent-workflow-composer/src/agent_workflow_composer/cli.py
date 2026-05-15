import argparse
from typing import Optional

from .composer import build_plan, template, validate_payload
from .models import InputError, read_input
from .render import dumps_json, error_payload


def main(argv: Optional[list] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.command == "plan":
        return _cmd_plan(args)
    if args.command == "validate":
        return _cmd_validate(args)
    if args.command == "template":
        return _cmd_template(args)
    if args.command == "self-test":
        return _cmd_self_test()

    parser.print_help()
    return 2


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="agent-workflow-composer",
        description="Compose safe multi-plugin Agentic Wallet workflows before execution.",
    )
    subparsers = parser.add_subparsers(dest="command")

    plan = subparsers.add_parser("plan", help="Build a guarded workflow plan from an intent request.")
    plan.add_argument("--input", required=True, help="Path to input JSON, or '-' for stdin.")
    plan.add_argument("--format", default="json", choices=["json"], help="Output format.")

    validate = subparsers.add_parser("validate", help="Validate a workflow request or plan.")
    validate.add_argument("--input", required=True, help="Path to input JSON, or '-' for stdin.")
    validate.add_argument("--format", default="json", choices=["json"], help="Output format.")

    template_cmd = subparsers.add_parser("template", help="Print a starter workflow request template.")
    template_cmd.add_argument("--name", default="guarded-swap", help="guarded-swap, competition-trade, or approval-review.")
    template_cmd.add_argument("--format", default="json", choices=["json"], help="Output format.")

    subparsers.add_parser("self-test", help="Run local composer checks.")
    return parser


def _cmd_plan(args: argparse.Namespace) -> int:
    try:
        payload = read_input(args.input)
        print(dumps_json(build_plan(payload)))
        return 0
    except OSError as exc:
        print(dumps_json(error_payload("INPUT_READ_FAILED", "Could not read input.", [str(exc)])))
        return 2
    except InputError as exc:
        print(dumps_json(error_payload(exc.code, exc.message, exc.details)))
        return 2


def _cmd_validate(args: argparse.Namespace) -> int:
    try:
        payload = read_input(args.input)
        result = validate_payload(payload)
        print(dumps_json(result))
        return 0 if result.get("ok") else 1
    except OSError as exc:
        print(dumps_json(error_payload("INPUT_READ_FAILED", "Could not read input.", [str(exc)])))
        return 2
    except InputError as exc:
        print(dumps_json(error_payload(exc.code, exc.message, exc.details)))
        return 2


def _cmd_template(args: argparse.Namespace) -> int:
    payload = template(args.name)
    print(dumps_json(payload))
    return 1 if "error" in payload else 0


def _cmd_self_test() -> int:
    request = template("guarded-swap")
    plan = build_plan(request)
    validation = validate_payload(plan)
    checks = [
        {"case": "build-plan", "ok": plan["validation"]["ok"] is True},
        {"case": "validate-plan", "ok": validation["ok"] is True},
        {"case": "firewall-before-execution", "ok": _firewall_precedes_execution(plan)},
    ]
    passed = all(item["ok"] for item in checks)
    print(dumps_json({"status": "pass" if passed else "fail", "checks": checks}))
    return 0 if passed else 1


def _firewall_precedes_execution(plan):
    steps = plan.get("steps") or []
    ids = [step.get("id") for step in steps]
    if "execute_after_confirmation" not in ids:
        return True
    return ids.index("risk_firewall_check") < ids.index("execute_after_confirmation")


if __name__ == "__main__":
    raise SystemExit(main())
