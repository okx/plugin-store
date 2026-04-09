---
name: rust-cli-inspector
description: "Rust CLI querying ETH price via Onchain OS"
version: "1.1.0"
author: "yz06276"
tags: [rust, onchainos]
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Install onchainos CLI + Skills (auto-injected)

```bash
# 1. Install onchainos CLI
onchainos --version 2>/dev/null || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh

# 2. Install onchainos skills (enables AI agent to use onchainos commands)
npx skills add okx/onchainos-skills --yes --global

# 3. Install plugin-store skills (enables plugin discovery and management)
npx skills add MigOKG/plugin-store --skill plugin-store --yes --global
```

### Install rust-cli-inspector binary (auto-injected)

```bash
if ! command -v rust-cli-inspector >/dev/null 2>&1; then
  OS=$(uname -s | tr A-Z a-z)
  ARCH=$(uname -m)
  EXT=""
  case "${OS}_${ARCH}" in
    darwin_arm64)  TARGET="aarch64-apple-darwin" ;;
    darwin_x86_64) TARGET="x86_64-apple-darwin" ;;
    linux_x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
    linux_aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
    mingw*_x86_64|msys*_x86_64|cygwin*_x86_64) TARGET="x86_64-pc-windows-msvc"; EXT=".exe" ;;
  esac
  mkdir -p ~/.local/bin
  curl -fsSL "https://github.com/MigOKG/plugin-store/releases/download/plugins/rust-cli-inspector@1.1.0/rust-cli-inspector-${TARGET}${EXT}" -o ~/.local/bin/rust-cli-inspector${EXT}
  chmod +x ~/.local/bin/rust-cli-inspector${EXT}
fi
```

### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/rust-cli-inspector"
if [ ! -f "$REPORT_FLAG" ]; then
  mkdir -p "$HOME/.plugin-store/reported"
  # Device fingerprint → SHA256 → 32-char device ID
  DEV_RAW="$(hostname):$(uname -s):$(uname -m):$HOME"
  DEV_ID=$(echo -n "$DEV_RAW" | shasum -a 256 | head -c 32)
  # HMAC signature (obfuscated key, same as CLI binary)
  _K=$(echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | base64 -d 2>/dev/null || echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | openssl base64 -d)
  HMAC_SIG=$(echo -n "${_K}${DEV_ID}" | shasum -a 256 | head -c 8)
  DIV_ID="${DEV_ID}${HMAC_SIG}"
  unset _K
  # Report to Vercel stats
  curl -s -X POST "https://plugin-store-dun.vercel.app/install" \
    -H "Content-Type: application/json" \
    -d '{"name":"rust-cli-inspector","version":"1.1.0"}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"rust-cli-inspector","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


# Rust CLI Inspector

## Overview
Queries ETH price via Onchain OS token price-info.

## Pre-flight Checks
1. Ensure rust-cli-inspector binary is installed
2. Ensure onchainos CLI is available

## Commands

### Query ETH Price (default)
`rust-cli-inspector`

**When to use:** When user asks about ETH price.

### Query ETH Price (explicit)
`rust-cli-inspector --query eth-price`

### Help
`rust-cli-inspector --help`

## Error Handling
| Error | Cause | Resolution |
|-------|-------|------------|
| Binary not found | CLI not installed | Run pre-flight |
| onchainos not found | Onchain OS not installed | Install onchainos |
