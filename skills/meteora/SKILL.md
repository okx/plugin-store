---
name: meteora
description: "Meteora DLMM plugin for searching pools, getting swap quotes, checking positions, and executing swaps on Solana"
version: "0.1.0"
tags:
  - dex
  - solana
  - dlmm
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
npx skills add okx/plugin-store --skill plugin-store --yes --global
```

### Install meteora binary (auto-injected)

```bash
if ! command -v meteora >/dev/null 2>&1; then
  OS=$(uname -s | tr A-Z a-z)
  ARCH=$(uname -m)
  EXT=""
  case "${OS}_${ARCH}" in
    darwin_arm64)  TARGET="aarch64-apple-darwin" ;;
    darwin_x86_64) TARGET="x86_64-apple-darwin" ;;
    linux_x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
    linux_i686)    TARGET="i686-unknown-linux-gnu" ;;
    linux_aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
    linux_armv7l)  TARGET="armv7-unknown-linux-gnueabihf" ;;
    mingw*_x86_64|msys*_x86_64|cygwin*_x86_64)   TARGET="x86_64-pc-windows-msvc"; EXT=".exe" ;;
    mingw*_i686|msys*_i686|cygwin*_i686)           TARGET="i686-pc-windows-msvc"; EXT=".exe" ;;
    mingw*_aarch64|msys*_aarch64|cygwin*_aarch64)  TARGET="aarch64-pc-windows-msvc"; EXT=".exe" ;;
  esac
  mkdir -p ~/.local/bin
  curl -fsSL "https://github.com/okx/plugin-store/releases/download/plugins/meteora@0.1.0/meteora-${TARGET}${EXT}" -o ~/.local/bin/meteora${EXT}
  chmod +x ~/.local/bin/meteora${EXT}
fi
```

### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/meteora"
if [ ! -f "$REPORT_FLAG" ]; then
  mkdir -p "$HOME/.plugin-store/reported"
  DEV_RAW="$(hostname):$(uname -s):$(uname -m):$HOME"
  DEV_ID=$(echo -n "$DEV_RAW" | shasum -a 256 | head -c 32)
  _K=$(echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | base64 -d 2>/dev/null || echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | openssl base64 -d)
  HMAC_SIG=$(echo -n "${_K}${DEV_ID}" | shasum -a 256 | head -c 8)
  DIV_ID="${DEV_ID}${HMAC_SIG}"
  unset _K
  curl -s -X POST "https://plugin-store-dun.vercel.app/install" \
    -H "Content-Type: application/json" \
    -d '{"name":"meteora","version":"0.1.0"}' >/dev/null 2>&1 || true
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"meteora","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---




# Meteora DLMM Plugin

Search pools, get swap quotes, check positions, and execute swaps on Solana via Meteora DLMM.
