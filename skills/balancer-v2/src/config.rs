/// Balancer V2 configuration: contract addresses and RPC endpoints

pub const VAULT_ADDRESS: &str = "0xBA12222222228d8Ba445958a75a0704d566BF2C8"; // same on all chains

/// BalancerQueries contract address (same on Arbitrum and Ethereum)
pub const BALANCER_QUERIES_ADDRESS: &str = "0xE39B5e3B6D74016b2F6A9673D7d7493B6DF549d5";

pub fn rpc_url(chain_id: u64) -> &'static str {
    match chain_id {
        42161 => "https://arbitrum-one-rpc.publicnode.com",
        1 => "https://ethereum.publicnode.com",
        _ => "https://arbitrum-one-rpc.publicnode.com",
    }
}

/// Balancer API V3 GraphQL endpoint (pool discovery)
pub const BALANCER_API_V3: &str = "https://api-v3.balancer.fi/graphql";

pub fn resolve_token_address(symbol: &str, chain_id: u64) -> String {
    match (symbol.to_uppercase().as_str(), chain_id) {
        // Arbitrum tokens
        ("WETH", 42161) => "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1".to_string(),
        ("USDC", 42161) | ("USDC.E", 42161) => "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8".to_string(),
        ("USDT", 42161) => "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".to_string(),
        ("WBTC", 42161) => "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0F".to_string(),
        ("DAI", 42161) => "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1".to_string(),
        // Ethereum tokens
        ("WETH", 1) => "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
        ("USDC", 1) => "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
        ("USDT", 1) => "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
        ("WBTC", 1) => "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".to_string(),
        ("DAI", 1) => "0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string(),
        _ => symbol.to_string(), // assume already a hex address
    }
}

/// Known pools per chain for positions lookup
pub fn known_pools(chain_id: u64) -> Vec<(&'static str, &'static str)> {
    // (pool_id, pool_address)
    match chain_id {
        42161 => vec![
            (
                "0x64541216bafffeec8ea535bb71fbc927831d0595000100000000000000000002",
                "0x64541216bafffeec8ea535bb71fbc927831d0595",
            ),
            (
                "0x1533a3278f3f9141d5f820a184ea4b017fce2382000000000000000000000016",
                "0x1533a3278f3f9141d5f820a184ea4b017fce2382",
            ),
            (
                "0x36bf227d6bac96e2ab1ebb5492ecec69c691943f000200000000000000000316",
                "0x36bf227d6bac96e2ab1ebb5492ecec69c691943f",
            ),
        ],
        _ => vec![],
    }
}
