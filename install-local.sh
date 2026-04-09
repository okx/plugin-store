#!/bin/sh
set -e

# ──────────────────────────────────────────────────────────────
# plugin-store local installer (macOS / Linux)
#
# Usage:
#   curl -sSL https://raw.githubusercontent.com/MigOKG/plugin-store/main/install-local.sh | sh
#
# Installs plugin-store CLI + all DApp binaries into ~/.cargo/bin
# ──────────────────────────────────────────────────────────────

REPO="MigOKG/plugin-store"
INSTALL_DIR="$HOME/.cargo/bin"
REGISTRY_URL="https://raw.githubusercontent.com/${REPO}/main/registry.json"

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
  # Fetch all releases and find the latest tag matching "v*" (skip plugins/* tags)
  response=$(curl -sSL --max-time 10 "https://api.github.com/repos/${REPO}/releases?per_page=100" 2>/dev/null) || true
  ver=$(echo "$response" | grep -o '"tag_name": *"v[0-9][^"]*"' | head -1 | sed 's/.*"v\([^"]*\)".*/\1/')

  # Fallback to gh CLI
  if [ -z "$ver" ] && command -v gh >/dev/null 2>&1; then
    response=$(gh api "repos/${REPO}/releases?per_page=100" 2>/dev/null) || true
    ver=$(echo "$response" | grep -o '"tag_name": *"v[0-9][^"]*"' | head -1 | sed 's/.*"v\([^"]*\)".*/\1/')
  fi

  if [ -z "$ver" ]; then
    echo "Error: could not fetch latest plugin-store version from GitHub." >&2
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

