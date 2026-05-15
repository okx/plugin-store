import json
import re
import sys
from decimal import Decimal, InvalidOperation
from typing import Any, Dict, List, Optional, Tuple


SUPPORTED_CHAINS = {"xlayer", "solana"}
SUPPORTED_OPERATIONS = {"buy", "sell", "swap", "approval"}
EVM_ADDRESS_RE = re.compile(r"^0x[a-fA-F0-9]{40}$")
SOLANA_ADDRESS_RE = re.compile(r"^[1-9A-HJ-NP-Za-km-z]{32,44}$")


class InputError(Exception):
    def __init__(self, code: str, message: str, details: Optional[List[str]] = None):
        super().__init__(message)
        self.code = code
        self.message = message
        self.details = details or []


def read_input(path: str) -> Dict[str, Any]:
    if path == "-":
        raw = sys.stdin.read()
    else:
        with open(path, "r", encoding="utf-8") as handle:
            raw = handle.read()
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise InputError("INVALID_JSON", "Input is not valid JSON.", [str(exc)])
    if not isinstance(payload, dict):
        raise InputError("INVALID_INPUT", "Input JSON must be an object.")
    return payload


def normalize_chain(value: Any) -> str:
    text = str(value or "").strip().lower().replace("_", "-")
    aliases = {
        "x-layer": "xlayer",
        "x layer": "xlayer",
        "196": "xlayer",
        "sol": "solana",
        "501": "solana",
    }
    return aliases.get(text, text)


def is_evm_address(value: Any) -> bool:
    return isinstance(value, str) and bool(EVM_ADDRESS_RE.match(value))


def is_solana_address(value: Any) -> bool:
    return isinstance(value, str) and bool(SOLANA_ADDRESS_RE.match(value))


def address_matches_chain(chain: str, value: Any) -> bool:
    if not value:
        return False
    if chain == "xlayer":
        return is_evm_address(value)
    if chain == "solana":
        return is_solana_address(value)
    return False


def optional_decimal(value: Any) -> Optional[Decimal]:
    if value is None or value == "":
        return None
    try:
        return Decimal(str(value))
    except (InvalidOperation, ValueError):
        return None


def _normalize_token(value: Any) -> Optional[Dict[str, Any]]:
    if value is None:
        return None
    if not isinstance(value, dict):
        return {"raw": value}
    token = dict(value)
    if "symbol" in token and isinstance(token["symbol"], str):
        token["symbol"] = token["symbol"].strip()
    return token


def target_token(context: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    operation = context.get("operation")
    if operation in ("buy", "swap"):
        return context.get("tokenOut") or context.get("tokenIn")
    if operation in ("sell", "approval"):
        return context.get("tokenIn") or context.get("tokenOut")
    return context.get("tokenOut") or context.get("tokenIn")


def validate_check_input(payload: Dict[str, Any]) -> Tuple[Dict[str, Any], List[Dict[str, Any]]]:
    fatal: List[str] = []
    findings: List[Dict[str, Any]] = []

    chain = normalize_chain(payload.get("chain"))
    if not chain:
        fatal.append("Missing required field: chain.")
    elif chain not in SUPPORTED_CHAINS:
        fatal.append("Unsupported chain. MVP supports only xlayer and solana.")

    operation = str(payload.get("operation") or "").strip().lower()
    if not operation:
        fatal.append("Missing required field: operation.")
    elif operation not in SUPPORTED_OPERATIONS:
        fatal.append("Unsupported operation. Use buy, sell, swap, or approval.")

    wallet_address = payload.get("walletAddress")
    if not wallet_address:
        fatal.append("Missing required field: walletAddress.")

    token_in = _normalize_token(payload.get("tokenIn"))
    token_out = _normalize_token(payload.get("tokenOut"))
    if token_in is None and token_out is None:
        fatal.append("At least one of tokenIn or tokenOut is required.")

    quote = payload.get("quote") if isinstance(payload.get("quote"), dict) else {}
    tx = payload.get("tx") if isinstance(payload.get("tx"), dict) else {}
    approval = payload.get("approval") if isinstance(payload.get("approval"), dict) else {}
    external_evidence = payload.get("externalEvidence") if isinstance(payload.get("externalEvidence"), dict) else {}
    competition = payload.get("competition") if isinstance(payload.get("competition"), dict) else {}

    context: Dict[str, Any] = {
        "chain": chain,
        "operation": operation,
        "walletAddress": wallet_address,
        "tokenIn": token_in,
        "tokenOut": token_out,
        "amountIn": payload.get("amountIn"),
        "amountInUsd": optional_decimal(payload.get("amountInUsd")),
        "walletValueUsd": optional_decimal(payload.get("walletValueUsd")),
        "quote": quote,
        "tx": tx,
        "approval": approval,
        "externalEvidence": external_evidence,
        "competition": competition,
        "policyProfile": str(payload.get("policyProfile") or "balanced").strip().lower(),
    }

    if fatal:
        raise InputError("INVALID_INPUT", "Input failed required-field validation.", fatal)

    if not address_matches_chain(chain, wallet_address):
        findings.append(
            {
                "code": "ADDRESS_CHAIN_MISMATCH",
                "severity": "block",
                "score": 95,
                "message": "Wallet address format does not match the requested chain.",
            }
        )

    for field_name, token in (("tokenIn", token_in), ("tokenOut", token_out)):
        if not token or not isinstance(token, dict):
            continue
        address = token.get("address")
        if address and not address_matches_chain(chain, address):
            findings.append(
                {
                    "code": "ADDRESS_CHAIN_MISMATCH",
                    "severity": "block",
                    "score": 95,
                    "message": "%s address format does not match the requested chain." % field_name,
                }
            )

    tx_from = tx.get("from")
    tx_to = tx.get("to")
    if tx_from and not address_matches_chain(chain, tx_from):
        findings.append(
            {
                "code": "ADDRESS_CHAIN_MISMATCH",
                "severity": "block",
                "score": 95,
                "message": "Transaction from address format does not match the requested chain.",
            }
        )
    if tx_to and chain == "xlayer" and not address_matches_chain(chain, tx_to):
        findings.append(
            {
                "code": "ADDRESS_CHAIN_MISMATCH",
                "severity": "block",
                "score": 95,
                "message": "Transaction recipient address format does not match X Layer.",
            }
        )

    spender = approval.get("spender") or tx.get("spender")
    if spender and chain == "xlayer" and not address_matches_chain(chain, spender):
        findings.append(
            {
                "code": "ADDRESS_CHAIN_MISMATCH",
                "severity": "block",
                "score": 95,
                "message": "Approval spender address format does not match X Layer.",
            }
        )

    return context, findings
