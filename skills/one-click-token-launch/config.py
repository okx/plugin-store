"""
一键发币 v1.0 — Configuration
Modify this file to adjust defaults. No need to change token_launch.py.

⚠️ Disclaimer:
This skill is for educational and research purposes only.
Token creation is irreversible. Review all parameters carefully.
"""

# ── Runtime Mode ──────────────────────────────────────────────────────
DRY_RUN         = True      # True=simulate (no on-chain TX), False=real launch
CONFIRM_REQUIRED = True     # Always require user confirmation before launch

# ── Default Launchpad ─────────────────────────────────────────────────
# Options: "pumpfun", "bags", "letsbonk", "moonit", "fourmeme", "flap"
DEFAULT_LAUNCHPAD = "pumpfun"

# ── Wallet ────────────────────────────────────────────────────────────
# Resolved automatically from onchainos wallet at startup
# Override only if you want a specific address
WALLET_SOL       = ""       # Leave empty = auto-detect from onchainos
WALLET_BSC       = ""       # Leave empty = auto-detect from onchainos

# ── IPFS (Pinata) ────────────────────────────────────────────────────
# Get your JWT at https://app.pinata.cloud/developers/api-keys
PINATA_JWT       = ""       # Set via env: export PINATA_JWT="your_jwt"
PINATA_GATEWAY   = "https://gateway.pinata.cloud/ipfs"
IPFS_TIMEOUT     = 30       # Upload timeout (seconds)

# ── Image ─────────────────────────────────────────────────────────────
IMAGE_MAX_SIZE   = 5 * 1024 * 1024   # 5 MB
IMAGE_FORMATS    = {"png", "jpg", "jpeg", "gif", "webp"}
IMAGE_MIN_DIM    = 200      # Minimum width/height (pixels)

# ── Bundled Buy Defaults ──────────────────────────────────────────────
DEFAULT_BUY_AMOUNT   = 0.0  # 0 = create only, >0 = bundled buy (native token)
DEFAULT_SLIPPAGE_BPS = 1000 # 10% (basis points) — bonding curve buys need high slippage
MEV_PROTECTION       = True # Use Jito bundle (SOL) / MEV bundle (BSC)

# ══════════════════════════════════════════════════════════════════════
# Per-Launchpad Configuration
# ══════════════════════════════════════════════════════════════════════

# ── pump.fun ──────────────────────────────────────────────────────────
PUMPFUN_API_BASE     = "https://pumpportal.fun"
PUMPFUN_POOL         = "pump"        # "pump" or "bonk" (LetsBonk pool)
PUMPFUN_PRIORITY_FEE = 0.0005        # SOL (priority fee for Solana validators)
PUMPFUN_TIP_FEE      = 0.0001        # SOL (Jito tip, only when MEV_PROTECTION=True)
PUMPFUN_TX_TIMEOUT   = 30            # seconds to wait for confirmation

# Jito bundle endpoint
JITO_BUNDLE_URL      = "https://mainnet.block-engine.jito.wtf/api/v1/bundles"

# ── Bags.fm ───────────────────────────────────────────────────────────
BAGS_API_BASE        = "https://api.bags.fm"
BAGS_DEFAULT_FEE_BPS = 10000         # 100% to creator (10000 bps = 100%)
# Fee sharing: list of {address, bps} dicts. Must total 10000.
# Example: [{"address": "Creator...", "bps": 5000}, {"address": "Partner...", "bps": 5000}]
BAGS_FEE_CLAIMERS    = []            # Empty = 100% to creator

# ── LetsBonk ──────────────────────────────────────────────────────────
LETSBONK_API_BASE    = "https://api.letsbonk.fun"
LETSBONK_PRIORITY_FEE = 0.0005       # SOL

# ── Moonit ────────────────────────────────────────────────────────────
MOONIT_API_BASE      = "https://api.moon.it"
MOONIT_MIGRATION_DEX = "RAYDIUM"     # "RAYDIUM" or "METEORA_V2"

# ── Four.Meme (BSC) ──────────────────────────────────────────────────
FOURMEME_FACTORY     = ""            # Factory contract address — REQUIRED for Four.Meme launches. Leave empty to disable.
FOURMEME_CATEGORY    = "Meme"        # Meme/AI/DeFi/Games/Infra/De-Sci/Social/Depin/Charity/Others
FOURMEME_GAS_PRICE   = ""            # Empty = auto, or wei string like "3000000000"

# ── Flap.sh (BSC) ────────────────────────────────────────────────────
FLAP_PORTAL          = "0xe2cE6ab80874Fa9Fa2aAE65D277Dd6B8e65C9De0"  # BNB Mainnet
FLAP_TOKEN_VERSION   = 6             # TOKEN_TAXED_V3 (recommended)
FLAP_MIGRATOR_TYPE   = 1             # 0 = V2_MIGRATOR, 1 = V3_MIGRATOR
FLAP_DEX_ID          = 0             # 0 = PancakeSwap
FLAP_LP_FEE_PROFILE  = 1             # LP fee tier (for V3)
# Default tax config (basis points)
FLAP_BUY_TAX         = 0             # Buy tax bps (0 = no tax)
FLAP_SELL_TAX        = 0             # Sell tax bps (0 = no tax)
FLAP_TAX_DURATION    = 0             # How long tax applies (seconds, 0 = forever)
FLAP_ANTI_FARMER     = 0             # Anti-dump duration (seconds)
# Tax allocation split (must equal total tax collected)
FLAP_MKT_BPS         = 10000         # Marketing %
FLAP_DEFLATION_BPS   = 0             # Burn %
FLAP_DIVIDEND_BPS    = 0             # Dividend %
FLAP_LP_BPS          = 0             # LP %

# ── Dashboard ─────────────────────────────────────────────────────────
DASHBOARD_PORT       = 3245

# ── Notifications ─────────────────────────────────────────────────────
LARK_WEBHOOK         = ""            # Set via env: export LARK_WEBHOOK="https://..."

# ── Chain Constants ───────────────────────────────────────────────────
SOL_CHAIN_INDEX      = "501"         # Solana
BSC_CHAIN_INDEX      = "56"          # BNB Smart Chain

# Map launchpad → chain
LAUNCHPAD_CHAIN = {
    "pumpfun":   "solana",
    "bags":      "solana",
    "letsbonk":  "solana",
    "moonit":    "solana",
    "fourmeme":  "bsc",
    "flap":      "bsc",
}

# Map launchpad → display name
LAUNCHPAD_DISPLAY = {
    "pumpfun":   "pump.fun",
    "bags":      "Bags.fm",
    "letsbonk":  "LetsBonk",
    "moonit":    "Moonit",
    "fourmeme":  "Four.Meme",
    "flap":      "Flap.sh",
}

# Minimum balance required (native token) beyond buyAmount
MIN_BALANCE_BUFFER = {
    "solana": 0.02,    # SOL (rent + fees)
    "bsc":   0.015,    # BNB (gas)
}
