#!/usr/bin/env bash
# Mock onchainos binary for integration testing.
#
# Behaviour is controlled by env vars:
#   MOCK_ONCHAINOS_CALL_LOG  — append each invocation (as a JSON line) to this file
#   MOCK_ONCHAINOS_WALLET    — wallet address to return for `wallet addresses` (default: 0xDEAD...BEEF)
#   MOCK_ONCHAINOS_TX_HASH   — tx hash to return for `wallet contract-call` (default: 0xABCD...1234)
#   MOCK_ONCHAINOS_FAIL_CMD  — if set, any invocation whose args contain this string returns exit-1
#
# Every call is logged as a JSON object:
#   { "args": [...], "calldata": "<0x hex if --input-data present>", "to": "<address>" }

set -euo pipefail

WALLET="${MOCK_ONCHAINOS_WALLET:-0xDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF}"
TX_HASH="${MOCK_ONCHAINOS_TX_HASH:-0xABCD1234ABCD1234ABCD1234ABCD1234ABCD1234ABCD1234ABCD1234ABCD1234}"

# Build JSON array of args for the call log
ARGS_JSON="["
FIRST=1
for arg in "$@"; do
  if [ $FIRST -eq 0 ]; then ARGS_JSON="$ARGS_JSON,"; fi
  ARGS_JSON="$ARGS_JSON\"$(echo "$arg" | sed 's/"/\\"/g')\""
  FIRST=0
done
ARGS_JSON="$ARGS_JSON]"

# Extract --to and --input-data from args
TO=""
CALLDATA=""
PREV=""
for arg in "$@"; do
  case "$PREV" in
    "--to")       TO="$arg" ;;
    "--input-data") CALLDATA="$arg" ;;
  esac
  PREV="$arg"
done

# Append call record to log file
if [ -n "${MOCK_ONCHAINOS_CALL_LOG:-}" ]; then
  echo "{\"args\":$ARGS_JSON,\"to\":\"$TO\",\"calldata\":\"$CALLDATA\"}" >> "$MOCK_ONCHAINOS_CALL_LOG"
fi

# Fail on demand (for error-path testing)
if [ -n "${MOCK_ONCHAINOS_FAIL_CMD:-}" ]; then
  case "$*" in
    *"$MOCK_ONCHAINOS_FAIL_CMD"*)
      echo '{"ok":false,"error":"mock_onchainos: forced failure for testing"}' >&2
      exit 1
      ;;
  esac
fi

# ── Dispatch on subcommand ────────────────────────────────────────────────────

case "$*" in

  *"wallet addresses"*)
    # Return a fixed EVM wallet address
    printf '{"ok":true,"data":{"evm":[{"address":"%s","type":"evm"}]}}\n' "$WALLET"
    ;;

  *"wallet contract-call"*)
    # Return a successful tx hash response
    # The test harness reads MOCK_ONCHAINOS_CALL_LOG to assert on the calldata.
    printf '{"ok":true,"data":{"txHash":"%s","chain":"137","status":"broadcast"}}\n' "$TX_HASH"
    ;;

  *"wallet sign-message"*)
    # Return a fake EIP-712 signature
    printf '{"ok":true,"data":{"signature":"0x1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b2a3c4d1b"}}\n'
    ;;

  *"wallet send"*)
    printf '{"ok":true,"data":{"txHash":"%s","chain":"137","status":"broadcast"}}\n' "$TX_HASH"
    ;;

  *"wallet balance"*)
    printf '{"ok":true,"data":{"tokens":[{"symbol":"POL","usdValue":"5.00","balance":"5.0"},{"symbol":"USDC.e","usdValue":"100.00","balance":"100.0"}]}}\n'
    ;;

  *"wallet report-plugin-info"*)
    printf '{"ok":true}\n'
    ;;

  *"--version"*)
    printf 'mock-onchainos 0.0.0 (test fixture)\n'
    ;;

  *)
    # Unknown command — log and return a generic error so tests fail clearly
    echo '{"ok":false,"error":"mock_onchainos: unrecognised command: '"$*"'"}' >&2
    exit 1
    ;;

esac
