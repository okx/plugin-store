---
name: one-click-token-launch
version: "1.0.0"
description: "One-click multi-launchpad token creation with bundled buy, IPFS metadata, MEV protection across 6 launchpads on Solana and BSC"
updated: 2026-04-13
triggers: >
  一键发币、发币、创建代币、launch token、create token、deploy token、mint token、
  launch meme coin、pump.fun launch、create a new token on Solana or BSC
---

# 一键发币 v1.0 — Multi-Launchpad Token Launch

> One-click token creation with optional bundled initial buy across 6+ launchpads.

---

## Disclaimer

**This skill is for educational and research purposes only. It does NOT constitute investment advice.**

1. **High Risk**: Launching tokens on bonding curve launchpads involves significant financial risk. Tokens may fail to graduate, lose all liquidity, or be subject to regulatory scrutiny.
2. **Irreversible**: On-chain token creation is permanent. Once launched, a token cannot be un-created.
3. **Fees**: Each launchpad charges platform fees. Jito bundles, priority fees, and gas costs add up. Review all costs before launching.
4. **Regulatory**: Token creation may be subject to securities regulations in your jurisdiction. Users are responsible for compliance with all applicable laws.
5. **AS-IS**: This skill is provided without warranty. All actions and consequences are the user's responsibility.

---

## Security Model

### TEE Signing

All on-chain write operations (token creation, buys, transfers) are signed via the onchainos Agentic Wallet running inside a Trusted Execution Environment (TEE). No private keys are stored in code or environment variables. The signing flow is:

1. Adapter builds an unsigned transaction (via launchpad API or ABI encoding)
2. Transaction is passed to `onchainos wallet contract-call --unsigned-tx` (Solana) or `--input-data` (EVM)
3. The TEE wallet signs and broadcasts the transaction
4. Confirmation is polled via `onchainos wallet history --tx-hash`

### Untrusted Data Boundary

External data enters the system at these points:

| Source | Data | Validation |
|--------|------|------------|
| PumpPortal API | Unsigned transaction bytes | Deserialized and verified before TEE signing |
| Bags.fm API | Token mint, metadata URL, serialized TX | Token mint checked, TX passed to TEE |
| Moonit API | Serialized TX, token mint | TX passed to TEE for signing |
| User input | Token name, symbol, description, image | Length limits enforced, image format validated |
| IPFS upload | CID hash | Immutable content-addressed -- no validation needed |
| Pinata API | Upload response (CID) | CID format validated |

User-supplied strings (name, symbol, description) are passed to launchpad APIs and on-chain metadata. They are NOT used in shell commands or SQL queries. Image files are validated for format and size before IPFS upload.

### Confirmation Gate

Live mode (`DRY_RUN=False`) always requires explicit user confirmation (typing "confirm") before any on-chain transaction. The `auto_confirm` parameter only applies in DRY_RUN mode. This prevents accidental irreversible token creation.

---

## File Structure

```
Token Launch/
├── SKILL.md              ← This file (strategy spec)
├── config.py             ← All configurable parameters
├── token_launch.py       ← Main program
├── launchpads/           ← Per-launchpad adapters
│   ├── __init__.py
│   ├── base.py           ← Abstract base class
│   ├── pumpfun.py        ← pump.fun via PumpPortal API
│   ├── bags.py           ← Bags.fm via REST SDK
│   ├── letsbonk.py       ← LetsBonk via API
│   ├── moonit.py         ← Moonit via SDK
│   ├── fourmeme.py       ← Four.Meme (BSC) via contract
│   └── flap.py           ← Flap.sh (BSC) via contract
├── ipfs.py               ← IPFS upload (pump.fun free endpoint + Pinata fallback)
├── post_launch.py        ← Post-launch monitor
├── dashboard.html        ← Web Dashboard UI
├── requirements.txt      ← Python dependencies
└── state/                ← [Auto-generated]
    └── launches.json     ← Launch history
```

---

## Prerequisites

### 1. Install onchainos CLI (>= 2.1.0)

```bash
onchainos --version
# If not installed, follow onchainos official docs
```

### 2. Login to Agentic Wallet (TEE Signing)

```bash
onchainos wallet login <your-email>
onchainos wallet status
# → loggedIn: true

# Confirm Solana address
onchainos wallet balance --chain solana
# → address: 2HNq...ErwW

# Confirm BSC address (if using BSC launchpads)
onchainos wallet balance --chain bsc
```

