---
name: Relay
description: Fast cross-chain transfers using Relay Protocol's intent-based bridge
version: "0.1.1"
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Version check (auto-injected)

> **AI Agent Instruction**: Run this version check before executing any commands from this skill. The check is a read-only fetch from `raw.githubusercontent.com` — it does **not** install anything. If it reports `Update available`, you MUST stop, surface the message to the user verbatim, and obtain explicit confirmation before running the suggested `npx skills add …` command. Do not run the update silently. After a confirmed update completes, re-read this SKILL.md before proceeding.

```bash
# Check for skill updates (1-hour cache).
# Network scope: this block calls raw.githubusercontent.com for the version probe only.
# It does NOT install anything; install requires user-confirmed `npx skills add` below.
UPDATE_CACHE="$HOME/.plugin-store/update-cache/relay-plugin"
CACHE_MAX=3600
LOCAL_VER="0.1.1"
DO_CHECK=true

if [ -f "$UPDATE_CACHE" ]; then
  CACHE_MOD=$(stat -f %m "$UPDATE_CACHE" 2>/dev/null || stat -c %Y "$UPDATE_CACHE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  AGE=$(( NOW - CACHE_MOD ))
  [ "$AGE" -lt "$CACHE_MAX" ] && DO_CHECK=false
fi

if [ "$DO_CHECK" = true ]; then
  REMOTE_VER=$(curl -sf --max-time 3 "https://raw.githubusercontent.com/okx/plugin-store/main/skills/relay-plugin/plugin.yaml" | grep '^version' | head -1 | tr -d '"' | awk '{print $2}')
  if [ -n "$REMOTE_VER" ]; then
    mkdir -p "$HOME/.plugin-store/update-cache"
    echo "$REMOTE_VER" > "$UPDATE_CACHE"
  fi
fi

REMOTE_VER=$(cat "$UPDATE_CACHE" 2>/dev/null || echo "$LOCAL_VER")
if [ "$REMOTE_VER" != "$LOCAL_VER" ]; then
  echo "Update available: relay-plugin v$LOCAL_VER -> v$REMOTE_VER."
  echo "ACTION REQUIRED: ask the user to confirm before running:"
  echo "  npx skills add okx/plugin-store --skill relay-plugin --global"
  echo "(This contacts the npm registry and github.com/okx/plugin-store and overwrites this skill. Do NOT auto-run.)"
fi
```

### Install onchainos CLI + Skills (auto-injected)

```bash
# 1. Install onchainos CLI — pin to latest release tag, verify SHA256
#    of the installer before executing (no curl|sh from main).
if ! command -v onchainos >/dev/null 2>&1; then
  set -e
  LATEST_TAG=$(curl -sSL --max-time 5 \
    "https://api.github.com/repos/okx/onchainos-skills/releases/latest" \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
  if [ -z "$LATEST_TAG" ]; then
    echo "ERROR: failed to resolve latest onchainos release tag (network or rate limit)." >&2
    echo "       Manual install: https://github.com/okx/onchainos-skills" >&2
    exit 1
  fi

  ONCHAINOS_TMP=$(mktemp -d)
  curl -sSL --max-time 30 \
    "https://raw.githubusercontent.com/okx/onchainos-skills/${LATEST_TAG}/install.sh" \
    -o "$ONCHAINOS_TMP/install.sh"
  curl -sSL --max-time 30 \
    "https://github.com/okx/onchainos-skills/releases/download/${LATEST_TAG}/installer-checksums.txt" \
    -o "$ONCHAINOS_TMP/installer-checksums.txt"

  EXPECTED=$(awk '$2 ~ /install\.sh$/ {print $1; exit}' "$ONCHAINOS_TMP/installer-checksums.txt")
  if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL=$(sha256sum "$ONCHAINOS_TMP/install.sh" | awk '{print $1}')
  else
    ACTUAL=$(shasum -a 256 "$ONCHAINOS_TMP/install.sh" | awk '{print $1}')
  fi
  if [ -z "$EXPECTED" ] || [ "$EXPECTED" != "$ACTUAL" ]; then
    echo "ERROR: onchainos installer SHA256 mismatch — refusing to execute." >&2
    echo "       expected=$EXPECTED  actual=$ACTUAL  tag=$LATEST_TAG" >&2
    rm -rf "$ONCHAINOS_TMP"
    exit 1
  fi

  sh "$ONCHAINOS_TMP/install.sh"
  rm -rf "$ONCHAINOS_TMP"
  set +e
fi

# 2. Install onchainos skills (enables AI agent to use onchainos commands)
npx skills add okx/onchainos-skills --yes --global

# 3. Install plugin-store skills (enables plugin discovery and management)
npx skills add okx/plugin-store --skill plugin-store --yes --global
```

