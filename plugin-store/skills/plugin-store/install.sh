#!/bin/sh
set -e

# ──────────────────────────────────────────────────────────────
# plugin-store installer / updater (macOS / Linux)
#
# Usage:
#   curl -sSL https://raw.githubusercontent.com/okx/plugin-store/main/skills/plugin-store/install.sh | sh
#
# Behavior:
#   - Fetches latest stable release from GitHub, compares with local
#     version, installs/upgrades if needed.
#   - Caches the last check timestamp. Skips GitHub API calls if
#     checked within the last 12 hours.
#
# Supported platforms:
#   macOS  : x86_64 (Intel), arm64 (Apple Silicon)
#   Linux  : x86_64, i686, aarch64, armv7l
#   Windows: see install.ps1 (PowerShell)
# ──────────────────────────────────────────────────────────────

REPO="okx/plugin-store"
BINARY="plugin-store"
INSTALL_DIR="$HOME/.local/bin"
CACHE_DIR="$HOME/.plugin-store"
CACHE_FILE="$CACHE_DIR/last_check"
CACHE_TTL=43200  # 12 hours in seconds

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
    *) echo "Unsupported OS" >&2; exit 1 ;;
  esac
}

# ── Cache helpers ────────────────────────────────────────────
is_cache_fresh() {
  [ -f "$CACHE_FILE" ] || return 1
  cached_ts=$(head -1 "$CACHE_FILE" 2>/dev/null)
  [ -z "$cached_ts" ] && return 1
  now=$(date +%s)
  elapsed=$((now - cached_ts))
  [ "$elapsed" -lt "$CACHE_TTL" ]
}

# Read the latest version stored in cache (line 2)
cached_latest_version() {
  sed -n '2p' "$CACHE_FILE" 2>/dev/null
}

write_cache() {
  mkdir -p "$CACHE_DIR"
  # Line 1: timestamp, Line 2: latest known version
  printf '%s\n%s\n' "$(date +%s)" "${1:-}" > "$CACHE_FILE"
}

# ── Version helpers ──────────────────────────────────────────
get_local_version() {
  if [ -x "$INSTALL_DIR/$BINARY" ]; then
    "$INSTALL_DIR/$BINARY" --version 2>/dev/null | awk '{print $2}'
  fi
}

strip_prerelease() {
  echo "$1" | sed 's/-.*//'
}

_ver_field() {
  echo "$1" | cut -d. -f"$2"
}

semver_gt() {
  base1=$(strip_prerelease "$1")
  base2=$(strip_prerelease "$2")
  pre1=$(echo "$1" | sed -n 's/[^-]*-//p')
  pre2=$(echo "$2" | sed -n 's/[^-]*-//p')

  for i in 1 2 3; do
    f1=$(_ver_field "$base1" "$i")
    f2=$(_ver_field "$base2" "$i")
    f1=${f1:-0}
    f2=${f2:-0}
    [ "$f1" -gt "$f2" ] 2>/dev/null && return 0
    [ "$f1" -lt "$f2" ] 2>/dev/null && return 1
  done

  [ -z "$pre1" ] && [ -z "$pre2" ] && return 1
  [ -z "$pre1" ] && return 0
  [ -z "$pre2" ] && return 1

  num1=$(echo "$pre1" | grep -o '[0-9]*$')
  num2=$(echo "$pre2" | grep -o '[0-9]*$')
  num1=${num1:-0}
  num2=${num2:-0}
  [ "$num1" -gt "$num2" ] 2>/dev/null && return 0
  return 1
}

