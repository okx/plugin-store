"""
Known-spenders registry.

A curated set of well-established protocol addresses that legitimate users
commonly grant token approvals to. Entries are informational only — being on
this list reduces (but does not eliminate) the risk score of an approval.

All addresses stored lowercase, keyed by (chain_id, address). Chain IDs:
1 = Ethereum, 42161 = Arbitrum, 10 = Optimism, 8453 = Base,
137 = Polygon, 56 = BSC, 43114 = Avalanche.

This list is not exhaustive. Unknown spenders are NOT automatically treated as
malicious — they are flagged for manual review.
"""

# Blue-chip tokens — approvals on these carry higher stakes because the tokens
# are more liquid and valuable to drain. Lowercase.
BLUECHIP_TOKENS = {
    # Ethereum mainnet
    (1, "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"): "USDC",
    (1, "0xdac17f958d2ee523a2206206994597c13d831ec7"): "USDT",
    (1, "0x6b175474e89094c44da98b954eedeac495271d0f"): "DAI",
    (1, "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"): "WETH",
    (1, "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599"): "WBTC",
    # Arbitrum
    (42161, "0xaf88d065e77c8cc2239327c5edb3a432268e5831"): "USDC",
    (42161, "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9"): "USDT",
    (42161, "0x82af49447d8a07e3bd95bd0d56f35241523fbab1"): "WETH",
    # Optimism
    (10, "0x0b2c639c533813f4aa9d7837caf62653d097ff85"): "USDC",
    (10, "0x94b008aa00579c1307b0ef2c499ad98a8ce58e58"): "USDT",
    (10, "0x4200000000000000000000000000000000000006"): "WETH",
    # Base
    (8453, "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"): "USDC",
    (8453, "0x4200000000000000000000000000000000000006"): "WETH",
    # Polygon
    (137, "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359"): "USDC",
    (137, "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"): "USDT",
    (137, "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"): "WETH",
    # BSC
    (56, "0x55d398326f99059ff775485246999027b3197955"): "USDT",
    (56, "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d"): "USDC",
    (56, "0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c"): "WBNB",
    # Avalanche
    (43114, "0xb97ef9ef8734c71904d8002f8b6bc66dd9c48a6e"): "USDC",
    (43114, "0x9702230a8ea53601f5cd2dc00fdbc13d4df4a8c7"): "USDT.e",
    (43114, "0xb31f66aa3c1e785363f0875a1b74e27b85fd66c7"): "WAVAX",
}


