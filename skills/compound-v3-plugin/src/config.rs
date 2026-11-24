/// Chain and market configuration for Compound V3

#[derive(Debug, Clone)]
pub struct MarketConfig {
    pub chain_id: u64,
    pub comet_proxy: &'static str,
    pub rewards_contract: &'static str,
    pub base_asset: &'static str,
    pub base_asset_decimals: u8,
    pub base_asset_symbol: &'static str,
    pub rpc_url: &'static str,
}

/// All known Compound V3 markets, indexed by (chain_id, market_symbol_lowercase)
pub fn get_market_config(chain_id: u64, market: &str) -> anyhow::Result<MarketConfig> {
    let m = market.to_lowercase();
    match (chain_id, m.as_str()) {
        (1, "usdc") => Ok(MarketConfig {
            chain_id: 1,
            comet_proxy: "0xc3d688B66703497DAA19211EEdff47f25384cdc3",
            rewards_contract: "0x1B0e765F6224C21223AeA2af16c1C46E38885a40",
            base_asset: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            base_asset_decimals: 6,
            base_asset_symbol: "USDC",
            rpc_url: "https://ethereum.publicnode.com",
        }),
        (8453, "usdc") => Ok(MarketConfig {
            chain_id: 8453,
            comet_proxy: "0xb125E6687d4313864e53df431d5425969c15Eb2F",
            rewards_contract: "0x123964802e6ABabBE1Bc9547D72Ef1B69B00A6b1",
            base_asset: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            base_asset_decimals: 6,
            base_asset_symbol: "USDC",
            rpc_url: "https://base-rpc.publicnode.com",
        }),
        (8453, "weth") => Ok(MarketConfig {
            chain_id: 8453,
            comet_proxy: "0x46e6b214b524310239732D51387075E0e70970bf",
            rewards_contract: "0x123964802e6ABabBE1Bc9547D72Ef1B69B00A6b1",
            base_asset: "0x4200000000000000000000000000000000000006",
            base_asset_decimals: 18,
            base_asset_symbol: "WETH",
            rpc_url: "https://base-rpc.publicnode.com",
        }),
        (42161, "usdc") => Ok(MarketConfig {
            chain_id: 42161,
            comet_proxy: "0x9c4ec768c28520B50860ea7a15bd7213a9fF58bf",
            rewards_contract: "0x88730d254A2f7e6AC8388c3198aFd694bA9f7fae",
            base_asset: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
            base_asset_decimals: 6,
            base_asset_symbol: "USDC",
            rpc_url: "https://arbitrum-one-rpc.publicnode.com",
        }),
        (42161, "weth") => Ok(MarketConfig {
            chain_id: 42161,
            comet_proxy: "0x6f7D514bbD4aFf3BcD1140B7344b32f063dEe486",
            rewards_contract: "0x88730d254A2f7e6AC8388c3198aFd694bA9f7fae",
            base_asset: "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
            base_asset_decimals: 18,
            base_asset_symbol: "WETH",
            rpc_url: "https://arbitrum-one-rpc.publicnode.com",
        }),
        (42161, "usdc.e") => Ok(MarketConfig {
            chain_id: 42161,
            comet_proxy: "0xA5EDBDD9646f8dFF606d7448e414884C7d905dCA",
            rewards_contract: "0x88730d254A2f7e6AC8388c3198aFd694bA9f7fae",
            base_asset: "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8",
            base_asset_decimals: 6,
            base_asset_symbol: "USDC.e",
            rpc_url: "https://arbitrum-one-rpc.publicnode.com",
        }),
        (137, "usdc") => Ok(MarketConfig {
            chain_id: 137,
            comet_proxy: "0xF25212E676D1F7F89Cd72fFEe66158f541246445",
            rewards_contract: "0x45939657d1CA34A8FA39A924B71D28Fe8431e581",
            base_asset: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
            base_asset_decimals: 6,
            base_asset_symbol: "USDC",
            rpc_url: "https://polygon-bor-rpc.publicnode.com",
        }),
        _ => anyhow::bail!(
            "Unsupported chain_id={} market={}. Supported markets: \
             usdc (chains 1/8453/42161/137), \
             weth (chains 8453/42161), \
             usdc.e (chain 42161 only)",
            chain_id,
            market
        ),
    }
}

/// Default RPC URL for a chain (used outside market context)
pub fn default_rpc_url(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "https://ethereum.publicnode.com",
        8453 => "https://base-rpc.publicnode.com",
        42161 => "https://arbitrum-one-rpc.publicnode.com",
        137 => "https://polygon-bor-rpc.publicnode.com",
        _ => "https://base-rpc.publicnode.com",
    }
}
