#!/usr/bin/env python
"""Plugin Store CLI version checker + auto-updater.
Compatible with Python 2.6+ and Python 3.x.
Called by launcher.sh on every CLI invocation.

Update strategy (dual-path, both non-blocking background):
  1. npx skills add  → updates SKILL.md for next AI agent session
  2. curl binary      → updates CLI binary for next CLI invocation
"""
import os, sys, json, time, subprocess, tempfile

CACHE_DIR = os.path.join(os.path.expanduser("~"), ".plugin-store", "version-cache")
REGISTRY_URL = "https://raw.githubusercontent.com/okx/plugin-store/main/registry.json"
CHECK_INTERVAL = 3600  # 1 hour


def check(name, current_version):
    cache_file = os.path.join(CACHE_DIR, name)

    # Cache check
    try:
        if os.path.exists(cache_file):
            if time.time() - os.path.getmtime(cache_file) < CHECK_INTERVAL:
                return
    except Exception:
        pass

    # Fetch latest version
    try:
        from urllib.request import urlopen, Request
    except ImportError:
        from urllib2 import urlopen, Request

    try:
        req = Request(REGISTRY_URL, headers={"User-Agent": "plugin-store-updater"})
        resp = urlopen(req, timeout=5)
        data = json.loads(resp.read().decode("utf-8"))

        latest = None
        for p in data.get("plugins", []):
            if p.get("name") == name:
                latest = p.get("version")
                break

        if not latest:
            return

        # Update cache
        try:
            os.makedirs(CACHE_DIR)
        except OSError:
            pass
        try:
            with open(cache_file, "w") as f:
                f.write(latest)
        except Exception:
            pass

        if latest == current_version:
            return

        # Version outdated — decide action based on terminal mode
        is_interactive = hasattr(sys.stdin, 'isatty') and sys.stdin.isatty()

        if is_interactive:
            sys.stderr.write(
                "\n\033[33m" + name + " v" + current_version
                + " -> v" + latest + " available. Update now? [Y/n] \033[0m"
            )
            sys.stderr.flush()
            try:
                answer = sys.stdin.readline().strip().lower()
                if answer == "n":
                    return
            except Exception:
                pass
            sys.stderr.write("\033[33m   Updating in background...\033[0m\n")
            sys.stderr.flush()
        else:
            sys.stderr.write(
                "\033[33m" + name + " v" + current_version
                + " -> v" + latest + " updating in background...\033[0m\n"
            )
            sys.stderr.flush()

        # Background update: dual-path (npx skills add + binary download)
        update_script = """#!/bin/sh
# Path 1: Update skill files (SKILL.md) for next AI agent session
npx skills add okx/plugin-store --skill {name} --yes --global >/dev/null 2>&1 || true

# Path 2: Download new binary for immediate CLI update
OS=$(uname -s | tr A-Z a-z)
ARCH=$(uname -m)
EXT=""
case "${{OS}}_${{ARCH}}" in
  darwin_arm64)  TARGET="aarch64-apple-darwin" ;;
  darwin_x86_64) TARGET="x86_64-apple-darwin" ;;
  linux_x86_64)  TARGET="x86_64-unknown-linux-musl" ;;
  linux_i686)    TARGET="i686-unknown-linux-musl" ;;
  linux_aarch64) TARGET="aarch64-unknown-linux-musl" ;;
  linux_armv7l)  TARGET="armv7-unknown-linux-musleabihf" ;;
  mingw*_x86_64|msys*_x86_64|cygwin*_x86_64)   TARGET="x86_64-pc-windows-msvc"; EXT=".exe" ;;
  mingw*_i686|msys*_i686|cygwin*_i686)           TARGET="i686-pc-windows-msvc"; EXT=".exe" ;;
  mingw*_aarch64|msys*_aarch64|cygwin*_aarch64)  TARGET="aarch64-pc-windows-msvc"; EXT=".exe" ;;
esac

if [ -n "$TARGET" ]; then
  CORE="$HOME/.local/bin/.{name}-core${{EXT}}"
  TMP="${{CORE}}.update-tmp"
  URL="https://github.com/okx/plugin-store/releases/download/plugins/{name}@{latest}/{name}-${{TARGET}}${{EXT}}"
  if curl -fsSL "$URL" -o "$TMP" 2>/dev/null; then
    chmod +x "$TMP"
    mv -f "$TMP" "$CORE"
    # Update version marker
    mkdir -p "$HOME/.plugin-store/managed"
    echo "{latest}" > "$HOME/.plugin-store/managed/{name}"
  else
    rm -f "$TMP" 2>/dev/null
  fi
fi
""".format(name=name, latest=latest)

        # Write update script to temp file and execute in background
        devnull = open(os.devnull, "w")
        try:
            script_dir = os.path.join(os.path.expanduser("~"), ".plugin-store")
            try:
                os.makedirs(script_dir)
            except OSError:
                pass
            script_path = os.path.join(script_dir, "update-" + name + ".sh")
            with open(script_path, "w") as f:
                f.write(update_script)
            os.chmod(script_path, 0o755)
            subprocess.Popen(
                ["sh", script_path],
                stdout=devnull,
                stderr=devnull,
                close_fds=True
            )
        except Exception:
            pass

    except Exception:
        pass


if __name__ == "__main__":
    if len(sys.argv) == 3:
        check(sys.argv[1], sys.argv[2])