# Well-known protocol spender addresses. Lowercase.
# Categories: dex-router, dex-aggregator, lending, permit-helper, bridge,
#             derivatives, yield-vault, launchpad.
KNOWN_SPENDERS = {
    # ── Ethereum mainnet ───────────────────────────────────────────────
    (1, "0xe592427a0aece92de3edee1f18e0157c05861564"):
        {"name": "Uniswap V3 SwapRouter", "category": "dex-router"},
    (1, "0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45"):
        {"name": "Uniswap V3 SwapRouter02", "category": "dex-router"},
    (1, "0x7a250d5630b4cf539739df2c5dacb4c659f2488d"):
        {"name": "Uniswap V2 Router02", "category": "dex-router"},
    (1, "0x000000000022d473030f116ddee9f6b43ac78ba3"):
        {"name": "Permit2 (Uniswap)", "category": "permit-helper"},
    (1, "0x66a9893cc07d91d95644aedd05d03f95e1dba8af"):
        {"name": "Uniswap V4 Universal Router", "category": "dex-router"},
    (1, "0x1111111254eeb25477b68fb85ed929f73a960582"):
        {"name": "1inch V5 Aggregation Router", "category": "dex-aggregator"},
    (1, "0x111111125421ca6dc452d289314280a0f8842a65"):
        {"name": "1inch V6 Aggregation Router", "category": "dex-aggregator"},
    (1, "0xdef1c0ded9bec7f1a1670819833240f027b25eff"):
        {"name": "0x Protocol ExchangeProxy", "category": "dex-aggregator"},
    (1, "0x881d40237659c251811cec9c364ef91dc08d300c"):
        {"name": "Metamask Swaps Router", "category": "dex-aggregator"},
    (1, "0xdef171fe48cf0115b1d80b88dc8eab59176fee57"):
        {"name": "Paraswap V5 AugustusSwapper", "category": "dex-aggregator"},
    (1, "0x6a000f20005980200259b80c5102003040001068"):
        {"name": "Paraswap V6 AugustusSwapper", "category": "dex-aggregator"},
    (1, "0x9008d19f58aabd9ed0d60971565aa8510560ab41"):
        {"name": "CowSwap GPv2Settlement", "category": "dex-aggregator"},
    (1, "0xc92e8bdf79f0507f65a392b0ab4667716bfe0110"):
        {"name": "CowSwap VaultRelayer", "category": "dex-aggregator"},
    (1, "0x87870bca3f3fd6335c3f4ce8392d69350b4fa4e2"):
        {"name": "Aave V3 Pool", "category": "lending"},
    (1, "0xc3d688b66703497daa19211eedff47f25384cdc3"):
        {"name": "Compound V3 cUSDCv3", "category": "lending"},
    (1, "0xbbbbbbbbbb9cc5e90e3b3af64bdaf62c37eeffcb"):
        {"name": "Morpho Blue", "category": "lending"},
    (1, "0xba12222222228d8ba445958a75a0704d566bf2c8"):
        {"name": "Balancer V2 Vault", "category": "dex-router"},
    (1, "0xdef1ca1fb7fbcdc777520aa7f396b4e015f497ab"):
        {"name": "CoW Protocol Router", "category": "dex-aggregator"},

    # ── Arbitrum ───────────────────────────────────────────────────────
    (42161, "0x000000000022d473030f116ddee9f6b43ac78ba3"):
        {"name": "Permit2 (Uniswap)", "category": "permit-helper"},
    (42161, "0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45"):
        {"name": "Uniswap V3 SwapRouter02", "category": "dex-router"},
    (42161, "0x1111111254eeb25477b68fb85ed929f73a960582"):
        {"name": "1inch V5 Aggregation Router", "category": "dex-aggregator"},
    (42161, "0x111111125421ca6dc452d289314280a0f8842a65"):
        {"name": "1inch V6 Aggregation Router", "category": "dex-aggregator"},
    (42161, "0x794a61358d6845594f94dc1db02a252b5b4814ad"):
        {"name": "Aave V3 Pool", "category": "lending"},
    (42161, "0xb4a7d971d0adea1c73198c97d7ab3f9ce4aafa13"):
        {"name": "GMX Router", "category": "derivatives"},

    # ── Optimism ───────────────────────────────────────────────────────
    (10, "0x000000000022d473030f116ddee9f6b43ac78ba3"):
        {"name": "Permit2 (Uniswap)", "category": "permit-helper"},
    (10, "0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45"):
        {"name": "Uniswap V3 SwapRouter02", "category": "dex-router"},
    (10, "0x794a61358d6845594f94dc1db02a252b5b4814ad"):
        {"name": "Aave V3 Pool", "category": "lending"},
    (10, "0x1111111254eeb25477b68fb85ed929f73a960582"):
        {"name": "1inch V5 Aggregation Router", "category": "dex-aggregator"},

    # ── Base ───────────────────────────────────────────────────────────
    (8453, "0x000000000022d473030f116ddee9f6b43ac78ba3"):
        {"name": "Permit2 (Uniswap)", "category": "permit-helper"},
    (8453, "0x2626664c2603336e57b271c5c0b26f421741e481"):
        {"name": "Uniswap V3 SwapRouter02", "category": "dex-router"},
    (8453, "0x198ef79f1f515f02dfe9e3115ed9fc07183f02fc"):
        {"name": "Aerodrome Router", "category": "dex-router"},

    # ── Polygon ────────────────────────────────────────────────────────
    (137, "0x000000000022d473030f116ddee9f6b43ac78ba3"):
        {"name": "Permit2 (Uniswap)", "category": "permit-helper"},
    (137, "0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45"):
        {"name": "Uniswap V3 SwapRouter02", "category": "dex-router"},
    (137, "0x1111111254eeb25477b68fb85ed929f73a960582"):
        {"name": "1inch V5 Aggregation Router", "category": "dex-aggregator"},
    (137, "0x794a61358d6845594f94dc1db02a252b5b4814ad"):
        {"name": "Aave V3 Pool", "category": "lending"},

    # ── BSC ────────────────────────────────────────────────────────────
    (56, "0x10ed43c718714eb63d5aa57b78b54704e256024e"):
        {"name": "PancakeSwap V2 Router", "category": "dex-router"},
    (56, "0x13f4ea83d0bd40e75c8222255bc855a974568dd4"):
        {"name": "PancakeSwap V3 SmartRouter", "category": "dex-router"},
    (56, "0x1111111254eeb25477b68fb85ed929f73a960582"):
        {"name": "1inch V5 Aggregation Router", "category": "dex-aggregator"},
    (56, "0x111111125421ca6dc452d289314280a0f8842a65"):
        {"name": "1inch V6 Aggregation Router", "category": "dex-aggregator"},

    # ── Avalanche ──────────────────────────────────────────────────────
    (43114, "0x60ae616a2155ee3d9a68541ba4544862310933d4"):
        {"name": "Trader Joe V1 Router", "category": "dex-router"},
    (43114, "0x794a61358d6845594f94dc1db02a252b5b4814ad"):
        {"name": "Aave V3 Pool", "category": "lending"},
    (43114, "0x1111111254eeb25477b68fb85ed929f73a960582"):
        {"name": "1inch V5 Aggregation Router", "category": "dex-aggregator"},
}


def lookup_spender(chain_id: int, address: str):
    """Return metadata dict if known, else None. Address case-insensitive."""
    key = (chain_id, address.lower())
    return KNOWN_SPENDERS.get(key)


def is_bluechip_token(chain_id: int, address: str) -> str:
    """Return token symbol if address is a blue-chip, else empty string."""
    return BLUECHIP_TOKENS.get((chain_id, address.lower()), "")
