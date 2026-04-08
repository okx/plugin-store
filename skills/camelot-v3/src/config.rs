/// Resolve token symbol or hex address to a hex address.
pub fn resolve_token_address(symbol: &str, chain_id: u64) -> String {
    if symbol.starts_with("0x") || symbol.starts_with("0X") {
        return symbol.to_string();
    }
    match (symbol.to_uppercase().as_str(), chain_id) {
        // Arbitrum (42161)
        ("WETH", 42161) | ("ETH", 42161) => "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
        ("USDT", 42161) | ("USD₮0", 42161) => "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9",
        ("USDC", 42161) => "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
        ("USDC.E", 42161) => "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8",
        ("ARB", 42161) => "0x912CE59144191C1204E64559FE8253a0e49E6548",
        ("GRAIL", 42161) => "0x3d9907F9a368ad0a51Be60f7Da3b97cf940982D8",
        _ => symbol,
    }
    .to_string()
}

pub fn rpc_url(chain_id: u64) -> anyhow::Result<String> {
    match chain_id {
        42161 => Ok("https://arbitrum-one-rpc.publicnode.com".to_string()),
        _ => anyhow::bail!("Unsupported chain_id: {}. Supported: 42161 (Arbitrum)", chain_id),
    }
}

/// Camelot V3 = Algebra V1 fork — one SwapRouter per chain
pub fn swap_router(chain_id: u64) -> anyhow::Result<&'static str> {
    match chain_id {
        42161 => Ok("0x1F721E2E82F6676FCE4eA07A5958cF098D339e18"),
        _ => anyhow::bail!("Unsupported chain_id: {}", chain_id),
    }
}

pub fn quoter(chain_id: u64) -> anyhow::Result<&'static str> {
    match chain_id {
        42161 => Ok("0x0Fc73040b26E9bC8514fA028D998E73A254Fa76E"),
        _ => anyhow::bail!("Unsupported chain_id: {}", chain_id),
    }
}

pub fn factory(chain_id: u64) -> anyhow::Result<&'static str> {
    match chain_id {
        42161 => Ok("0x1a3c9B1d2F0529D97f2afC5136Cc23e58f1FD35B"),
        _ => anyhow::bail!("Unsupported chain_id: {}", chain_id),
    }
}

pub fn nfpm(chain_id: u64) -> anyhow::Result<&'static str> {
    match chain_id {
        42161 => Ok("0x00c7f3082833e796A5b3e4Bd59f6642FF44DCD15"),
        _ => anyhow::bail!("Unsupported chain_id: {}", chain_id),
    }
}

/// Pad an address to 32 bytes ABI-encoded (no 0x prefix in output).
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u256 value to 32 bytes hex.
pub fn pad_u256(val: u128) -> String {
    format!("{:0>64x}", val)
}

/// Encode an int24 tick as a 32-byte ABI hex string (sign-extended).
pub fn encode_tick(tick: i32) -> String {
    if tick >= 0 {
        format!("{:0>64x}", tick as u64)
    } else {
        // sign-extend negative: fill upper bytes with ff
        format!(
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffff{:08x}",
            tick as u32
        )
    }
}

/// Current unix timestamp in seconds.
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
