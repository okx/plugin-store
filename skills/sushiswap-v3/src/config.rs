/// Resolve a token symbol or hex address to a checksummed hex address.
/// If the input already starts with 0x, return as-is.
pub fn resolve_token_address(symbol: &str, chain_id: u64) -> String {
    match (symbol.to_uppercase().as_str(), chain_id) {
        // Ethereum (1)
        ("WETH", 1) | ("ETH", 1) => "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        ("USDC", 1) => "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        ("USDT", 1) => "0xdAC17F958D2ee523a2206206994597C13D831ec7",
        ("SUSHI", 1) => "0x6B3595068778DD592e39A122f4f5a5cF09C90fE2",
        // Base (8453)
        ("WETH", 8453) | ("ETH", 8453) => "0x4200000000000000000000000000000000000006",
        ("USDC", 8453) => "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        ("CBBTC", 8453) => "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf",
        ("DAI", 8453) => "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb",
        // Arbitrum (42161)
        ("WETH", 42161) | ("ETH", 42161) => "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
        ("USDC", 42161) => "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
        ("USDT", 42161) => "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9",
        // BSC (56)
        ("WBNB", 56) | ("BNB", 56) => "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c",
        ("USDT", 56) => "0x55d398326f99059fF775485246999027B3197955",
        ("USDC", 56) => "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d",
        // Polygon (137)
        ("WMATIC", 137) | ("MATIC", 137) => "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270",
        ("USDC", 137) => "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
        ("USDT", 137) => "0xc2132D05D31c914a87C6611C10748AEb04B58e8F",
        // Optimism (10)
        ("WETH", 10) | ("ETH", 10) => "0x4200000000000000000000000000000000000006",
        ("USDC", 10) => "0x7F5c764cBc14f9669B88837ca1490cCa17c31607",
        ("USDT", 10) => "0x94b008aA00579c1307B0EF2c499aD98a8ce58e58",
        // Avalanche (43114)
        ("WAVAX", 43114) | ("AVAX", 43114) => "0xB31f66AA3C1e785363F0875A1B74E27b85FD66c7",
        ("USDC", 43114) => "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E",
        _ => symbol, // assume already a hex address
    }
    .to_string()
}

/// Return the JSON-RPC endpoint for a given chain.
pub fn rpc_url(chain_id: u64) -> anyhow::Result<String> {
    match chain_id {
        1 => Ok("https://eth-rpc.publicnode.com".to_string()),
        8453 => Ok("https://base-rpc.publicnode.com".to_string()),
        42161 => Ok("https://arbitrum-one-rpc.publicnode.com".to_string()),
        56 => Ok("https://bsc-rpc.publicnode.com".to_string()),
        137 => Ok("https://polygon-bor-rpc.publicnode.com".to_string()),
        10 => Ok("https://optimism-rpc.publicnode.com".to_string()),
        43114 => Ok("https://avalanche-c-chain-rpc.publicnode.com".to_string()),
        _ => anyhow::bail!(
            "Unsupported chain_id: {}. Supported: 1 (Ethereum), 8453 (Base), 42161 (Arbitrum), 56 (BSC), 137 (Polygon), 10 (Optimism), 43114 (Avalanche)",
            chain_id
        ),
    }
}

/// SushiSwap V3 SwapRouter address — identical across all chains (deterministic CREATE2).
pub fn swap_router(_chain_id: u64) -> &'static str {
    "0xFB7eF66a7e61224DD6FcD0D7d9C3be5C8B049b9f"
}

/// SushiSwap V3 NonfungiblePositionManager address — identical across all chains.
pub fn nfpm_address(_chain_id: u64) -> &'static str {
    "0x80C7DD17B01855a6D2347444a0FCC36136a314de"
}

/// SushiSwap V3 QuoterV2 address — identical across all chains.
pub fn quoter_v2_address(_chain_id: u64) -> &'static str {
    "0xb1E835Dc2785b52265711e17fCCb0fd018226a6e"
}

/// SushiSwap V3 Factory address — identical across all chains.
pub fn factory_address(_chain_id: u64) -> &'static str {
    "0xc35DADB65012eC5796536bD9864eD8773aBc74C4"
}

/// Encode a signed int24 tick value as a 32-byte ABI hex string.
pub fn encode_tick(tick: i32) -> String {
    if tick >= 0 {
        format!("{:0>64x}", tick as u64)
    } else {
        // Two's complement sign extension for negative values
        format!(
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffff{:08x}",
            tick as u32
        )
    }
}

/// Decode an ABI int256 hex word back to i32 (tick fits in lower 4 bytes).
#[allow(dead_code)]
pub fn decode_tick(hex: &str) -> i32 {
    let clean = hex.trim_start_matches("0x");
    let last8 = &clean[clean.len().saturating_sub(8)..];
    u32::from_str_radix(last8, 16).unwrap_or(0) as i32
}

/// Build ERC-20 approve(address,uint256) calldata.
/// Selector: 0x095ea7b3
pub fn build_approve_calldata(spender: &str, amount: u128) -> String {
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:0>64x}", amount);
    format!("0x095ea7b3{}{}", spender_padded, amount_hex)
}

/// Pad an address to 32 bytes (no 0x prefix in the padded portion).
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u128 value to 32 bytes hex.
pub fn pad_u256(val: u128) -> String {
    format!("{:0>64x}", val)
}

/// UINT128_MAX as 32-byte padded hex (for collect maxAmount params).
pub fn uint128_max_padded() -> String {
    format!("{:0>64x}", u128::MAX)
}

/// Return current unix timestamp in seconds.
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
