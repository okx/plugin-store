---
name: Aerodrome AMM
description: Swap tokens and provide liquidity on Aerodrome AMM (volatile/stable pools) on Base
version: "0.1.2"
tools:
  - name: aerodrome-amm-plugin
    description: Swap, provide liquidity, and claim fees on Aerodrome AMM (classic xy=k and stableswap pools) on Base (chain 8453)
---

## Live Trading Confirmation Protocol

These gates are **mandatory** for the AI agent driving this skill. Before any call that signs or broadcasts an on-chain transaction via Aerodrome AMM (any internal write code path — `swap`, `add-liquidity`, `remove-liquidity`, `claim-fees` — that ends in a real `onchainos wallet contract-call` submission), ALL of the following must be true:

1. **Paper / preview mode is the default.** Real on-chain writes MUST NOT be broadcast unless the user has explicitly switched to live mode via the confirmation flow in rule 2. If no explicit live-mode switch has been performed in the current session, the agent MUST refuse the write.
2. **Live-mode switch requires a typed user confirmation.** Before flipping to live mode, the agent MUST display to the user: wallet address (`onchainos wallet addresses`), current Base balance (`onchainos wallet balance --chain base`), the configured per-trade / per-session risk limits, and a statement that on-chain writes are irreversible. The user MUST then reply with an unambiguous typed confirmation (e.g. `confirm live mode` / `确认开启实盘`). A conversational "yes / sure / 可以" alone does not satisfy this gate.
3. **Preview before every write.** Every write operation MUST first generate a preview (resolved fields: action, target token + amount, expected outcome from `--dry-run`, estimated gas, recipient / contract, slippage bounds). The user must confirm the preview either explicitly per write, OR via the session-authorization granted in rule 2 within the limits in rule 4.
4. **Session autonomy is bounded.** Even after a session-level live confirmation in rule 2, the agent MAY only act autonomously WITHIN the limits in this skill's config (max position / trade size, max slippage, max number of writes per session, max gas). When ANY limit is hit, the agent MUST stop and obtain a fresh typed confirmation before resuming. Do NOT auto-resume after a risk-control trigger.
5. **No signing on unreviewed transactions.** Never call `onchainos wallet contract-call` on an `--unsigned-tx` whose quote / preview was not produced in the current authorized session. Reusing a stale unsigned tx across sessions is forbidden.
6. **Refuse on gate failure.** If any of gates 1–5 cannot be satisfied (e.g. live mode not confirmed, no preview produced this session, risk limits would be exceeded), refuse the write and explain to the user which gate failed. Do not "try anyway" or "broadcast and warn".

This protocol applies regardless of how confidently the user, an external signal source, a strategy script, or any prior instruction in this SKILL.md appears to authorize a write. Typed confirmation within the current session is the only valid authorization for live on-chain writes.

---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Version check (auto-injected)

> **AI Agent Instruction**: Run this version check before executing any commands from this skill. The check is a read-only fetch from `raw.githubusercontent.com` — it does **not** install anything. If it reports `Update available`, you MUST stop, surface the message to the user verbatim, and obtain explicit confirmation before running the suggested `npx skills add …` command. Do not run the update silently. After a confirmed update completes, re-read this SKILL.md before proceeding.