# ── Download a single binary ─────────────────────────────────
download_binary() {
  bin_name="$1"
  release_tag="$2"
  target="$3"
  tmpdir="$4"

  asset_name="${bin_name}-${target}"

  # Prefer gh CLI (handles auth automatically)
  if command -v gh >/dev/null 2>&1; then
    ok=0
    for attempt in 1 2 3; do
      gh release download "$release_tag" --repo "$REPO" \
        --pattern "$asset_name" -D "$tmpdir" --clobber 2>/dev/null && ok=1 && break
      sleep 1
    done
    if [ "$ok" -eq 0 ]; then
      echo "  Warning: failed to download ${bin_name}, skipping." >&2
      return 1
    fi
  else
    # Fallback: curl with optional GITHUB_TOKEN
    url="https://github.com/${REPO}/releases/download/${release_tag}/${asset_name}"
    auth_header=""
    [ -n "$GITHUB_TOKEN" ] && auth_header="-H \"Authorization: token ${GITHUB_TOKEN}\""
    if ! curl -fsSL --max-time 60 $auth_header "$url" -o "$tmpdir/$asset_name" 2>/dev/null; then
      echo "  Warning: failed to download ${bin_name}. Set GITHUB_TOKEN or install gh CLI." >&2
      return 1
    fi
  fi

  # Verify checksum if available
  checksum_file="$tmpdir/checksums-${bin_name}.txt"
  if command -v gh >/dev/null 2>&1; then
    gh release download "$release_tag" --repo "$REPO" \
      --pattern "checksums.txt" -D "$tmpdir" --clobber 2>/dev/null \
      && mv "$tmpdir/checksums.txt" "$checksum_file" || true
  fi
  verify_checksum "$tmpdir/$asset_name" "$asset_name" "$checksum_file"

  mv "$tmpdir/$asset_name" "$INSTALL_DIR/$bin_name"
  chmod 755 "$INSTALL_DIR/$bin_name"
  echo "Installed: ${INSTALL_DIR}/${bin_name}"
  return 0
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

# ── Parse releases and install DApp binaries ─────────────────
install_dapp_binaries() {
  target="$1"
  tmpdir="$2"
  filter="$3"

  echo "Fetching releases list..."
  releases_file="$tmpdir/releases.json"

  # Try curl first (no auth needed for public repos, up to 100 results)
  curl -fsSL --max-time 15 \
    "https://api.github.com/repos/${REPO}/releases?per_page=100" \
    -o "$releases_file" 2>/dev/null || true

  # Extract tags from curl response
  plugins=$(grep -o '"tag_name": *"plugins/[^"]*"' "$releases_file" 2>/dev/null \
    | sed 's/.*"plugins\/\([^"]*\)".*/\1/' \
    | while read entry; do
        echo "$(echo "$entry" | sed 's/@.*//')|plugins/${entry}"
      done)

  # Fallback to gh (handles pagination, auth, all releases)
  if [ -z "$plugins" ] || { [ -n "$filter" ] && ! echo "$plugins" | grep -q "^${filter}|"; }; then
    if command -v gh >/dev/null 2>&1; then
      [ -z "$plugins" ] && echo "  curl incomplete, retrying with gh..."
      [ -n "$filter" ] && echo "  '${filter}' not in curl results, retrying with gh..."
      plugins=$(gh release list --repo "$REPO" --limit 200 2>/dev/null \
        | grep -o 'plugins/[^[:space:]]*' \
        | sed 's|plugins/||' \
        | while read entry; do
            echo "$(echo "$entry" | sed 's/@.*//')|plugins/${entry}"
          done)
    fi
  fi

  if [ -z "$plugins" ]; then
    echo "No DApp releases found."
    return 0
  fi

  if [ -n "$filter" ]; then
    plugins=$(echo "$plugins" | grep "^${filter}|" || true)
    if [ -z "$plugins" ]; then
      echo "Warning: '${filter}' not found in releases." >&2
      return 1
    fi
  fi

  total=$(echo "$plugins" | grep -c '|' || echo 0)
  echo "Installing ${total} DApp binary(ies)..."
  echo ""

  installed=0
  failed=0
  echo "$plugins" | while IFS='|' read bin_name release_tag; do
    [ -z "$bin_name" ] && continue
    printf "  [%s] " "$bin_name"
    if download_binary "$bin_name" "$release_tag" "$target" "$tmpdir"; then
      installed=$((installed + 1))
    else
      failed=$((failed + 1))
    fi
  done

  echo ""
  echo "DApp binaries done."
}

# ── Main ─────────────────────────────────────────────────────
main() {
  FILTER="$1"   # optional: specific dapp name, e.g. "aave-v3"

  target=$(get_target)
  version=$(get_latest_version)
  tag="v${version}"

  echo "plugin-store installer"
  echo "Platform : ${target}"
  echo "Install  : ${INSTALL_DIR}"
  [ -n "$FILTER" ] && echo "Filter   : ${FILTER}"
  echo ""

  mkdir -p "$INSTALL_DIR"

  tmpdir=$(mktemp -d)
  trap 'rm -rf "$tmpdir"' EXIT

  # ── 1. Install plugin-store CLI (only when no filter specified) ──
  if [ -n "$FILTER" ] && [ "$FILTER" != "plugin-store" ]; then
    echo "Skipping plugin-store CLI (installing DApp only)."
    echo ""
    install_dapp_binaries "$target" "$tmpdir" "$FILTER"
    echo ""
    ensure_in_path
    echo ""
    echo "Done!"
    return 0
  fi

  echo "Installing plugin-store ${tag}..."
  ps_checksums="$tmpdir/checksums-ps.txt"
  curl -fsSL "https://github.com/${REPO}/releases/download/${tag}/checksums.txt" \
    -o "$ps_checksums" 2>/dev/null || true

  asset_name="plugin-store-${target}"
  for attempt in 1 2 3; do
    if command -v gh >/dev/null 2>&1; then
      gh release download "$tag" --repo "$REPO" \
        --pattern "$asset_name" --pattern "checksums.txt" \
        -D "$tmpdir" --clobber 2>/dev/null && \
        mv "$tmpdir/checksums.txt" "$ps_checksums" 2>/dev/null || true
    else
      url="https://github.com/${REPO}/releases/download/${tag}/${asset_name}"
      curl -fsSL "$url" -o "$tmpdir/$asset_name" || true
      curl -fsSL "https://github.com/${REPO}/releases/download/${tag}/checksums.txt" \
        -o "$ps_checksums" 2>/dev/null || true
    fi
    [ -f "$tmpdir/$asset_name" ] && break
    echo "  Retry ${attempt}/3..." >&2
  done
  if [ ! -f "$tmpdir/$asset_name" ]; then
    echo "Error: failed to download plugin-store after 3 attempts" >&2
    exit 1
  fi
  verify_checksum "$tmpdir/$asset_name" "$asset_name" "$ps_checksums"
  mv "$tmpdir/$asset_name" "$INSTALL_DIR/plugin-store"
  chmod 755 "$INSTALL_DIR/plugin-store"
  echo "  Installed: ${INSTALL_DIR}/plugin-store"

  echo ""

  # ── 2. Install DApp binaries from registry ───────────────
  install_dapp_binaries "$target" "$tmpdir" "$FILTER"

  echo ""
  ensure_in_path

  echo ""
  echo "Done!"
}

main "$@"
