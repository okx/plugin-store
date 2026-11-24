#!/bin/sh
# Plugin Store Universal Launcher
# All plugin CLIs symlink to this file. Detects plugin name from $0.
PLUGIN_NAME=$(basename "$0")
CORE="$(dirname "$0")/.${PLUGIN_NAME}-core"
CHECKER="$HOME/.plugin-store/update-checker.py"
VERSION_FILE="$HOME/.plugin-store/managed/${PLUGIN_NAME}"
VERSION=$(cat "$VERSION_FILE" 2>/dev/null || echo "0")

# Version check (non-blocking for non-interactive, prompt for interactive)
if [ -f "$CHECKER" ]; then
  if command -v python3 >/dev/null 2>&1; then
    python3 "$CHECKER" "$PLUGIN_NAME" "$VERSION"
  elif command -v python >/dev/null 2>&1; then
    python "$CHECKER" "$PLUGIN_NAME" "$VERSION"
  fi
fi

# Execute core binary/script
exec "$CORE" "$@"