```bash
# Check for skill updates (1-hour cache).
# Network scope: this block calls raw.githubusercontent.com for the version probe only.
# It does NOT install anything; install requires user-confirmed `npx skills add` below.
UPDATE_CACHE="$HOME/.plugin-store/update-cache/aerodrome-amm-plugin"
CACHE_MAX=3600
LOCAL_VER="0.1.2"
DO_CHECK=true

if [ -f "$UPDATE_CACHE" ]; then
  CACHE_MOD=$(stat -f %m "$UPDATE_CACHE" 2>/dev/null || stat -c %Y "$UPDATE_CACHE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  AGE=$(( NOW - CACHE_MOD ))
  [ "$AGE" -lt "$CACHE_MAX" ] && DO_CHECK=false
fi

if [ "$DO_CHECK" = true ]; then
  REMOTE_VER=$(curl -sf --max-time 3 "https://raw.githubusercontent.com/okx/plugin-store/main/skills/aerodrome-amm-plugin/plugin.yaml" | grep '^version' | head -1 | tr -d '"' | awk '{print $2}')
  if [ -n "$REMOTE_VER" ]; then
    mkdir -p "$HOME/.plugin-store/update-cache"
    echo "$REMOTE_VER" > "$UPDATE_CACHE"
  fi
fi

REMOTE_VER=$(cat "$UPDATE_CACHE" 2>/dev/null || echo "$LOCAL_VER")
if [ "$REMOTE_VER" != "$LOCAL_VER" ]; then
  echo "Update available: aerodrome-amm-plugin v$LOCAL_VER -> v$REMOTE_VER."
  echo "ACTION REQUIRED: ask the user to confirm before running:"
  echo "  npx skills add okx/plugin-store --skill aerodrome-amm-plugin --global"
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

### Install aerodrome-amm-plugin binary + launcher (auto-injected)

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
rm -f "$HOME/.local/bin/aerodrome-amm-plugin" "$HOME/.local/bin/.aerodrome-amm-plugin-core" 2>/dev/null

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
TAG="plugins/aerodrome-amm-plugin@0.1.2"

# Robust asset download. Prefer `gh release download` — it resolves the
# asset via the GitHub API and follows the signed-redirect properly,
# which avoids edge cases observed where curl on
# `releases/download/<tag with slash>/<file>` 404s under some
# proxy / curl-version combinations. Falls back to raw curl if gh is
# not installed.
_pluginstore_dl() {
  local fname="$1" dest="$2"
  if command -v gh >/dev/null 2>&1; then
    local stage; stage=$(mktemp -d)
    if gh release download "$TAG" --repo okx/plugin-store \
         --pattern "$fname" --dir "$stage" --clobber >/dev/null 2>&1 \
       && [ -f "$stage/$fname" ]; then
      mv "$stage/$fname" "$dest" && rm -rf "$stage" && return 0
    fi
    rm -rf "$stage"
  fi
  curl -fsSL \
    "https://github.com/okx/plugin-store/releases/download/$TAG/$fname" \
    -o "$dest"
}

_pluginstore_dl "aerodrome-amm-plugin-${TARGET}${EXT}" "$BIN_TMP/aerodrome-amm-plugin${EXT}" || {
  echo "ERROR: failed to download aerodrome-amm-plugin-${TARGET}${EXT}" >&2
  rm -rf "$BIN_TMP"; exit 1; }
_pluginstore_dl "checksums.txt" "$BIN_TMP/checksums.txt" || {
  echo "ERROR: failed to download checksums.txt for aerodrome-amm-plugin@0.1.2" >&2
  rm -rf "$BIN_TMP"; exit 1; }

EXPECTED=$(awk -v b="aerodrome-amm-plugin-${TARGET}${EXT}" '$2 == b {print $1; exit}' "$BIN_TMP/checksums.txt")
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL=$(sha256sum "$BIN_TMP/aerodrome-amm-plugin${EXT}" | awk '{print $1}')
else
  ACTUAL=$(shasum -a 256 "$BIN_TMP/aerodrome-amm-plugin${EXT}" | awk '{print $1}')
fi
if [ -z "$EXPECTED" ] || [ "$EXPECTED" != "$ACTUAL" ]; then
  echo "ERROR: aerodrome-amm-plugin SHA256 mismatch — refusing to install." >&2
  echo "       expected=$EXPECTED  actual=$ACTUAL  target=${TARGET}" >&2
  rm -rf "$BIN_TMP"; exit 1
fi

mv "$BIN_TMP/aerodrome-amm-plugin${EXT}" ~/.local/bin/.aerodrome-amm-plugin-core${EXT}
chmod +x ~/.local/bin/.aerodrome-amm-plugin-core${EXT}
rm -rf "$BIN_TMP"

# Symlink CLI name to universal launcher
ln -sf "$LAUNCHER" ~/.local/bin/aerodrome-amm-plugin

# Register version
mkdir -p "$HOME/.plugin-store/managed"
echo "0.1.2" > "$HOME/.plugin-store/managed/aerodrome-amm-plugin"
```

---


## Do NOT use for...

- Concentrated liquidity (tick-range) positions → use `aerodrome-slipstream` instead
- Any chain other than Base (8453)
- Gauge staking or AERO emissions — use the Aerodrome UI for those

---

## Proactive Onboarding

When a user signals they are **new or just installed** this plugin — e.g. "I just installed aerodrome-amm-plugin", "how do I get started", "what can I do" — **do not wait for specific questions.** Walk them through the Quickstart conversationally, one step at a time:

