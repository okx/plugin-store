/// Chain configuration and contract addresses for the Dolomite plugin.

#[allow(dead_code)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub name: &'static str,
    pub rpc_url: &'static str,
    /// DolomiteMargin core contract
    pub dolomite_margin: &'static str,
}

pub const CHAIN_ARBITRUM: ChainConfig = ChainConfig {
    chain_id: 42161,
    name: "Arbitrum",
    rpc_url: "https://arbitrum-one-rpc.publicnode.com",
    dolomite_margin: "0x6Bd780E7fDf01D77e4d475c821f1e7AE05409072",
};

pub const CHAIN_MANTLE: ChainConfig = ChainConfig {
    chain_id: 5000,
    name: "Mantle",
    rpc_url: "https://rpc.mantle.xyz",
    // Dolomite is deployed on Mantle; address to be confirmed from docs
    dolomite_margin: "0x323a65F1780a9fA3B0c2ECa7EFc5D3e16FabE4BE",
};

pub const CHAIN_BERACHAIN: ChainConfig = ChainConfig {
    chain_id: 80094,
    name: "Berachain",
    rpc_url: "https://rpc.berachain.com",
    // Dolomite is deployed on Berachain; address to be confirmed from docs
    dolomite_margin: "0x407a859af7B798D8Da9B73Da5Bcff6f57df8b987",
};

pub fn get_chain_config(chain_id: u64) -> anyhow::Result<&'static ChainConfig> {
    match chain_id {
        42161 => Ok(&CHAIN_ARBITRUM),
        5000 => Ok(&CHAIN_MANTLE),
        80094 => Ok(&CHAIN_BERACHAIN),
        _ => anyhow::bail!(
            "Unsupported chain ID: {}. Supported: 42161 (Arbitrum), 5000 (Mantle), 80094 (Berachain)",
            chain_id
        ),
    }
}

/// Known token addresses per chain.
/// Returns (token_address, decimals, market_id_hint)
pub fn get_known_token(symbol: &str, chain_id: u64) -> Option<(&'static str, u8)> {
    match (chain_id, symbol.to_uppercase().as_str()) {
        // Arbitrum (42161)
        (42161, "USDC") => Some(("0xaf88d065e77c8cC2239327C5EDb3A432268e5831", 6)),
        (42161, "WETH") | (42161, "ETH") => Some(("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", 18)),
        (42161, "USDT") => Some(("0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9", 6)),
        (42161, "WBTC") => Some(("0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f", 8)),
        (42161, "ARB") => Some(("0x912CE59144191C1204E64559FE8253a0e49E6548", 18)),
        _ => None,
    }
}