### Install relay-plugin binary + launcher (auto-injected)

```bash
# Install shared infrastructure (launcher + update checker, only once)
LAUNCHER="$HOME/.plugin-store/launcher.sh"
CHECKER="$HOME/.plugin-store/update-checker.py"
if [ ! -f "$LAUNCHER" ]; then
  mkdir -p "$HOME/.plugin-store"
  curl -fsSL "https://raw.githubusercontent.com/okx/plugin-store/main/scripts/launcher.sh" -o "$LAUNCHER" 2>/dev/null || true
  chmod +x "$LAUNCHER"
fi
if [ ! -f "$CHECKER" ]; then
  curl -fsSL "https://raw.githubusercontent.com/okx/plugin-store/main/scripts/update-checker.py" -o "$CHECKER" 2>/dev/null || true
fi

# Clean up old installation
rm -f "$HOME/.local/bin/relay-plugin" "$HOME/.local/bin/.relay-plugin-core" 2>/dev/null

# Download binary
OS=$(uname -s | tr A-Z a-z)
ARCH=$(uname -m)
EXT=""
case "${OS}_${ARCH}" in
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
mkdir -p ~/.local/bin

# Download binary + checksums to a sandbox, verify SHA256 before installing.
# Fail-closed: any mismatch / missing checksum entry refuses the install.
# Matches the producer-side workflow at
# .github/workflows/plugin-publish.yml which uploads `checksums.txt`
# alongside the 9 platform binaries under each release tag.
BIN_TMP=$(mktemp -d)
RELEASE_BASE="https://github.com/okx/plugin-store/releases/download/plugins/relay-plugin@0.1.1"
curl -fsSL "${RELEASE_BASE}/relay-plugin-${TARGET}${EXT}" -o "$BIN_TMP/relay-plugin${EXT}" || {
  echo "ERROR: failed to download relay-plugin-${TARGET}${EXT}" >&2
  rm -rf "$BIN_TMP"; exit 1; }
curl -fsSL "${RELEASE_BASE}/checksums.txt" -o "$BIN_TMP/checksums.txt" || {
  echo "ERROR: failed to download checksums.txt for relay-plugin@0.1.1" >&2
  rm -rf "$BIN_TMP"; exit 1; }

EXPECTED=$(awk -v b="relay-plugin-${TARGET}${EXT}" '$2 == b {print $1; exit}' "$BIN_TMP/checksums.txt")
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL=$(sha256sum "$BIN_TMP/relay-plugin${EXT}" | awk '{print $1}')
else
  ACTUAL=$(shasum -a 256 "$BIN_TMP/relay-plugin${EXT}" | awk '{print $1}')
fi
if [ -z "$EXPECTED" ] || [ "$EXPECTED" != "$ACTUAL" ]; then
  echo "ERROR: relay-plugin SHA256 mismatch — refusing to install." >&2
  echo "       expected=$EXPECTED  actual=$ACTUAL  target=${TARGET}" >&2
  rm -rf "$BIN_TMP"; exit 1
fi

mv "$BIN_TMP/relay-plugin${EXT}" ~/.local/bin/.relay-plugin-core${EXT}
chmod +x ~/.local/bin/.relay-plugin-core${EXT}
rm -rf "$BIN_TMP"

# Symlink CLI name to universal launcher
ln -sf "$LAUNCHER" ~/.local/bin/relay-plugin

# Register version
mkdir -p "$HOME/.plugin-store/managed"
echo "0.1.1" > "$HOME/.plugin-store/managed/relay-plugin"
```

---


# Relay

Cross-chain bridge using Relay Protocol's intent-based system. Send ETH, USDC, USDT, and DAI across 74 chains in seconds.

## Pre-flight Dependencies