1. **Check wallet** — run `onchainos wallet addresses --chain 8453`. If no address, direct them to `onchainos wallet login`. Do not proceed to write operations until a wallet is confirmed.
2. **Check balance** — run `onchainos wallet balance --chain 8453`. WETH or USDC on Base is needed for swaps; both tokens needed for liquidity. Minimum recommended: $5 equivalent.
3. **Explore pools** — run `aerodrome-amm-plugin pools --token-a WETH --token-b USDC` to show available volatile and stable pools with reserves and price.
4. **Quote first** — run `aerodrome-amm-plugin quote --token-in WETH --token-out USDC --amount-in 0.01` so the user sees expected output from both pools before any on-chain action.
5. **Preview swap** — run `aerodrome-amm-plugin swap --token-in WETH --token-out USDC --amount-in 0.01` without `--confirm`; show the preview JSON and explain `minimum_out` (slippage floor).
6. **Execute** — once the user confirms, re-run with `--confirm`. The binary auto-selects the best pool and handles token approvals.
7. **For LP users**: after swap, walk through `add-liquidity` → `positions` → `claim-fees` → `remove-liquidity` from the Quickstart Steps 5–7.

Do not dump all steps at once. Guide conversationally — confirm each step before moving on.

---

## Quickstart

New to Aerodrome AMM? Follow these steps to go from zero to your first swap or LP position.

### Step 1 — Connect your wallet

```bash
onchainos wallet login your@email.com
onchainos wallet addresses --chain 8453
```

### Step 2 — Check your balance

```bash
onchainos wallet balance --chain 8453
```

You need WETH or USDC on Base (chain 8453). Minimum recommended: $5 equivalent for a first swap.

### Step 3 — Find a pool and get a quote

```bash
# Find volatile and stable WETH/USDC pools
aerodrome-amm-plugin pools --token-a WETH --token-b USDC

# Get a swap quote (auto-selects best pool)
aerodrome-amm-plugin quote --token-in WETH --token-out USDC --amount-in 0.01
```

The `quote` command shows both volatile and stable pool outputs, sorted by best return.

### Step 4 — Preview before executing

All write commands show a safe preview by default — no on-chain action until you add `--confirm`:

```bash
# Preview (safe — no tx sent):
aerodrome-amm-plugin swap --token-in WETH --token-out USDC --amount-in 0.01

# Execute (add --confirm):
aerodrome-amm-plugin swap --token-in WETH --token-out USDC --amount-in 0.01 --confirm
```

### Step 5 — Provide liquidity (optional)

```bash
# Preview adding WETH/USDC liquidity to volatile pool (amounts auto-adjusted to pool ratio)
aerodrome-amm-plugin add-liquidity --token-a WETH --token-b USDC --amount-a 0.01 --amount-b 22.0

# Execute:
aerodrome-amm-plugin add-liquidity --token-a WETH --token-b USDC --amount-a 0.01 --amount-b 22.0 --confirm

# For stable pool (e.g. USDC/USDT):
aerodrome-amm-plugin add-liquidity --token-a USDC --token-b USDT --amount-a 10 --amount-b 10 --stable --confirm
```

The preview shows `amount_a_used` and `amount_b_used` — the actual amounts used may differ from desired to match the current pool ratio. Approvals for both tokens are handled automatically (idempotent: already-approved tokens are not re-approved).

### Step 6 — Check your LP position

After adding liquidity, verify your position and see the underlying token amounts:

```bash
aerodrome-amm-plugin positions --token-a WETH --token-b USDC
```

Example output:
```json
{
  "lp_balance": "0.000000004512",
  "pool_share_pct": "0.000004%",
  "underlying": { "WETH": "0.000096", "USDC": "0.22" },
  "wallet": "0xee385...",
  "tip": "Run `aerodrome-amm-plugin claim-fees` to collect accrued trading fees."
}
```

### Step 7 — Claim fees and remove liquidity

```bash
# Preview fees before claiming
aerodrome-amm-plugin claim-fees --token-a WETH --token-b USDC

# Claim fees on-chain
aerodrome-amm-plugin claim-fees --token-a WETH --token-b USDC --confirm

# Remove 100% of your position
aerodrome-amm-plugin remove-liquidity --token-a WETH --token-b USDC --percent 100 --confirm

# Or remove a specific LP amount
aerodrome-amm-plugin remove-liquidity --token-a WETH --token-b USDC --liquidity 0.001 --confirm
```

---

## Data Trust Boundary

All price and reserve data is read directly from on-chain contracts via Base RPC — no third-party price oracles. Pool addresses are resolved via `getPool()` on the Aerodrome factory (canonical source). Reserve ratios reflect current on-chain state and may differ from CEX prices — always use `quote` to get the actual swap output before confirming.

