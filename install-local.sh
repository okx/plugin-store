#!/bin/sh
set -e

# ──────────────────────────────────────────────────────────────
# plugin-store local installer (macOS / Linux)
#
# Usage:
#   curl -sSL https://raw.githubusercontent.com/okx/plugin-store/main/install-local.sh | sh
#
# Installs plugin-store + 4 official strategies into ~/.cargo/bin
#
# Binaries:
#   - plugin-store
#   - dapp-hyperliquid
#   - strategy-memepump-scanner
#   - strategy-ranking-sniper
#   - strategy-signal-tracker
# ──────────────────────────────────────────────────────────────

REPO="ganlinux/plugin-store"
INSTALL_DIR="$HOME/.cargo/bin"

BINARIES="plugin-store dapp-hyperliquid strategy-memepump-scanner strategy-ranking-sniper strategy-signal-tracker"

# ── Platform detection ───────────────────────────────────────
get_target() {
  os=$(uname -s)
  arch=$(uname -m)

  case "$os" in
    Darwin)
      case "$arch" in
        x86_64) echo "x86_64-apple-darwin" ;;
        arm64)  echo "aarch64-apple-darwin" ;;
        *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
      esac
      ;;
    Linux)
      case "$arch" in
        x86_64)  echo "x86_64-unknown-linux-gnu" ;;
        i686)    echo "i686-unknown-linux-gnu" ;;
        aarch64) echo "aarch64-unknown-linux-gnu" ;;
        armv7l)  echo "armv7-unknown-linux-gnueabihf" ;;
        *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
      esac
      ;;
    *) echo "Unsupported OS: $os" >&2; exit 1 ;;
  esac
}

# ── GitHub API ───────────────────────────────────────────────
get_latest_version() {
  response=$(curl -sSL --max-time 10 "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null) || true
  ver=$(echo "$response" | grep -o '"tag_name": *"v[^"]*"' | head -1 | sed 's/.*"v\([^"]*\)".*/\1/')
  if [ -z "$ver" ]; then
    echo "Error: could not fetch latest version from GitHub." >&2
    exit 1
  fi
  echo "$ver"
}

# ── Checksum verification ────────────────────────────────────
verify_checksum() {
  file="$1"
  name="$2"
  checksums_file="$3"

  [ -f "$checksums_file" ] || return 0

  expected=$(grep "$name" "$checksums_file" 2>/dev/null | awk '{print $1}')
  [ -z "$expected" ] && return 0

  if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$file" | awk '{print $1}')
  elif command -v shasum >/dev/null 2>&1; then
    actual=$(shasum -a 256 "$file" | awk '{print $1}')
  else
    return 0
  fi

  if [ "$actual" != "$expected" ]; then
    echo "Error: checksum mismatch for $name!" >&2
    exit 1
  fi
  echo "  Checksum OK: $name"
}

# ── PATH setup ───────────────────────────────────────────────
ensure_in_path() {
  case ":$PATH:" in
    *":$INSTALL_DIR:"*) return 0 ;;
  esac

  EXPORT_LINE="export PATH=\"\$HOME/.cargo/bin:\$PATH\""

  shell_name=$(basename "$SHELL" 2>/dev/null || echo "sh")
  case "$shell_name" in
    zsh)  profile="$HOME/.zshrc" ;;
    bash)
      if [ -f "$HOME/.bash_profile" ]; then
        profile="$HOME/.bash_profile"
      elif [ -f "$HOME/.bashrc" ]; then
        profile="$HOME/.bashrc"
      else
        profile="$HOME/.profile"
      fi
      ;;
    *)    profile="$HOME/.profile" ;;
  esac

  if [ -f "$profile" ] && grep -qF '$HOME/.cargo/bin' "$profile" 2>/dev/null; then
    return 0
  fi

  echo "" >> "$profile"
  echo "# Added by plugin-store installer" >> "$profile"
  echo "$EXPORT_LINE" >> "$profile"

  export PATH="$INSTALL_DIR:$PATH"

  echo ""
  echo "Added $INSTALL_DIR to PATH in $profile"
  echo "Run 'source $profile' or open a new terminal."
}

# ── Main ─────────────────────────────────────────────────────
main() {
  target=$(get_target)
  version=$(get_latest_version)
  tag="v${version}"

  echo "Installing plugin-store ${tag} + 4 strategies..."
  echo "Platform: ${target}"
  echo "Install dir: ${INSTALL_DIR}"
  echo ""

  mkdir -p "$INSTALL_DIR"

  tmpdir=$(mktemp -d)
  trap 'rm -rf "$tmpdir"' EXIT

  # Download checksums
  curl -fsSL "https://github.com/${REPO}/releases/download/${tag}/checksums.txt" \
    -o "$tmpdir/checksums.txt" 2>/dev/null || true

  # Download and install each binary
  for bin in $BINARIES; do
    asset_name="${bin}-${target}"
    url="https://github.com/${REPO}/releases/download/${tag}/${asset_name}"

    echo "Downloading ${bin}..."
    if ! curl -fsSL "$url" -o "$tmpdir/$asset_name"; then
      echo "  Warning: failed to download ${bin}, skipping." >&2
      continue
    fi

    verify_checksum "$tmpdir/$asset_name" "$asset_name" "$tmpdir/checksums.txt"

    mv "$tmpdir/$asset_name" "$INSTALL_DIR/$bin"
    chmod 777 "$INSTALL_DIR/$bin"
    echo "  Installed: ${INSTALL_DIR}/${bin}"
  done

  echo ""
  ensure_in_path

  echo ""
  echo "Done! Installed:"
  for bin in $BINARIES; do
    [ -x "$INSTALL_DIR/$bin" ] && echo "  $bin -> $INSTALL_DIR/$bin"
  done
}

main