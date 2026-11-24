/// Static config for dolomite-plugin: chain whitelist + Dolomite contract addresses
/// + commonly-used market IDs.
///
/// v0.1.0 scope: **Arbitrum One only**. Berachain / Polygon zkEVM / X Layer / Mantle
/// are valid Dolomite deployments but onchainos doesn't have wallet support for them
/// yet — adding requires both a config entry here AND onchainos coverage.
///
/// Adding a chain requires extending plugin.yaml `api_calls` to whitelist its RPC.

#[derive(Debug, Clone)]
pub struct ChainInfo {
    pub id: u64,
    pub key: &'static str,
    pub name: &'static str,
    pub rpc: &'static str,
    pub native_symbol: &'static str,
    /// DolomiteMargin core contract (immutable; address of the protocol entry point).
    pub dolomite_margin: &'static str,
    /// DepositWithdrawalProxy — the contract regular suppliers / withdrawers call.
    pub deposit_withdrawal_proxy: &'static str,
    /// BorrowPositionProxyV2 — opens / manages / repays isolated borrow positions.
    pub borrow_position_proxy: &'static str,
}

/// 1 supported chain in v0.1.0.
///
/// Address sources:
///   - https://docs.dolomite.io/smart-contract-addresses/core-immutable.md (DolomiteMargin)
///   - https://docs.dolomite.io/smart-contract-addresses/core-proxies.md (proxies)
pub const SUPPORTED_CHAINS: &[ChainInfo] = &[
    ChainInfo {
        id: 42161,
        key: "ARB",
        name: "Arbitrum",
        rpc: "https://arbitrum-one-rpc.publicnode.com",
        native_symbol: "ETH",
        dolomite_margin:           "0x6Bd780E7fDf01D77e4d475c821f1e7AE05409072",
        deposit_withdrawal_proxy:  "0xAdB9D68c613df4AA363B42161E1282117C7B9594",
        borrow_position_proxy:     "0x38E49A617305101216eC6306e3a18065D14Bf3a7",
    },
];

pub fn chain_by_id(id: u64) -> Option<&'static ChainInfo> {
    SUPPORTED_CHAINS.iter().find(|c| c.id == id)
}

/// Look up by chain id OR canonical key (case-insensitive).
pub fn parse_chain(s: &str) -> Option<&'static ChainInfo> {
    if let Ok(id) = s.parse::<u64>() {
        return chain_by_id(id);
    }
    let upper = s.to_uppercase();
    let canon = match upper.as_str() {
        "ARBITRUM" | "ARB" | "ARBITRUM-ONE" => "ARB",
        other => other,
    };
    SUPPORTED_CHAINS.iter().find(|c| c.key.eq_ignore_ascii_case(canon))
}

pub fn supported_chains_help() -> String {
    SUPPORTED_CHAINS
        .iter()
        .map(|c| format!("{} ({}, id={})", c.key, c.name, c.id))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Well-known Dolomite market IDs on Arbitrum.
/// Source: per-market on-chain `getNumMarkets()` enumeration + token symbol lookup.
/// We hard-code the most common stable + ETH-related markets so users can pass
/// `--token USDC` instead of having to know market_id=17.
///
/// To enumerate fully at runtime, the `markets` command iterates 0..getNumMarkets()
/// and resolves each via `getMarketTokenAddress`. The map below is for shorthand only.
pub const ARB_KNOWN_MARKETS: &[(u64, &str, &str)] = &[
    // (market_id, symbol, token_address) — IDs verified on-chain via
    // DolomiteMargin.getMarketTokenAddress(uint256) on Arbitrum (chainId 42161).
    (17, "USDC",  "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"),  // native USDC (Circle)
    (5,  "USDT",  "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9"),
    (0,  "WETH",  "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"),
    (1,  "DAI",   "0xDA10009cBd5D07dD0CeCc66161FC93D7c9000da1"),
    (4,  "WBTC",  "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f"),
    (7,  "ARB",   "0x912CE59144191C1204E64559FE8253a0e49E6548"),
    (2,  "USDC.e","0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8"),  // bridged USDC
    (3,  "LINK",  "0xf97f4df75117a78c1A5a0DBb814Af92458539FB4"),
];

/// Resolve a user-friendly token symbol or address to a market_id.
/// Returns None if unknown — caller should suggest `markets` command to enumerate.
pub fn resolve_market_id(token: &str) -> Option<(u64, &'static str, &'static str)> {
    let trimmed = token.trim();
    let upper = trimmed.to_uppercase();
    for (id, sym, addr) in ARB_KNOWN_MARKETS {
        if upper == *sym || trimmed.eq_ignore_ascii_case(addr) {
            return Some((*id, *sym, *addr));
        }
    }
    None
}

/// Token decimals lookup. ERC-20 tokens carry their own `decimals()` but for
/// known stable+canonical tokens we hardcode to avoid an extra RPC call.
pub fn token_decimals(symbol: &str) -> Option<u32> {
    match symbol.to_uppercase().as_str() {
        "USDC" | "USDC.E" | "USDT" => Some(6),
        "WBTC" => Some(8),
        "WETH" | "DAI" | "ARB" | "LINK" => Some(18),
        _ => None,
    }
}