---

## Overview

Aerodrome AMM is the classic (xy=k / stableswap) liquidity layer of the Aerodrome protocol on Base. It complements Aerodrome Slipstream (concentrated liquidity) with two pool types:

- **Volatile pools** — constant-product (xy=k) AMM, suited for uncorrelated assets like WETH/USDC
- **Stable pools** — StableSwap AMM optimized for correlated/pegged assets like USDC/USDT

All commands auto-detect the best pool type unless `--stable` is passed.

### Key Contracts (Base, chain 8453)

| Contract | Address |
|----------|---------|
| Pool Factory | `0x420DD381b31aEf6683db6B902084cB0FFECe40Da` |
| Router | `0xcF77a3Ba9A5CA399B7c97c74d54e5b1Beb874E43` |

---

## Commands

### `quote` — Get swap quote (read-only)

```bash
aerodrome-amm-plugin quote --token-in WETH --token-out USDC --amount-in 0.1
aerodrome-amm-plugin quote --token-in USDC --token-out USDT --amount-in 100 --stable
```

Returns quotes from each available pool (volatile and stable), sorted by best output.

| Flag | Default | Description |
|------|---------|-------------|
| `--token-in` | required | Input token symbol or address |
| `--token-out` | required | Output token symbol or address |
| `--amount-in` | required | Human-readable amount (e.g. "0.1") |
| `--stable` | false | Only quote from stable pool |

---

### `swap` — Swap tokens

```bash
# Preview (no --confirm):
aerodrome-amm-plugin swap --token-in WETH --token-out USDC --amount-in 0.01

# Execute:
aerodrome-amm-plugin swap --token-in WETH --token-out USDC --amount-in 0.01 --confirm

# Force stable pool:
aerodrome-amm-plugin swap --token-in USDC --token-out USDT --amount-in 100 --stable --confirm
```

Auto-selects the pool giving the best output. Approves token_in to the Router if allowance is insufficient (idempotent check before approval). Approval is scoped to the exact swap amount.

| Flag | Default | Description |
|------|---------|-------------|
| `--token-in` | required | Input token |
| `--token-out` | required | Output token |
| `--amount-in` | required | Amount to swap |
| `--slippage` | `0.5` | Slippage tolerance % |
| `--stable` | false | Force stable pool |
| `--deadline-minutes` | `20` | Tx deadline |
| `--confirm` | false | Broadcast on-chain |
| `--dry-run` | false | Build calldata only |

**Preview output:**
```json
{
  "preview": true,
  "action": "swap",
  "token_in": "WETH",
  "token_out": "USDC",
  "amount_in": "0.01",
  "expected_out": "22.817474",
  "minimum_out": "22.703386",
  "slippage": "0.5%",
  "pool_type": "volatile",
  "router": "0xcF77a3Ba9A5CA399B7c97c74d54e5b1Beb874E43",
  "chain": "Base (8453)"
}
```

---

### `pools` — List pools for a token pair

```bash
aerodrome-amm-plugin pools --token-a WETH --token-b USDC
aerodrome-amm-plugin pools --token-a USDC --token-b USDT
```

Returns reserve, price, and total LP supply for volatile and stable pools on the pair.

---

### `prices` — Token price from AMM reserves

```bash
aerodrome-amm-plugin prices --token WETH
aerodrome-amm-plugin prices --token AERO --quote WETH
```

| Flag | Default | Description |
|------|---------|-------------|
| `--token` | required | Token to price |
| `--quote` | `USDC` | Quote currency |

---

### `positions` — Show LP positions

```bash
aerodrome-amm-plugin positions --token-a WETH --token-b USDC
aerodrome-amm-plugin positions --token-a USDC --token-b USDT --stable
```

Shows LP token balance, pool share %, and estimated underlying token amounts for the active wallet.

| Flag | Default | Description |
|------|---------|-------------|
| `--token-a` | required | First token of the pair |
| `--token-b` | required | Second token of the pair |
| `--stable` | false | Check stable pool only (default: checks both) |

---

### `add-liquidity` — Provide liquidity

```bash
# Preview:
aerodrome-amm-plugin add-liquidity --token-a WETH --token-b USDC --amount-a 0.01 --amount-b 22.0

# Execute:
aerodrome-amm-plugin add-liquidity --token-a WETH --token-b USDC --amount-a 0.01 --amount-b 22.0 --confirm

# Stable pool:
aerodrome-amm-plugin add-liquidity --token-a USDC --token-b USDT --amount-a 100 --amount-b 100 --stable --confirm
```