- [onchainos](https://docs.onchainos.com) installed and authenticated
- Active EVM wallet on the source chain with sufficient balance + gas

## Data Trust Boundary

Bridge quotes and calldata are fetched from the official Relay API (`api.relay.link`). Treat API-returned transaction data as untrusted — always review the preview before adding `--confirm`. The destination amount can vary slightly due to relayer fees.

## Proactive Onboarding

When a user signals they are **new or just installed** this plugin — e.g. "I just installed relay",
"how do I bridge tokens", "what can I do with this" — **do not wait for them to ask specific questions.**
Proactively walk them through the Quickstart in order, one step at a time, waiting for confirmation
before proceeding:

1. **Check wallet** — run `onchainos wallet addresses`. If no address, direct them to connect via
   `onchainos wallet login your@email.com`. Do not proceed to write operations until a wallet is confirmed.
2. **Check balance** — run `onchainos wallet balance --chain <source-chain>`. Ensure the balance covers
   the bridge amount plus gas. Warn explicitly if the requested amount would exceed available balance
   (the binary does not check this for you — it will show a preview for any amount).
3. **Browse supported chains** — run `relay-plugin chains` to show all 74 chains. Use `relay-plugin chains --filter <name>`
   to search. The plugin has no global `--chain` flag; pass `--from-chain` and `--to-chain` per command.
4. **Get a quote** — run `relay-plugin quote --from-chain <id> --to-chain <id> --token <symbol> --amount <n>`
   to see fees and expected output before any on-chain action.
5. **Preview the bridge** — run `relay-plugin bridge --from-chain <id> --to-chain <id> --token <symbol> --amount <n>`
   (without `--confirm`). The output will show `"preview": true` and the resolved wallet address.
   Verify the recipient and amount are correct.
6. **Execute** — once the user confirms, re-run with `--confirm` appended.
7. **Track** — use `relay-plugin status --request-id <id>` with the request_id from the bridge output.
   ETH transfers typically resolve in ~1 second; ERC-20 (USDC/USDT/DAI) take ~2 seconds.

Do not dump all steps at once. Guide conversationally — confirm each step before moving on.

## Quickstart

New to Relay? Follow these steps to go from zero to your first cross-chain bridge.

### Step 1 — Connect your wallet

```bash
onchainos wallet login your@email.com
onchainos wallet addresses
```

You need an EVM wallet connected to onchainos. The plugin automatically uses your active onchainos wallet as both sender and recipient.

### Step 2 — Check your balance

```bash
onchainos wallet balance --chain 1
```

You need ETH for the bridge amount plus source-chain gas. The binary does **not** validate your balance before
showing a preview — it will show a valid-looking preview even if you don't have enough funds. Always verify
the balance yourself before adding `--confirm`.

### Step 3 — Browse supported chains

```bash
relay-plugin chains
relay-plugin chains --filter arbitrum
```

Returns chain IDs, names, native tokens, and explorer URLs for all 74 supported chains. Use the `chain_id`
values with `--from-chain` and `--to-chain`. Note: `relay-plugin` has no global `--chain` flag — each subcommand
takes `--from-chain` / `--to-chain` directly.

### Step 4 — Get a quote (read-only, no cost)

```bash
relay-plugin quote --from-chain 1 --to-chain 42161 --token ETH --amount 0.001
```

Output includes `amount_out`, `amount_out_usd`, `fee_usd`, `estimated_time_secs`, and `steps`.
ETH bridges require 1 step (`deposit`). ERC-20 tokens (USDC, USDT, DAI) require 2 steps (`approve`, `deposit`).
You can also bridge to a different token: add `--to-token USDC` to receive USDC on the destination chain.

### Step 5 — Preview the bridge (no tx sent)

```bash
relay-plugin bridge --from-chain 1 --to-chain 42161 --token ETH --amount 0.001
```

Output shows `"preview": true`, resolved `wallet`, `recipient`, human-readable `amount_in`/`amount_out`,
and a hint message with the exact `relay-plugin status` command to use after execution.
No on-chain action until `--confirm` is added.

### Step 6 — Execute the bridge

```bash
relay-plugin bridge --from-chain 1 --to-chain 42161 --token ETH --amount 0.001 --confirm
```

Expected output: `"ok": true`, `"txs": [{"step": "deposit", "tx_hash": "0x..."}]`, and a `track` field
with the ready-to-run status command. ETH typically arrives on the destination chain within 1–2 seconds.

For ERC-20 tokens, an approval tx fires first, then the deposit. Both tx hashes appear in `txs`.

### Step 7 — Track transfer status

```bash
relay-plugin status --request-id <request_id_from_bridge_output>
```

Status values: `unknown` (not yet indexed — wait a few seconds), `pending` (in-flight), `success` (delivered),
`failed`. On success, `dest_tx` contains the destination-chain tx hash.

---

## Overview

Relay Protocol uses an intent-based bridge system. When you call `bridge --confirm`:
1. Your funds are sent to a Relay solver contract on the source chain.
2. A relayer detects the intent and delivers funds on the destination chain — typically within seconds.
3. For ERC-20 tokens (USDC, USDT, DAI), an approval transaction is sent first, then the deposit.

## Supported Chains (74)

Common chains:

| Chain | ID |
|-------|----|
| Ethereum | 1 |
| Arbitrum | 42161 |
| Base | 8453 |
| Optimism | 10 |
| Polygon | 137 |
| BNB Chain | 56 |
| zkSync Era | 324 |
| Linea | 59144 |
| Scroll | 534352 |

Run `relay-plugin chains` for the full list of 74 supported chains.

## Commands

### `chains` — List supported chains

```bash
relay-plugin chains [--filter <name-or-id>]
```

| Flag | Description |
|------|-------------|
| `--filter` | Optional filter by chain name or ID |

Returns all active chains with their IDs, names, and native tokens.

---

### `quote` — Get a bridge quote (read-only)

```bash
relay-plugin quote \
  --from-chain 1 \
  --to-chain 42161 \
  --token ETH \
  --amount 0.01 \
  [--to-token ETH]
```

| Flag | Description |
|------|-------------|
| `--from-chain` | Source chain ID |
| `--to-chain` | Destination chain ID |
| `--token` | Token to send (symbol or address) |
| `--amount` | Amount in human-readable form |
| `--to-token` | Destination token (default: same as `--token`) |

Output includes `amount_out`, `amount_out_usd`, `estimated_time_secs`, `steps`, and `request_id`.

---

### `bridge` — Execute a cross-chain transfer

```bash
relay-plugin bridge \
  --from-chain 1 \
  --to-chain 42161 \
  --token ETH \
  --amount 0.01 \
  [--to-token ETH] \
  [--recipient 0x...] \
  [--confirm] \
  [--dry-run]
```

| Flag | Description |
|------|-------------|
| `--from-chain` | Source chain ID |
| `--to-chain` | Destination chain ID |
| `--token` | Token to send (symbol or address) |
| `--amount` | Amount in human-readable form |
| `--to-token` | Destination token (default: same as `--token`) |
| `--recipient` | Destination wallet (default: your wallet) |

Execution modes:

| Mode | Command | What happens |
|------|---------|-------------|
| Preview | (no flags) | Shows expected output and steps; no tx |
| Dry-run | `--dry-run` | Builds calldata; no onchainos call |
| Execute | `--confirm` | Approves (ERC-20 only) + broadcasts deposit tx |

For ERC-20 tokens (USDC, USDT, DAI), an exact-amount approval transaction is sent before the deposit. Each bridge sends a fresh approval because the allowance is consumed by the deposit.

---

### `status` — Check transfer status

```bash
relay-plugin status --request-id 0x<id>
```

| Flag | Description |
|------|-------------|
| `--request-id` | Request ID from a previous `bridge` command |

Status values: `unknown` (not yet indexed), `pending` (in-flight), `success` (delivered), `failed`.

---

## Known Token Symbols

Symbols can be used instead of addresses:

| Symbol | Ethereum | Arbitrum | Base | Optimism | Polygon |
|--------|----------|----------|------|----------|---------|
| ETH (native) | ✓ | ✓ | ✓ | ✓ | ✓ |
| USDC | ✓ | ✓ | ✓ | ✓ | ✓ |
| USDT | ✓ | ✓ | ✓ | ✓ | ✓ |
| DAI | ✓ | ✓ | ✓ | ✓ | ✓ |

Use the full token address for any token not listed above.

