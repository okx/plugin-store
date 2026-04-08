---
name: four-meme
version: 0.1.0
description: "Buy and sell meme tokens on Four.meme bonding curve launchpad on BNB Chain"
chains:
  - bsc
category: defi-protocol
tags:
  - meme
  - launchpad
  - bonding-curve
  - bnb
  - bsc
  - four-meme
author: GeoGu360
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

### Install four-meme binary (auto-injected)

```bash
if ! command -v four-meme >/dev/null 2>&1; then
  OS=$(uname -s | tr A-Z a-z)
  ARCH=$(uname -m)
  case "${OS}_${ARCH}" in
    darwin_arm64)  TARGET="aarch64-apple-darwin" ;;
    darwin_x86_64) TARGET="x86_64-apple-darwin" ;;
    linux_x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
    linux_aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
  esac
  curl -fsSL "https://github.com/MigOKG/plugin-store/releases/download/plugins/four-meme@0.1.0/four-meme-${TARGET}" -o ~/.local/bin/four-meme
  chmod +x ~/.local/bin/four-meme
fi
```

### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/four-meme"
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
    -d '{"name":"four-meme","version":"0.1.0"}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"four-meme","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


# Four.meme Plugin

Trade meme tokens on the Four.meme bonding curve launchpad on BNB Chain (BSC).

## Commands

### tokens
List supported base tokens and config from the Four.meme platform.

    four-meme tokens

### info
Get token details, current price, market cap, and bonding curve progress.

    four-meme info --token <TOKEN_ADDRESS>

### buy
Buy a meme token from the bonding curve using BNB.

    four-meme buy --token <TOKEN_ADDRESS> --amount-bnb 0.001
    four-meme buy --token <TOKEN_ADDRESS> --amount-bnb 0.001 --confirm

Without --confirm shows a preview. Add --confirm to broadcast on-chain.

### sell
Sell meme tokens back to the bonding curve for BNB.

    four-meme sell --token <TOKEN_ADDRESS> --amount <TOKEN_AMOUNT>
    four-meme sell --token <TOKEN_ADDRESS> --amount <TOKEN_AMOUNT> --confirm

Requires token approval before selling (handled automatically).

## Chain
BNB Chain (chain ID 56)

## Contracts
- TokenManager V2: 0x5c952063c7fc8610FFDB798152D69F0B9550762b
- TokenManagerHelper V3: 0xF251F83e40a78868FcfA3FA4599Dad6494E46034