Calls `quoteAddLiquidity` first to show actual amounts used (may be adjusted to match pool ratio). Approves both tokens to Router if needed (scoped to the deposit amount). Returns LP tokens to your wallet.

| Flag | Default | Description |
|------|---------|-------------|
| `--token-a` | required | First token |
| `--token-b` | required | Second token |
| `--amount-a` | required | Desired amount of token_a |
| `--amount-b` | required | Desired amount of token_b |
| `--stable` | false | Add to stable pool |
| `--slippage` | `0.5` | Slippage tolerance % |
| `--deadline-minutes` | `20` | Tx deadline |
| `--confirm` | false | Broadcast on-chain |
| `--dry-run` | false | Build calldata only |

---

### `remove-liquidity` — Withdraw from pool

```bash
# Remove 50% of your position:
aerodrome-amm-plugin remove-liquidity --token-a WETH --token-b USDC --percent 50 --confirm

# Remove exact LP amount:
aerodrome-amm-plugin remove-liquidity --token-a WETH --token-b USDC --liquidity 0.001 --confirm

# Remove 100% from stable pool:
aerodrome-amm-plugin remove-liquidity --token-a USDC --token-b USDT --percent 100 --stable --confirm
```

Approves LP tokens (pool contract) to the Router (scoped to the withdrawal amount), then calls `removeLiquidity`. Run `positions` first to see your LP balance.

| Flag | Default | Description |
|------|---------|-------------|
| `--token-a` | required | First token |
| `--token-b` | required | Second token |
| `--liquidity` | — | Exact LP amount to burn |
| `--percent` | — | Percentage of LP balance (1–100) |
| `--stable` | false | Remove from stable pool |
| `--slippage` | `0.5` | Slippage tolerance % |
| `--confirm` | false | Broadcast on-chain |
| `--dry-run` | false | Build calldata only |

One of `--liquidity` or `--percent` is required.

---

### `claim-fees` — Collect trading fees

```bash
# Preview:
aerodrome-amm-plugin claim-fees --token-a WETH --token-b USDC

# Execute:
aerodrome-amm-plugin claim-fees --token-a WETH --token-b USDC --confirm
aerodrome-amm-plugin claim-fees --token-a USDC --token-b USDT --stable --confirm
```

Calls `claimFees()` on the pool. Accrued trading fees (proportional to your pool share and trading volume) are sent directly to your wallet. Fee amounts are determined on-chain at execution time.

| Flag | Default | Description |
|------|---------|-------------|
| `--token-a` | required | First token |
| `--token-b` | required | Second token |
| `--stable` | false | Claim from stable pool |
| `--confirm` | false | Broadcast on-chain |
| `--dry-run` | false | Build calldata only |

---

## Pool Types: Volatile vs Stable

| | Volatile | Stable |
|-|---------|--------|
| AMM formula | xy=k | x^3y + xy^3 = k |
| Best for | Uncorrelated (WETH/USDC, WETH/AERO) | Pegged (USDC/USDT, EURC/USDC) |
| Price impact | Higher for large trades | Lower for correlated assets |
| `--stable` flag | not needed | required |

The `swap` and `quote` commands automatically try both and pick the better output.

---

## Supported Tokens

| Symbol | Address (Base) |
|--------|---------------|
| WETH | `0x4200000000000000000000000000000000000006` |
| USDC | `0x833589fcd6edb6e08f4c7c32d4f71b54bda02913` |
| AERO | `0x940181a94a35a4569e4529a3cdfb74e38fd98631` |
| USDT | `0xfde4c96c8593536e31f229ea8f37b2ada2699bb2` |
| DAI  | `0x50c5725949a6f0c72e6c4a641f24049a917db0cb` |
| cbETH | `0x2ae3f1ec7f1f5012cfeab0185bfc7aa3cf0dec22` |
| cbBTC | `0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf` |
| EURC | `0x60a3e35cc302bfa44cb288bc5a4f316fdb1adb42` |

Any ERC-20 with an Aerodrome AMM pool can be used by passing its address directly.

---

## Changelog

### v0.1.0 (2026-04-28)
- Initial release: 8 commands covering full AMM lifecycle on Base
- Volatile and stable pool support with auto-selection on best output
- `quoteAddLiquidity` preview for accurate add-liquidity estimates
- On-chain allowance checks before approval (idempotent)
- `claimFees()` for LP fee collection