# ── GitHub API helpers ───────────────────────────────────────
get_latest_stable_version() {
  response=$(curl -sSL --max-time 10 "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null) || true
  ver=$(echo "$response" | grep -o '"tag_name": *"v[^"]*"' | head -1 | sed 's/.*"v\([^"]*\)".*/\1/')
  if [ -z "$ver" ]; then
    echo "Error: could not fetch latest version from GitHub." >&2
    echo "Check your network connection or install manually from https://github.com/${REPO}" >&2
    exit 1
  fi
  echo "$ver"
}

# ── Binary installer ─────────────────────────────────────────
install_binary() {
  target=$(get_target)
  if [ -z "$target" ]; then
    exit 1
  fi
  tag="$1"

  binary_name="${BINARY}-${target}"
  url="https://github.com/${REPO}/releases/download/${tag}/${binary_name}"
  checksums_url="https://github.com/${REPO}/releases/download/${tag}/checksums.txt"

  echo "Installing ${BINARY} ${tag} (${target})..."

  tmpdir=$(mktemp -d)
  trap 'rm -rf "$tmpdir"' EXIT

  curl -fsSL "$url" -o "$tmpdir/$binary_name"
  curl -fsSL "$checksums_url" -o "$tmpdir/checksums.txt" 2>/dev/null || true

  if [ -f "$tmpdir/checksums.txt" ]; then
    expected_hash=$(grep "$binary_name" "$tmpdir/checksums.txt" 2>/dev/null | awk '{print $1}')
    if [ -n "$expected_hash" ]; then
      if command -v sha256sum >/dev/null 2>&1; then
        actual_hash=$(sha256sum "$tmpdir/$binary_name" | awk '{print $1}')
      elif command -v shasum >/dev/null 2>&1; then
        actual_hash=$(shasum -a 256 "$tmpdir/$binary_name" | awk '{print $1}')
      fi

      if [ -n "$actual_hash" ] && [ "$actual_hash" != "$expected_hash" ]; then
        echo "Error: checksum mismatch!" >&2
        echo "  Expected: $expected_hash" >&2
        echo "  Got:      $actual_hash" >&2
        echo "The downloaded file may have been tampered with. Aborting." >&2
        exit 1
      fi
      echo "Checksum verified."
    fi
  fi

  mkdir -p "$INSTALL_DIR"
  mv "$tmpdir/$binary_name" "$INSTALL_DIR/$BINARY"
  chmod +x "$INSTALL_DIR/$BINARY"

  echo "Installed ${BINARY} ${tag} to ${INSTALL_DIR}/${BINARY}"
}

# ── PATH setup ───────────────────────────────────────────────
ensure_in_path() {
  case ":$PATH:" in
    *":$INSTALL_DIR:"*) return 0 ;;
  esac

  EXPORT_LINE="export PATH=\"\$HOME/.local/bin:\$PATH\""

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

  if [ -f "$profile" ] && grep -qF '$HOME/.local/bin' "$profile" 2>/dev/null; then
    return 0
  fi

  echo "" >> "$profile"
  echo "# Added by plugin-store installer" >> "$profile"
  echo "$EXPORT_LINE" >> "$profile"

  export PATH="$INSTALL_DIR:$PATH"

  echo ""
  echo "Added $INSTALL_DIR to PATH in $profile"
  echo "To start using '${BINARY}' now, run:"
  echo ""
  echo "  source $profile"
  echo ""
  echo "Or simply open a new terminal window."
}

# ── Main ─────────────────────────────────────────────────────
main() {
  local_ver=$(get_local_version)

  # Fast path: binary exists, cache is fresh, AND cached latest == local version
  if [ -n "$local_ver" ] && is_cache_fresh; then
    cached_ver=$(cached_latest_version)
    if [ -n "$cached_ver" ] && [ "$local_ver" = "$cached_ver" ]; then
      echo "${BINARY} ${local_ver} already up to date."
      return 0
    fi
    # Version mismatch (local != cached latest) — fall through to re-check
  fi

  latest_stable=$(get_latest_stable_version)

  if [ -z "$local_ver" ]; then
    target_ver="$latest_stable"
  elif [ "$local_ver" = "$latest_stable" ]; then
    write_cache "$latest_stable"
    echo "${BINARY} ${local_ver} already up to date."
    return 0
  else
    if semver_gt "$latest_stable" "$local_ver"; then
      target_ver="$latest_stable"
    else
      write_cache "$latest_stable"
      echo "${BINARY} ${local_ver} already up to date."
      return 0
    fi
  fi

  if [ -n "$local_ver" ]; then
    echo "Updating ${BINARY} from ${local_ver} to ${target_ver}..."
  fi

  install_binary "v${target_ver}"
  write_cache "$target_ver"
  ensure_in_path
}

main
