/// Chain configuration and contract addresses for PancakeSwap V3.

pub struct ChainConfig {
    pub chain_id: u64,
    pub rpc_url: &'static str,
    pub smart_router: &'static str,
    pub factory: &'static str,
    pub npm: &'static str,   // NonfungiblePositionManager
    pub quoter_v2: &'static str,
    pub subgraph_url: &'static str,
}

pub const BSC: ChainConfig = ChainConfig {
    chain_id: 56,
    rpc_url: "https://bsc-rpc.publicnode.com",
    smart_router: "0x13f4EA83D0bd40E75C8222255bc855a974568Dd4",
    factory: "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865",
    npm: "0x46A15B0b27311cedF172AB29E4f4766fbE7F4364",
    quoter_v2: "0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997",
    subgraph_url: "https://api.thegraph.com/subgraphs/name/pancakeswap/exchange-v3-bsc",
};

pub const BASE: ChainConfig = ChainConfig {
    chain_id: 8453,
    rpc_url: "https://base-rpc.publicnode.com",
    smart_router: "0x678Aa4bF4E210cf2166753e054d5b7c31cc7fa86",
    factory: "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865",
    npm: "0x46A15B0b27311cedF172AB29E4f4766fbE7F4364",
    quoter_v2: "0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997",
    subgraph_url: "https://api.studio.thegraph.com/query/45376/exchange-v3-base/version/latest",
};

pub const ARBITRUM: ChainConfig = ChainConfig {
    chain_id: 42161,
    rpc_url: "https://arbitrum-one-rpc.publicnode.com",
    smart_router: "0x5E325eDA8064b456f4781070C0738d849c824258",
    factory: "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865",
    npm: "0x46A15B0b27311cedF172AB29E4f4766fbE7F4364",
    quoter_v2: "0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997",
    subgraph_url: "https://api.thegraph.com/subgraphs/name/pancakeswap/exchange-v3-arbitrum",
};

pub fn get_chain_config(chain_id: u64) -> anyhow::Result<&'static ChainConfig> {
    match chain_id {
        56 => Ok(&BSC),
        8453 => Ok(&BASE),
        42161 => Ok(&ARBITRUM),
        _ => anyhow::bail!("Unsupported chain ID: {}. Supported: 56 (BSC), 8453 (Base), 42161 (Arbitrum)", chain_id),
    }
}

/// tickSpacing for each fee tier.
pub fn tick_spacing(fee: u32) -> anyhow::Result<i32> {
    match fee {
        100 => Ok(1),
        500 => Ok(10),
        2500 => Ok(50),
        10000 => Ok(200),
        _ => anyhow::bail!("Unknown fee tier: {}. Valid: 100, 500, 2500, 10000", fee),
    }
}

/// Resolve a token symbol to its canonical address for the given chain.
/// If the input is already a 0x... address, it is returned as-is.
pub fn resolve_token_address(symbol_or_addr: &str, chain_id: u64) -> anyhow::Result<String> {
    // Already an address
    if symbol_or_addr.starts_with("0x") || symbol_or_addr.starts_with("0X") {
        return Ok(symbol_or_addr.to_string());
    }
    let sym = symbol_or_addr.to_uppercase();
    let addr = match (chain_id, sym.as_str()) {
        // BSC (56)
        (56, "WBNB") | (56, "BNB") => "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c",
        (56, "USDT") => "0x55d398326f99059fF775485246999027B3197955",
        (56, "USDC") => "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d",
        (56, "BUSD") => "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56",
        (56, "ETH") | (56, "WETH") => "0x2170Ed0880ac9A755fd29B2688956BD959F933F8",
        (56, "CAKE") => "0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82",
        // Base (8453)
        (8453, "WETH") | (8453, "ETH") => "0x4200000000000000000000000000000000000006",
        (8453, "USDC") => "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        (8453, "USDT") => "0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2",
        (8453, "DAI") => "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb",
        (8453, "CBETH") => "0x2Ae3F1Ec7F1F5012CFEab0185bfc7aa3cf0DEc22",
        // Arbitrum (42161)
        (42161, "WETH") | (42161, "ETH") => "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
        (42161, "USDC") => "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
        (42161, "USDC.E") | (42161, "USDCE") => "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8",
        (42161, "USDT") => "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9",
        (42161, "ARB") => "0x912CE59144191C1204E64559FE8253a0e49E6548",
        (42161, "WBTC") => "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f",
        _ => anyhow::bail!(
            "Unknown token symbol '{}' on chain {}. Please use a full 0x address.",
            symbol_or_addr, chain_id
        ),
    };
    Ok(addr.to_string())
}

/// Convert human-readable token amount to minimal units (wei/atomic).
/// Uses string-based arithmetic to avoid f64 precision loss for large amounts.
pub fn human_to_minimal(amount: &str, decimals: u8) -> anyhow::Result<u128> {
    let amount = amount.trim();
    let (int_str, frac_str) = match amount.find('.') {
        Some(pos) => (&amount[..pos], &amount[pos + 1..]),
        None => (amount, ""),
    };
    if int_str.is_empty() && frac_str.is_empty() {
        anyhow::bail!("Invalid amount: {}", amount);
    }
    let int_val: u128 = if int_str.is_empty() {
        0
    } else {
        int_str.parse().map_err(|_| anyhow::anyhow!("Invalid amount: {}", amount))?
    };
    let d = decimals as usize;
    // Pad fractional part to exactly `decimals` digits (truncate if longer)
    let frac_padded = format!("{:0<width$}", frac_str, width = d);
    let frac_val: u128 = if d == 0 {
        0
    } else {
        frac_padded[..d].parse().map_err(|_| anyhow::anyhow!("Invalid fractional part: {}", amount))?
    };
    let multiplier = 10u128.checked_pow(decimals as u32)
        .ok_or_else(|| anyhow::anyhow!("Decimals too large: {}", decimals))?;
    int_val
        .checked_mul(multiplier)
        .and_then(|v| v.checked_add(frac_val))
        .ok_or_else(|| anyhow::anyhow!("Amount overflow: {}", amount))
}
