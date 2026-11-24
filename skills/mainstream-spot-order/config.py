"""Configuration for multi-chain spot trading system.

Built-in pairs: SOL, ETH, BTC, BNB, AVAX, DOGE
Custom pairs:   Add via `python3 config.py --add-pair` or edit pairs.json
"""
from __future__ import annotations

import json
import os
import sys

_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
_PAIRS_JSON = os.path.join(_SCRIPT_DIR, "pairs.json")

# ── Required keys for every pair ────────────────────────────────────
_REQUIRED_KEYS = (
    "chain_index", "chain_family", "base_mint", "base_symbol",
    "base_decimals", "quote_mint", "quote_decimals", "native_for_sell",
    "gas_reserve", "label",
)

# ── Built-in Pair Registry ──────────────────────────────────────────
# native_for_sell:
#   Solana native SOL → "11111111111111111111111111111111"
#   EVM native tokens → "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
#   ERC-20 tokens      → their contract address

_BUILTIN_PAIRS = {
    "SOL": {
        "chain_index": "501",
        "chain_family": "solana",
        "base_mint": "So11111111111111111111111111111111111111112",
        "base_symbol": "SOL",
        "base_decimals": 9,
        "quote_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "quote_decimals": 6,
        "native_for_sell": "11111111111111111111111111111111",
        "gas_reserve": 0.01,
        "label": "SOL/USDC",
    },
    "ETH": {
        "chain_index": "1",
        "chain_family": "evm",
        "base_mint": "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "base_symbol": "ETH",
        "base_decimals": 18,
        "quote_mint": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        "quote_decimals": 6,
        "native_for_sell": "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "gas_reserve": 0.005,
        "label": "ETH/USDC",
    },
    "BTC": {
        "chain_index": "1",
        "chain_family": "evm",
        "base_mint": "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599",
        "base_symbol": "BTC",
        "base_decimals": 8,
        "quote_mint": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        "quote_decimals": 6,
        "native_for_sell": "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599",
        "gas_reserve": 0.005,
        "label": "BTC/USDC",
    },
    "BNB": {
        "chain_index": "56",
        "chain_family": "evm",
        "base_mint": "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "base_symbol": "BNB",
        "base_decimals": 18,
        "quote_mint": "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d",
        "quote_decimals": 18,
        "native_for_sell": "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "gas_reserve": 0.005,
        "label": "BNB/USDC",
    },
    "AVAX": {
        "chain_index": "43114",
        "chain_family": "evm",
        "base_mint": "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "base_symbol": "AVAX",
        "base_decimals": 18,
        "quote_mint": "0xb97ef9ef8734c71904d8002f8b6bc66dd9c48a6e",
        "quote_decimals": 6,
        "native_for_sell": "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "gas_reserve": 0.1,
        "label": "AVAX/USDC",
    },
    "DOGE": {
        "chain_index": "1",
        "chain_family": "evm",
        "base_mint": "0x4206931337dc273a630d328da6441786bfad668f",
        "base_symbol": "DOGE",
        "base_decimals": 8,
        "quote_mint": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        "quote_decimals": 6,
        "native_for_sell": "0x4206931337dc273a630d328da6441786bfad668f",
        "gas_reserve": 0.005,
        "label": "DOGE/USDC",
    },
}


# ── Load user-defined pairs from pairs.json ─────────────────────────

def _load_custom_pairs() -> dict:
    """Load pairs.json if it exists. Returns dict of pair dicts."""
    if not os.path.exists(_PAIRS_JSON):
        return {}
    try:
        with open(_PAIRS_JSON, "r") as f:
            data = json.load(f)
        if not isinstance(data, dict):
            return {}
        return data
    except (json.JSONDecodeError, OSError):
        return {}


def _save_custom_pairs(custom: dict):
    """Write custom pairs to pairs.json."""
    with open(_PAIRS_JSON, "w") as f:
        json.dump(custom, f, indent=2)


# Merge: custom pairs override built-in pairs with the same key
PAIRS: dict[str, dict] = {**_BUILTIN_PAIRS, **_load_custom_pairs()}


# ── Active Pair (set by --pair arg at startup) ──────────────────────
ACTIVE_PAIR = "SOL"


def pair() -> dict:
    """Return the active pair config dict. Exits with clear message if unknown."""
    if ACTIVE_PAIR in PAIRS:
        return PAIRS[ACTIVE_PAIR]
    available = ", ".join(sorted(PAIRS.keys()))
    print(f"ERROR: Unknown pair '{ACTIVE_PAIR}'. Available: {available}")
    print(f"Add a custom pair: python3 config.py --add-pair {ACTIVE_PAIR}")
    sys.exit(1)


def register_pair(name: str, pair_dict: dict) -> None:
    """Register a new pair at runtime and persist to pairs.json.

    pair_dict must contain all required keys:
      chain_index, chain_family, base_mint, base_symbol, base_decimals,
      quote_mint, quote_decimals, native_for_sell, gas_reserve, label

    Raises ValueError if required keys are missing or chain_family is invalid.
    """
    missing = [k for k in _REQUIRED_KEYS if k not in pair_dict]
    if missing:
        raise ValueError(f"Missing required keys: {', '.join(missing)}")
    if pair_dict["chain_family"] not in ("solana", "evm"):
        raise ValueError(f"chain_family must be 'solana' or 'evm', got '{pair_dict['chain_family']}'")
    pair_dict["base_decimals"] = int(pair_dict["base_decimals"])
    pair_dict["quote_decimals"] = int(pair_dict["quote_decimals"])
    pair_dict["gas_reserve"] = float(pair_dict["gas_reserve"])

    # Update runtime registry
    PAIRS[name] = pair_dict

    # Persist to pairs.json (only custom pairs, not built-ins)
    custom = _load_custom_pairs()
    custom[name] = pair_dict
    _save_custom_pairs(custom)


