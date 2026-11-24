// config.rs — Chain configuration and contract addresses

/// RPC URL for a given chain ID
pub fn rpc_url(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "https://ethereum.publicnode.com",
        56 => "https://bsc-rpc.publicnode.com",
        137 => "https://polygon-bor-rpc.publicnode.com",
        8453 => "https://base-rpc.publicnode.com",
        42161 => "https://arbitrum-one-rpc.publicnode.com",
        _ => "https://ethereum.publicnode.com",
    }
}

/// CurveRouterNG address for a given chain ID
pub fn curve_router_ng(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "0x45312ea0eFf7E09C83CBE249fa1d7598c4C8cd4e",
        42161 => "0x2191718CD32d02B8E60BAdFFeA33E4B5DD9A0A0D",
        8453 => "0x4f37A9d177470499A2dD084621020b023fcffc1F",
        137 => "0x0DCDED3545D565bA3B19E683431381007245d983",
        56 => "0xA72C85C258A81761433B4e8da60505Fe3Dd551CC",
        _ => "",
    }
}

/// Chain name for Curve API
pub fn chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "ethereum",
        56 => "bsc",
        137 => "polygon",
        8453 => "base",
        42161 => "arbitrum",
        _ => "ethereum",
    }
}

/// Explorer URL prefix for a chain
pub fn explorer_url(chain_id: u64, tx_hash: &str) -> String {
    let base = match chain_id {
        1 => "https://etherscan.io/tx/",
        56 => "https://bscscan.com/tx/",
        137 => "https://polygonscan.com/tx/",
        8453 => "https://basescan.org/tx/",
        42161 => "https://arbiscan.io/tx/",
        _ => "https://etherscan.io/tx/",
    };
    format!("{}{}", base, tx_hash)
}

/// Resolve common token symbols to addresses on a given chain
pub fn resolve_token_address(symbol: &str, chain_id: u64) -> String {
    match (symbol.to_uppercase().as_str(), chain_id) {
        // Ethereum (1)
        ("ETH", 1) | ("WETH", 1) => "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string(),
        ("USDC", 1) => "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
        ("USDT", 1) => "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
        ("DAI", 1) => "0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string(),
        ("FRAX", 1) => "0x853d955aCEf822Db058eb8505911ED77F175b99e".to_string(),
        ("STETH", 1) | ("WSTETH", 1) => "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84".to_string(),
        // Arbitrum (42161)
        // Curve pools on Arbitrum use bridged USDC.e, not native USDC
        ("USDC", 42161) | ("USDC.E", 42161) => "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8".to_string(),
        ("USDT", 42161) => "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".to_string(),
        ("WETH", 42161) => "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1".to_string(),
        ("DAI", 42161) => "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1".to_string(),
        // Base (8453)
        ("USDC", 8453) => "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string(),
        ("WETH", 8453) => "0x4200000000000000000000000000000000000006".to_string(),
        ("ETH", 8453) => "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string(),
        // Polygon (137)
        ("USDC", 137) => "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(),
        ("USDT", 137) => "0xc2132D05D31c914a87C6611C10748AEb04B58e8F".to_string(),
        ("DAI", 137) => "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063".to_string(),
        ("WETH", 137) => "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619".to_string(),
        // BSC (56)
        ("USDT", 56) => "0x55d398326f99059fF775485246999027B3197955".to_string(),
        ("USDC", 56) => "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d".to_string(),
        ("BUSD", 56) => "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56".to_string(),
        _ => symbol.to_string(), // assume already a hex address
    }
}

/// Check whether an address is the native ETH sentinel
pub fn is_native_eth(addr: &str) -> bool {
    addr.eq_ignore_ascii_case("0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")
}