### 3. IPFS Upload (No Setup Needed)

IPFS upload uses pump.fun's free `/api/ipfs` endpoint by default — **no API key required**.

Optional fallback: Pinata (set `export PINATA_JWT="your_jwt"` if you want redundancy).

### 4. Python Dependencies

```bash
pip install -r requirements.txt
# or manually:
pip install httpx base58 solders
```

---

## Supported Launchpads

### Solana

| Launchpad | Protocol | Migration Target | Bundled Buy | MEV Protection | API Type |
|-----------|----------|------------------|-------------|----------------|----------|
| **pump.fun** | pump.fun bonding curve | Raydium | Yes (Jito bundle) | Jito bundle | PumpPortal REST |
| **Bags.fm** | Meteora DBC | Meteora | Yes (atomic) | Built-in | Official REST SDK |
| **LetsBonk** | Bonk bonding curve | Raydium | Yes | Built-in | MCP / REST |
| **Moonit** | Moonit bonding curve | Raydium/Meteora | Yes | Built-in | Official SDK |

### BSC

| Launchpad | Protocol | Migration Target | Bundled Buy | Tax Token | API Type |
|-----------|----------|------------------|-------------|-----------|----------|
| **Four.Meme** | Four.Meme bonding curve | PancakeSwap | Yes | No | Contract call |
| **Flap.sh** | Flap bonding curve | PancakeSwap V2/V3 | Yes | Yes (buy/sell tax) | Contract call |

---

## User Flow

### Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│  USER: "发币" / "launch token" / "create a meme coin"                │
└────────────────────────────┬─────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────────┐
│  STEP 1: BASIC INFO                                                  │
│                                                                      │
│  ┌──────────────┬───────────────────────────────────────────────┐    │
│  │ Token Name   │  "DogWifHat"                     [Required]   │    │
│  │ Ticker       │  "WIF"                           [Required]   │    │
│  │ Description  │  "The dog with the hat"          [Required]   │    │
│  │ Image        │  ./wif.png or URL                [Required]   │    │
│  └──────────────┴───────────────────────────────────────────────┘    │
└────────────────────────────┬─────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────────┐
│  STEP 2: SOCIAL LINKS (Optional)                                     │
│                                                                      │
│  ┌──────────────┬───────────────────────────────────────────────┐    │
│  │ Website      │  <your-website-url>           [Optional]   │    │
│  │ Twitter / X  │  <your-twitter-url>         [Optional]   │    │
│  │ Telegram     │  <your-telegram-url>          [Optional]   │    │
│  └──────────────┴───────────────────────────────────────────────┘    │
└────────────────────────────┬─────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────────┐
│  STEP 3: CHOOSE LAUNCHPAD                                            │
│                                                                      │
│  Solana:                                                             │
│  ┌─────┬──────────────────────────────────────────────────────────┐  │
│  │  1  │ 🟢 pump.fun   — Largest SOL launchpad, Raydium migrate  │  │
│  │  2  │ 🔵 Bags.fm    — Fee sharing, Meteora DBC               │  │
│  │  3  │ 🟡 LetsBonk   — BONK ecosystem, Raydium migrate        │  │
│  │  4  │ 🟠 Moonit     — Creator rewards, 80% fee share         │  │
│  └─────┴──────────────────────────────────────────────────────────┘  │
│                                                                      │
│  BSC:                                                                │
│  ┌─────┬──────────────────────────────────────────────────────────┐  │
│  │  5  │ 🔴 Four.Meme  — Largest BSC launchpad, PCS migrate     │  │
│  │  6  │ 🟣 Flap.sh    — Tax tokens, vanity addr, PCS V3        │  │
│  └─────┴──────────────────────────────────────────────────────────┘  │
│                                                                      │
│  Default: pump.fun (if user doesn't specify)                         │
└────────────────────────────┬─────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────────┐
│  STEP 4: LAUNCHPAD-SPECIFIC CONFIG                                   │
│                                                                      │
│  ┌─── pump.fun ────────────────────────────────────────────────────┐ │
│  │  Category:      (not applicable — pump.fun has no categories)   │ │
│  │  Priority Fee:  0.0005 SOL (default)                            │ │
│  │  Tip Fee:       0.0001 SOL (default)                            │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  ┌─── Bags.fm ─────────────────────────────────────────────────────┐ │
│  │  Fee Sharing:   Creator 100% (default) or split with others     │ │
│  │  Fee Claimers:  [{address, bps}] — must total 10,000 bps       │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  ┌─── Four.Meme ───────────────────────────────────────────────────┐ │
│  │  Category:      Meme/AI/DeFi/Games/Infra/De-Sci/Social/...     │ │
│  │  Gas Price:     auto (default) or custom wei                    │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  ┌─── Flap.sh ─────────────────────────────────────────────────────┐ │
│  │  Category:      (via extensionData)                             │ │
│  │  Buy Tax:       0-10000 bps                                     │ │
│  │  Sell Tax:      0-10000 bps                                     │ │
│  │  Tax Duration:  seconds                                         │ │
│  │  Tax Split:     mktBps + deflationBps + dividendBps + lpBps     │ │
│  │  DEX Target:    PancakeSwap V2 or V3                            │ │
│  │  LP Fee Tier:   (if V3)                                         │ │
│  │  Vanity Salt:   bytes32 (optional, for custom token address)    │ │
│  └─────────────────────────────────────────────────────────────────┘ │
└────────────────────────────┬─────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────────┐
│  STEP 5: BUNDLED INITIAL BUY (捆绑买入)                               │
│                                                                      │
│  "Buy your own token at launch?"                                     │
│                                                                      │
│  ┌──────────────────┬────────────────────────────────────────────┐   │
│  │ Buy Amount       │  0 = create only                           │   │
│  │                  │  0.5 SOL = buy at launch (bundled)         │   │
│  │ MEV Protection   │  ON (Jito bundle) — default, recommended  │   │
│  │ Slippage         │  10% (default for bonding curve buys)      │   │
│  └──────────────────┴────────────────────────────────────────────┘   │
│                                                                      │
│  How it works:                                                       │
│  • buyAmount = 0  → Token creation TX only                           │
│  • buyAmount > 0  → Create + Buy in ONE atomic Jito bundle           │
│  •                   No one can front-run your initial purchase       │
│  • Platform fees are deducted from buyAmount automatically            │
│                                                                      │
│  Balance check:                                                      │
│  • SOL: need buyAmount + 0.02 SOL (fees + rent)                      │
│  • BSC: need buyAmount + 0.015 BNB (gas)                             │
└────────────────────────────┬─────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────────┐
│  STEP 6: CONFIRMATION TABLE                                          │
│                                                                      │
│  ┌────────────────┬──────────────────────────────────────────────┐   │
│  │ Launchpad      │ pump.fun                                     │   │
│  │ Chain          │ Solana                                       │   │
│  │ Token Name     │ DogWifHat                                    │   │
│  │ Ticker         │ WIF                                          │   │
│  │ Description    │ The dog with the hat                         │   │
│  │ Image          │ wif.png (420x420, 85KB)                      │   │
│  │ Website        │ <your-website-url>                        │   │
│  │ Twitter        │ <your-twitter-url>                      │   │
│  │ Telegram       │ <your-telegram-url>                       │   │
│  │ ─────────────  │ ──────────────────────────                   │   │
│  │ Wallet         │ 2HNq...ErwW (1.23 SOL)                      │   │
│  │ Initial Buy    │ 0.5 SOL                                      │   │
│  │ MEV Protection │ ON (Jito bundle)                             │   │
│  │ Slippage       │ 10%                                          │   │
│  │ Priority Fee   │ 0.0001 SOL                                   │   │
│  │ Est. Cost      │ ~0.52 SOL (buy + fees + rent)                │   │
│  └────────────────┴──────────────────────────────────────────────┘   │
│                                                                      │
│  ⚡ Type "confirm" to launch. Type "cancel" to abort.                │
│                                                                      │
│  ⚠️  This is IRREVERSIBLE. The token will be created on-chain.       │
└────────────────────────────┬─────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────────┐
│  STEP 7: EXECUTION (what happens under the hood)                     │
│                                                                      │
│  7a. Upload image to IPFS (pump.fun free endpoint, Pinata fallback)                                   │
│      → ipfs://QmXxx...                                               │
│                                                                      │
│  7b. Create metadata JSON, upload to IPFS                            │
│      {                                                               │
│        "name": "DogWifHat",                                          │
│        "symbol": "WIF",                                              │
│        "description": "The dog with the hat",                        │
│        "image": "ipfs://QmXxx...",                                   │
│        "twitter": "<your-twitter-url>",                        │
│        "telegram": "<your-telegram-url>",                        │
│        "website": "<your-website-url>"                            │
│      }                                                               │
│      → ipfs://QmYyy... (metadata URI)                               │
│                                                                      │
│  7c. Call launchpad adapter:                                         │
│      pump.fun  → PumpPortal /api/trade-local (action: create)        │
│      Bags      → SDK createLaunchTransaction()                       │
│      Moonit    → SDK prepareMintTx()                                 │
│      LetsBonk  → REST API                                            │
│      Four.Meme → onchainos wallet contract-call (user confirms first) │
│      Flap      → onchainos wallet contract-call (user confirms first) │
│                                                                      │
│  7d. If buyAmount > 0:                                               │
│      • Bundle: [CreateToken IX, Buy IX] → Jito bundle (SOL)         │
│      • Or atomic contract call with value (BSC)                      │
│                                                                      │
│  7e. Sign via onchainos wallet (TEE)                                 │
│                                                                      │
│  7f. Submit to chain                                                 │
│      • SOL: submit Jito bundle → wait ~25s                           │
│      • BSC: broadcast tx → wait ~3-5s                                │
└────────────────────────────┬─────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────────┐
│  STEP 8: RESULT                                                      │
│                                                                      │
│  ✅ Token Launched Successfully!                                      │
│                                                                      │
│  ┌────────────────┬──────────────────────────────────────────────┐   │
│  │ Token Name     │ DogWifHat (WIF)                              │   │
│  │ Token Address  │ 7xKXtg2CW87d97TXJSDpbD5jBkhT...pump         │   │
│  │ TX Hash        │ 4nF8kJ...                                    │   │
│  │ Initial Buy    │ 0.5 SOL → 12,500,000 WIF                    │   │
│  │ Launchpad      │ pump.fun                                     │   │
│  │ Explorer       │ https://solscan.io/tx/4nF8kJ...              │   │
│  │ Trade Page     │ https://pump.fun/7xKXtg2CW87d...             │   │
│  └────────────────┴──────────────────────────────────────────────┘   │
│                                                                      │
│  Next steps:                                                         │
│  • "sell 50% WIF" — sell via onchainos swap                          │
│  • "buy more WIF" — buy more via onchainos swap                      │
│  • "check WIF" — view token info, holders, liquidity                 │
│  • Share the trade page link to promote your token                   │
└──────────────────────────────────────────────────────────────────────┘
```

---

## AI Agent Startup Interaction Protocol

> **When a user requests to launch a token, the AI Agent must follow the procedure below. Do not skip directly to launch.**

### Phase 1: Present Strategy Overview

Show the user the following:

```
一键发币 v1.0 — Multi-Launchpad Token Launch

This skill creates tokens on bonding curve launchpads with optional bundled initial buy.
Supports 6 launchpads: pump.fun, Bags.fm, LetsBonk, Moonit (Solana) + Four.Meme, Flap.sh (BSC).
IPFS metadata upload is handled automatically (pump.fun free endpoint, no API key needed).
Bundled buy creates token + initial buy in ONE atomic Jito bundle — no front-running.
All signing via onchainos Agentic Wallet (TEE) — no private keys in code.

Current: Paper Mode (DRY_RUN=True) — no real on-chain transactions.

Risk Notice: Token creation is IRREVERSIBLE. You may lose all invested capital.
```

### Q1: Choose Launchpad (Optional — default pump.fun)

| # | Launchpad | Chain | Notes |
|---|-----------|-------|-------|
| 1 | pump.fun | Solana | Largest SOL launchpad, Raydium migration, Jito MEV protection |
| 2 | Bags.fm | Solana | Fee sharing, Meteora DBC |
| 3 | LetsBonk | Solana | BONK ecosystem, Raydium migration |
| 4 | Moonit | Solana | 80% creator fee share |
| 5 | Four.Meme | BSC | Largest BSC launchpad, PancakeSwap migration |
| 6 | Flap.sh | BSC | Tax tokens, vanity addresses, PCS V3 |

If user doesn't specify → default to pump.fun.

### Q2: Token Details (Required)

Collect from user:
- **Name** — Token name (e.g., "MoonDog") [Required]
- **Symbol** — Ticker (e.g., "MDOG") [Required]
- **Description** — Short description [Required]
- **Image** — File path, URL, base64, or data URI [Required]
- **Website** — Project URL [Optional]
- **Twitter / X** — Twitter URL [Optional]
- **Telegram** — Telegram URL [Optional]

If user provides all in one message (e.g., "launch MoonDog MDOG on pump.fun, image is /tmp/dog.png"), extract directly — don't re-ask.

### Q3: Bundled Initial Buy?

- **A. Create only** (buy_amount = 0) — just create the token, no initial purchase
- **B. Buy at launch** — specify amount in SOL/BNB (e.g., 0.1 SOL)
  - Create + Buy bundled in ONE atomic Jito bundle (Solana) or contract call (BSC)
  - No one can front-run your initial purchase
  - Slippage: 10% default (configurable)

### Q4: Paper Mode or Live Mode?

- **A. Paper Mode** (default, recommended for first use) → `DRY_RUN = True`
  - Simulates the entire flow, no real on-chain TX
- **B. Live Mode** → `DRY_RUN = False`
  - Confirm with user: "Live Mode will create a REAL token on-chain. This is IRREVERSIBLE. Confirm?"
  - User confirms → set `DRY_RUN = False` in config.py
  - User declines → fall back to Paper Mode

### Launch

1. Modify `config.py` based on user responses (launchpad, DRY_RUN mode)
2. Check prerequisites: `onchainos --version`, `onchainos wallet status`
3. Install dependencies: `pip install -r requirements.txt`
4. Start dashboard: `python3 token_launch.py` (runs in background, serves at `http://localhost:3245`)
5. Show confirmation summary table (name, symbol, launchpad, buy amount, wallet, balance, mode)
6. Wait for user confirmation ("confirm" to launch, "cancel" to abort)
7. Execute via `quick_launch()` — one call handles everything
8. Show result: token address, TX hash, explorer link, trade page URL
9. Show Dashboard link: `http://localhost:3245`

### Special Cases

- User explicitly says "just launch it" or gives all details upfront → Extract params, show confirmation, launch (skip Q1-Q4 if info is complete)
- User says "use defaults" → pump.fun, Paper Mode, no initial buy, but still need name/symbol/description/image
- Returning user (previous launch in conversation) → Remind of previous config, ask whether to reuse

---

## Execution Rules

### Primary Entry Point: `quick_launch()`

One call does everything — wallet, IPFS, signing, broadcast, record-keeping:

```python
# token_launch.py auto-adds its directory to sys.path, so just point to the skill folder:
import sys, os
sys.path.insert(0, os.path.expanduser("~/path/to/Token Launch"))
from token_launch import quick_launch

# Minimal — just name, symbol, description, image:
result = await quick_launch("MoonDog", "MDOG", "a good boy", "/path/to/dog.png")

# Full options:
result = await quick_launch(
    "MoonDog", "MDOG", "a good boy", "<your-website-url>/dog.png",
    launchpad="pumpfun",   # pumpfun | bags | letsbonk | moonit | fourmeme | flap
    buy_amount=0.1,        # SOL/BNB — 0 = create only
    website="https://moondog.xyz",
    twitter="https://twitter.com/moondog",
    telegram="https://t.me/moondog",
)

# result.success, result.token_address, result.tx_hash, result.explorer_url
```

**Image input** — accepts any of:
- Local file path: `"/tmp/dog.png"`
- URL: `"<your-website-url>/dog.png"`
- Base64 data URI: `"data:image/png;base64,iVBOR…"`
- Raw base64 string

`quick_launch()` handles everything automatically:
1. Wallet login check + address resolution (cached after first call)
2. Balance check (reject early if insufficient)
3. Image normalization (download URL / decode base64 if needed)
4. IPFS upload (pump.fun free endpoint first, Pinata fallback)
5. Confirmation display (shows all params in a summary box)
6. Launch execution via the appropriate adapter
7. Record saved to `state/launches.json`
8. Lark notification (if `LARK_WEBHOOK` set)

### Configuration

`config.py` controls all defaults:
- `DRY_RUN = True` → simulate (no on-chain TX). Set `False` for real launches.
- `DEFAULT_LAUNCHPAD = "pumpfun"` → default when user doesn't specify
- `CONFIRM_REQUIRED = True` → show confirmation before launch

### Image Validation
- Accepted formats: PNG, JPG, GIF, WEBP
- Max size: 5 MB
- Recommended: square (1:1 ratio), minimum 200x200

### IPFS Upload
- **Primary**: pump.fun `/api/ipfs` — free, no API key needed, one call uploads image + creates Metaplex metadata
- **Fallback**: Pinata — requires `PINATA_JWT` env var
- No setup required for the primary path

### Safety
- ALWAYS show confirmation summary before execution
- NEVER auto-execute — token creation is irreversible
- If balance insufficient → reject with clear message, do NOT proceed
- If IPFS upload fails → abort with error
- If on-chain TX fails → show TX hash + error, do NOT retry

### Post-Launch
- Record saved to `state/launches.json`
- Explorer link + trade page URL returned in result
- Lark webhook notification (if `LARK_WEBHOOK` env is set)
- Post-launch monitor available: `python3 post_launch.py <token_address> --refresh 10`

---

## Launchpad Adapter Specs

### pump.fun (via PumpPortal)

**API Base**: `https://pumpportal.fun`

**Token Creation + Buy (bundled)**:
```
POST /api/trade-local
{
  "action": "create",
  "tokenMetadata": {
    "name": "DogWifHat",
    "symbol": "WIF",
    "uri": "https://gateway.pinata.cloud/ipfs/QmYyy..."
  },
  "mint": "<base58_mint_keypair>",
  "denominatedInSol": "true",
  "amount": 0.5,
  "slippage": 10,
  "priorityFee": 0.0001,
  "pool": "pump"
}
```

**Response**: unsigned transaction bytes (with mint keypair signature embedded by PumpPortal)

**Signing flow**:
1. Mint keypair generated locally (pump.fun protocol requirement)
2. Mint keypair secret passed to PumpPortal -- PumpPortal embeds the mint signature
3. Unsigned TX (needs only fee payer signature) sent to `onchainos wallet contract-call --unsigned-tx`
4. TEE wallet adds fee payer signature and broadcasts
5. Optional `--mev-protection` uses Jito bundle for front-run protection

**Notes**:
- Mint keypair is randomly generated client-side (protocol requirement)
- User wallet is the fee payer -- no ephemeral keypairs needed
- IPFS upload via pump.fun `/api/ipfs` (free, no API key) with Pinata fallback
- No platform fee on creation, standard fee on dev buy
- Pool options: "pump" (default) or "bonk" (LetsBonk pool)

---

### Bags.fm

**SDK**: `@bags-fm/sdk` (TypeScript) — we call via REST endpoints

**Flow**:
1. `POST /token-launch/create-token-info` — upload metadata (name, symbol, desc, image, socials)
2. `POST /fee-share/config` — create fee share config (creator BPS, optional co-earners)
3. `POST /token-launch/create-launch-transaction` — create launch TX with `initialBuyLamports`

**Fee Sharing**:
- Creator must set their BPS explicitly (no default allocation)
- Total must = 10,000 bps (100%)
- Max 100 fee earners per token
- Supports social username lookups (Twitter, GitHub, Kick)

**Notes**:
- Uses Meteora Dynamic Bonding Curve
- Bags handles IPFS upload internally via their API
- No external Pinata needed (optional)

---

### Moonit

**SDK**: `@moonit/sdk` (TypeScript) — Python wrapper calls SDK methods

**Flow**:
1. `prepareMintTx()` — builds mint transaction with token metadata
2. Sign transaction
3. `submitMintTx()` — submit signed transaction

**Notes**:
- Creator earns 80% of all trading fees
- Supports Raydium and Meteora V2 migration targets
- Built-in IPFS upload in SDK

---

### LetsBonk

**MCP Server**: `bonk-mcp` — or direct REST API

**Flow**:
1. Create token via API (name, symbol, metadata URI)
2. Optional initial buy
3. Submit to Solana

**Notes**:
- Part of BONK ecosystem
- Migrates to Raydium after bonding curve completion
- Pool option `pool: "bonk"` also available via PumpPortal

---

### Four.Meme (BSC)

**Method**: Direct contract interaction via `onchainos wallet contract-call`

**IMPORTANT**: The agent MUST display all transaction parameters and receive explicit user confirmation (typing "confirm") BEFORE executing any contract call. Never auto-execute.

**Flow**:
1. Upload image to IPFS (pump.fun free endpoint, Pinata fallback)
2. Create metadata (description, image CID, socials)
3. Display full transaction summary and wait for user to type "confirm"
4. Call Four.Meme factory contract: `createToken(name, symbol, metadataURI, ...)`
5. Include `msg.value` for initial buy (if buyAmount > 0)

**Notes**:
- Image upload handled by Four.Meme platform internally (if using their web UI)
- For programmatic: use Pinata, pass CID
- Categories: Meme, AI, DeFi, Games, Infra, De-Sci, Social, Depin, Charity, Others
- No tax token support on Four.Meme

---

### Flap.sh (BSC)

**Method**: Direct contract interaction via `onchainos wallet contract-call`

**IMPORTANT**: The agent MUST display all transaction parameters and receive explicit user confirmation (typing "confirm") BEFORE executing any contract call. Never auto-execute.

**Portal Contract**: `0xe2cE6ab80874Fa9Fa2aAE65D277Dd6B8e65C9De0` (BNB Mainnet)

**Function**: `newTokenV6(NewTokenV6Params)`

**Parameters**:
```
name:               string    — Token name
symbol:             string    — Token symbol
meta:               string    — IPFS CID of metadata
dexThresh:          uint8     — DEX listing threshold type
salt:               bytes32   — Vanity salt (0x0 for random)
migratorType:       uint8     — V2_MIGRATOR or V3_MIGRATOR
quoteToken:         address   — address(0) for native BNB
quoteAmt:           uint256   — Initial buy amount
beneficiary:        address   — Tax recipient
buyTaxRate:         uint16    — Buy tax (basis points)
sellTaxRate:        uint16    — Sell tax (basis points)
taxDuration:        uint256   — How long tax applies (seconds)
antiFarmerDuration: uint256   — Anti-dump duration (seconds)
mktBps:             uint16    — Marketing allocation from tax
deflationBps:       uint16    — Burn allocation from tax
dividendBps:        uint16    — Dividend allocation from tax
lpBps:              uint16    — LP allocation from tax
tokenVersion:       uint8     — 6 (TOKEN_TAXED_V3, recommended)
```

**Notes**:
- Supports asymmetric buy/sell tax rates
- Vanity token addresses via `salt` parameter
- Tax splits: mktBps + deflationBps + dividendBps + lpBps = total tax allocation
- DEX migration to PancakeSwap V2 or V3

---

## Dashboard

**Port**: 3245

**Features**:
- Numbered timeline list (newest at top) with token logos
- Bonding curve progress bar (live-updating for active tokens)
- Live stats: price, market cap, holders, buy/sell volume
- Wallet balance display and mode indicator (DRY RUN / LIVE)
- Social links, explorer links, and launchpad chips

---

## Config Reference

See `config.py` for all configurable parameters with descriptions.

---

## Quick Start Examples

### Launch on pump.fun (create only, no buy)
```
User: "Launch a token called MoonCat, ticker MCAT, on pump.fun"
→ Skill collects: description, image
→ Uploads to IPFS
→ Calls PumpPortal create (buyAmount = 0)
→ Returns token address
```

### Launch on pump.fun with bundled buy
```
User: "发币 CoolDog, ticker CDOG, buy 0.5 SOL"
→ Skill collects: description, image
→ Uploads to IPFS
→ Bundles: create TX + buy 0.5 SOL TX → Jito bundle
→ Returns token address + initial position
```

### Launch on Flap.sh with tax token
```
User: "Create a tax token on BSC via Flap, 5% buy tax, 3% sell tax"
→ Skill collects: name, ticker, desc, image, tax config
→ Uploads metadata to IPFS
→ Calls Flap portal newTokenV6() with tax params
→ Returns token address on BSC
```

### Launch on Bags.fm with fee sharing
```
User: "Launch on Bags, share 50% fees with my partner"
→ Skill collects: name, ticker, desc, image, partner address
→ Creates fee share config (creator 5000 bps, partner 5000 bps)
→ Creates launch TX
→ Returns token address + fee share config
```