# ── BTC Overlay (for non-Ethereum pairs fetching BTC reference) ────
BTC_OVERLAY_CHAIN = "1"
BTC_OVERLAY_MINT = "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599"

# ── Solana-era BTC reference (for backward compat with SOL pair) ───
WBTC_MINT = "3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh"

# ── Strategy / Backtest Globals (unchanged) ─────────────────────────
BAR_SIZE    = "15m"
BAR_SECONDS = 900

INITIAL_USDC = 1000.0
COST_PER_LEG = 0.003       # 0.3% DEX fee
SLIPPAGE_PCT = 0.005        # 0.5% assumed slippage
LIVE_USDC_PCT = 0.90        # 90% of balance per trade
MAX_DAILY_LOSS = 0.05       # 5% equity -> stop
MIN_TRADES_FOR_SCORE = 20

DASHBOARD_PORT = 3250

PAPER_TRADE = True          # True = simulate trades, False = real swaps


# ── Well-known USDC addresses per chain (for --add-pair helper) ─────
_USDC_BY_CHAIN = {
    "1":     ("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48", 6),    # Ethereum
    "56":    ("0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d", 18),   # BSC
    "43114": ("0xb97ef9ef8734c71904d8002f8b6bc66dd9c48a6e", 6),    # Avalanche
    "137":   ("0x3c499c542cef5e3811e1192ce70d8cc03d5c3359", 6),    # Polygon
    "42161": ("0xaf88d065e77c8cc2239327c5edb3a432268e5831", 6),    # Arbitrum
    "10":    ("0x0b2c639c533813f4aa9d7837caf62653d097ff85", 6),    # Optimism
    "8453":  ("0x833589fcd6edb6e08f4c7c32d4f71b54bda02913", 6),    # Base
    "501":   ("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", 6), # Solana
}

_EVM_NATIVE = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
_SOL_NATIVE = "11111111111111111111111111111111"


# ── CLI: add-pair helper ────────────────────────────────────────────

def _cli_add_pair():
    """Interactive CLI to register a new trading pair."""
    import argparse as _ap
    parser = _ap.ArgumentParser(description="Add a custom trading pair")
    parser.add_argument("name", help="Pair name, e.g. LINK, UNI, LTC")
    parser.add_argument("--chain", required=True, help="Chain index (1=ETH, 56=BSC, 43114=AVAX, 501=SOL, ...)")
    parser.add_argument("--mint", required=True, help="Base token contract address (or 'native' for chain-native)")
    parser.add_argument("--decimals", required=True, type=int, help="Base token decimals")
    parser.add_argument("--quote-mint", help="Quote token address (default: USDC on that chain)")
    parser.add_argument("--quote-decimals", type=int, help="Quote token decimals (default: auto from chain)")
    parser.add_argument("--gas-reserve", type=float, default=0.005, help="Gas reserve (default: 0.005)")
    args = parser.parse_args(sys.argv[2:])  # skip 'config.py' and '--add-pair'

    name = args.name.upper()
    chain = str(args.chain)
    is_solana = chain == "501"
    family = "solana" if is_solana else "evm"

    # Resolve mint
    if args.mint.lower() == "native":
        base_mint = "So11111111111111111111111111111111111111112" if is_solana else _EVM_NATIVE
        native_for_sell = _SOL_NATIVE if is_solana else _EVM_NATIVE
    else:
        base_mint = args.mint
        # If mint is the native placeholder, selling uses native address; otherwise use contract
        if base_mint.lower() == _EVM_NATIVE:
            native_for_sell = _EVM_NATIVE
        elif is_solana and base_mint == "So11111111111111111111111111111111111111112":
            native_for_sell = _SOL_NATIVE
        else:
            native_for_sell = base_mint  # ERC-20 / SPL token

    # Resolve quote
    if args.quote_mint:
        quote_mint = args.quote_mint
        quote_dec = args.quote_decimals or 6
    elif chain in _USDC_BY_CHAIN:
        quote_mint, quote_dec = _USDC_BY_CHAIN[chain]
    else:
        print(f"ERROR: No default USDC known for chain {chain}. Use --quote-mint and --quote-decimals.")
        sys.exit(1)

    if args.quote_decimals is not None:
        quote_dec = args.quote_decimals

    pair_dict = {
        "chain_index": chain,
        "chain_family": family,
        "base_mint": base_mint,
        "base_symbol": name,
        "base_decimals": args.decimals,
        "quote_mint": quote_mint,
        "quote_decimals": quote_dec,
        "native_for_sell": native_for_sell,
        "gas_reserve": args.gas_reserve,
        "label": f"{name}/USDC",
    }

    register_pair(name, pair_dict)
    print(f"Registered pair: {name}")
    print(json.dumps(pair_dict, indent=2))
    print(f"\nUsage: python3 live.py --pair {name}")


def _cli_list_pairs():
    """Print all available pairs."""
    custom = _load_custom_pairs()
    for name in sorted(PAIRS.keys()):
        p = PAIRS[name]
        tag = " [custom]" if name in custom else ""
        print(f"  {name:6s}  {p['label']:12s}  chain={p['chain_index']:5s}  "
              f"family={p['chain_family']:6s}  decimals={p['base_decimals']}{tag}")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--add-pair":
        _cli_add_pair()
    elif len(sys.argv) > 1 and sys.argv[1] == "--list":
        _cli_list_pairs()
    else:
        print("Usage:")
        print("  python3 config.py --list                              List all pairs")
        print("  python3 config.py --add-pair LINK --chain 1 --mint 0x... --decimals 18")
        print("")
        _cli_list_pairs()
