#!/usr/bin/env bash
# Mock onchainos binary for hyperliquid-plugin integration tests.
#
# Env vars:
#   MOCK_ONCHAINOS_CALL_LOG        — append one JSON line per invocation
#   MOCK_ONCHAINOS_WALLET          — address returned by `wallet addresses`
#   MOCK_ONCHAINOS_GRANT_RESULT    — ok | deny | hang | badjson | crash | (unset = old CLI)
#   MOCK_ONCHAINOS_GRANT_REASON    — reason string for the deny case
#   MOCK_ONCHAINOS_GRANT_HANG_SECS — sleep length for the hang case
#
# The grant-file path in the `crash` case is deliberately secret-looking: the plugin
# must never surface it, and a test asserts on its absence.

set -uo pipefail

WALLET="${MOCK_ONCHAINOS_WALLET:-0xDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF}"
GRANT_FILE_MARKER="/home/user/.onchainos/grants/job-secret.json"

ARGS_JSON="["
FIRST=1
for arg in "$@"; do
  if [ $FIRST -eq 0 ]; then ARGS_JSON="$ARGS_JSON,"; fi
  ARGS_JSON="$ARGS_JSON\"$(printf '%s' "$arg" | sed 's/"/\\"/g')\""
  FIRST=0
done
ARGS_JSON="$ARGS_JSON]"

if [ -n "${MOCK_ONCHAINOS_CALL_LOG:-}" ]; then
  echo "{\"args\":$ARGS_JSON}" >> "$MOCK_ONCHAINOS_CALL_LOG"
fi

case "$*" in

  *"agent autotrade-grant-check"*)
    case "${MOCK_ONCHAINOS_GRANT_RESULT:-}" in
      ok)
        printf '{"ok":true}\n'
        ;;
      deny)
        printf '{"ok":false,"reason":"%s"}\n' "${MOCK_ONCHAINOS_GRANT_REASON:-per-trade cap exceeded}"
        exit 1
        ;;
      hang)
        sleep "${MOCK_ONCHAINOS_GRANT_HANG_SECS:-30}"
        printf '{"ok":true}\n'
        ;;
      badjson)
        printf 'this is not json\n'
        ;;
      crash)
        echo "thread 'main' panicked: failed to open ${GRANT_FILE_MARKER}: permission denied" >&2
        exit 101
        ;;
      *)
        # An onchainos predating the subcommand.
        echo "error: unrecognized subcommand 'autotrade-grant-check'" >&2
        exit 2
        ;;
    esac
    ;;

  *"wallet addresses"*)
    printf '{"ok":true,"data":{"evm":[{"address":"%s","type":"evm"}]}}\n' "$WALLET"
    ;;

  *"wallet sign-message"*)
    # 65 bytes = 130 hex chars (r ‖ s ‖ v); the plugin rejects any other length.
    SIG=$(printf '1b%.0s' $(seq 1 65))
    printf '{"ok":true,"data":{"signature":"0x%s"}}\n' "$SIG"
    ;;

  *"wallet report-plugin-info"*)
    printf '{"ok":true}\n'
    ;;

  *"--version"*)
    printf 'mock-onchainos 0.0.0 (test fixture)\n'
    ;;

  *)
    echo "{\"ok\":false,\"error\":\"mock_onchainos: unrecognised command: $*\"}" >&2
    exit 1
    ;;

esac
