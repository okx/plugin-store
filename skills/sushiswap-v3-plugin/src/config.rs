// SushiSwap V3 — multi-chain concentrated liquidity (UniV3 fork)
//
// Factory and NFPM addresses from https://github.com/sushiswap/sushiswap deployments.
// Swaps use the Sushi Swap API (api.sushi.com/swap/v7) which returns the correct
// router contract and calldata, avoiding hardcoded SwapRouter addresses that differ per chain.

pub struct ChainConfig {
    pub chain_id: u64,
    pub name: &'static str,
    pub factory: &'static str,
    pub nfpm: &'static str,
    pub rpc_url: &'static str,
    pub explorer: &'static str,
}

pub fn chain_config(chain_id: u64) -> anyhow::Result<&'static ChainConfig> {
    CHAIN_CONFIGS
        .iter()
        .find(|c| c.chain_id == chain_id)
        .ok_or_else(|| anyhow::anyhow!(
            "Unsupported chain: {}. Supported chains: 1 (Ethereum), 42161 (Arbitrum), 8453 (Base), 137 (Polygon), 10 (Optimism)",
            chain_id
        ))
}

/// Return the RPC URL for a chain, checking SUSHI_RPC_<CHAIN_ID> env var first.
/// This allows users to override the default public endpoint when rate-limited.
/// Example: `export SUSHI_RPC_137=https://polygon-mainnet.g.alchemy.com/v2/YOUR_KEY`
pub fn rpc_url(chain_id: u64) -> anyhow::Result<String> {
    let env_key = format!("SUSHI_RPC_{}", chain_id);
    if let Ok(url) = std::env::var(&env_key) {
        if !url.is_empty() {
            return Ok(url);
        }
    }
    Ok(chain_config(chain_id)?.rpc_url.to_string())
}

static CHAIN_CONFIGS: &[ChainConfig] = &[
    ChainConfig {
        chain_id: 1,
        name:      "Ethereum Mainnet",
        factory:   "0xbACEB8eC6b9355Dfc0269C18bac9d6E2Bdc29C4F",
        nfpm:      "0x2214A42d8e2A1d20635c2cb0664422c528B6A432",
        rpc_url:   "https://eth.llamarpc.com",
        explorer:  "https://etherscan.io/tx",
    },
    ChainConfig {
        chain_id: 42161,
        name:      "Arbitrum",
        factory:   "0x1af415a1EbA07a4986a52B6f2e7dE7003D82231e",
        nfpm:      "0xF0cBCe1942A68BEB3d1b73F0DD86C8DCc363eF49",
        rpc_url:   "https://arb1.arbitrum.io/rpc",
        explorer:  "https://arbiscan.io/tx",
    },
    ChainConfig {
        chain_id: 8453,
        name:      "Base",
        factory:   "0xc35DADB65012eC5796536bD9864eD8773aBc74C4",
        nfpm:      "0x80C7DD17B01855a6D2347444a0FCC36136a314de",
        rpc_url:   "https://mainnet.base.org",
        explorer:  "https://basescan.org/tx",
    },
    ChainConfig {
        chain_id: 137,
        name:      "Polygon",
        factory:   "0x917933899c6a5F8E37F31E19f92CdBFF7e8FF0e2",
        nfpm:      "0xb7402ee99F0A008e461098AC3A27F4957Df89a40",
        rpc_url:   "https://polygon-bor-rpc.publicnode.com",
        explorer:  "https://polygonscan.com/tx",
    },
    ChainConfig {
        chain_id: 10,
        name:      "Optimism",
        factory:   "0x9c6522117e2ed1fE5bdb72bb0eD5E3f2bde7dbE0",
        nfpm:      "0x1af415a1EbA07a4986a52B6f2e7dE7003D82231e",
        rpc_url:   "https://mainnet.optimism.io",
        explorer:  "https://optimistic.etherscan.io/tx",
    },
];

/// Common UniV3 fee tiers (basis points).
/// Fee → tick spacing: 100→1, 500→10, 3000→60, 10000→200
pub fn common_fee_tiers() -> &'static [u32] {
    &[100, 500, 3000, 10000]
}

/// Tick spacing for a given fee tier.
pub fn fee_to_tick_spacing(fee: u32) -> i32 {
    match fee {
        100   => 1,
        500   => 10,
        3000  => 60,
        10000 => 200,
        _     => 60,
    }
}

// ── Token helpers per chain ───────────────────────────────────────────────────

pub fn resolve_token(symbol: &str, chain_id: u64) -> String {
    if symbol.starts_with("0x") || symbol.starts_with("0X") {
        return symbol.to_lowercase();
    }
    let s = symbol.to_uppercase();
    let addr = match chain_id {
        1     => eth_token(&s),
        42161 => arb_token(&s),
        8453  => base_token(&s),
        137   => polygon_token(&s),
        10    => optimism_token(&s),
        _     => None,
    };
    addr.unwrap_or(symbol).to_string()
}

pub fn token_symbol(addr: &str, chain_id: u64) -> &'static str {
    let a = addr.to_lowercase();
    match chain_id {
        1     => eth_symbol(&a),
        42161 => arb_symbol(&a),
        8453  => base_symbol(&a),
        137   => poly_symbol(&a),
        10    => opt_symbol(&a),
        _     => "UNKNOWN",
    }
}

fn eth_token(s: &str) -> Option<&'static str> {
    match s {
        "ETH" | "WETH" => Some("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"),
        "USDC"         => Some("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"),
        "USDT"         => Some("0xdac17f958d2ee523a2206206994597c13d831ec7"),
        "DAI"          => Some("0x6b175474e89094c44da98b954eedeac495271d0f"),
        "WBTC"         => Some("0x2260fac5e5542a773aa44fbcfedf7c193bc2c599"),
        "LINK"         => Some("0x514910771af9ca656af840dff83e8264ecf986ca"),
        "UNI"          => Some("0x1f9840a85d5af5bf1d1762f925bdaddc4201f984"),
        "SUSHI"        => Some("0x6b3595068778dd592e39a122f4f5a5cf09c90fe2"),
        _              => None,
    }
}

fn eth_symbol(a: &str) -> &'static str {
    match a {
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => "WETH",
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" => "USDC",
        "0xdac17f958d2ee523a2206206994597c13d831ec7" => "USDT",
        "0x6b175474e89094c44da98b954eedeac495271d0f" => "DAI",
        "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599" => "WBTC",
        "0x514910771af9ca656af840dff83e8264ecf986ca" => "LINK",
        "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984" => "UNI",
        "0x6b3595068778dd592e39a122f4f5a5cf09c90fe2" => "SUSHI",
        _ => "UNKNOWN",
    }
}

fn arb_token(s: &str) -> Option<&'static str> {
    match s {
        "ETH" | "WETH" => Some("0x82af49447d8a07e3bd95bd0d56f35241523fbab1"),
        "USDC"         => Some("0xaf88d065e77c8cc2239327c5edb3a432268e5831"),
        "USDC.E"       => Some("0xff970a61a04b1ca14834a43f5de4533ebddb5cc8"),
        "USDT"         => Some("0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9"),
        "DAI"          => Some("0xda10009cbd5d07dd0cecc66161fc93d7c9000da1"),
        "WBTC"         => Some("0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f"),
        "ARB"          => Some("0x912ce59144191c1204e64559fe8253a0e49e6548"),
        "SUSHI"        => Some("0xd4d42f0b6def4ce0383636770ef773390d85c61a"),
        _              => None,
    }
}

fn arb_symbol(a: &str) -> &'static str {
    match a {
        "0x82af49447d8a07e3bd95bd0d56f35241523fbab1" => "WETH",
        "0xaf88d065e77c8cc2239327c5edb3a432268e5831" => "USDC",
        "0xff970a61a04b1ca14834a43f5de4533ebddb5cc8" => "USDC.e",
        "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9" => "USDT",
        "0xda10009cbd5d07dd0cecc66161fc93d7c9000da1" => "DAI",
        "0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f" => "WBTC",
        "0x912ce59144191c1204e64559fe8253a0e49e6548" => "ARB",
        "0xd4d42f0b6def4ce0383636770ef773390d85c61a" => "SUSHI",
        _ => "UNKNOWN",
    }
}

fn base_token(s: &str) -> Option<&'static str> {
    match s {
        "ETH" | "WETH" => Some("0x4200000000000000000000000000000000000006"),
        "USDC"         => Some("0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"),
        "USDT"         => Some("0xfde4c96c8593536e31f229ea8f37b2ada2699bb2"),
        "DAI"          => Some("0x50c5725949a6f0c72e6c4a641f24049a917db0cb"),
        "CBBTC"        => Some("0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf"),
        _              => None,
    }
}

fn base_symbol(a: &str) -> &'static str {
    match a {
        "0x4200000000000000000000000000000000000006" => "WETH",
        "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" => "USDC",
        "0xfde4c96c8593536e31f229ea8f37b2ada2699bb2" => "USDT",
        "0x50c5725949a6f0c72e6c4a641f24049a917db0cb" => "DAI",
        "0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf" => "cbBTC",
        _ => "UNKNOWN",
    }
}

fn polygon_token(s: &str) -> Option<&'static str> {
    match s {
        "ETH" | "WETH"     => Some("0x7ceb23fd6bc0add59e62ac25578270cff1b9f619"),
        "MATIC" | "WMATIC" => Some("0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270"),
        "USDC"             => Some("0x3c499c542cef5e3811e1192ce70d8cc03d5c3359"),
        "USDC.E"           => Some("0x2791bca1f2de4661ed88a30c99a7a9449aa84174"),
        "USDT"             => Some("0xc2132d05d31c914a87c6611c10748aeb04b58e8f"),
        "DAI"              => Some("0x8f3cf7ad23cd3cadbd9735aff958023239c6a063"),
        "WBTC"             => Some("0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6"),
        _                  => None,
    }
}

fn poly_symbol(a: &str) -> &'static str {
    match a {
        "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619" => "WETH",
        "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270" => "WMATIC",
        "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359" => "USDC",
        "0x2791bca1f2de4661ed88a30c99a7a9449aa84174" => "USDC.e",
        "0xc2132d05d31c914a87c6611c10748aeb04b58e8f" => "USDT",
        "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063" => "DAI",
        "0x1bfd67037b42cf73acf2047067bd4f2c47d9bfd6" => "WBTC",
        _ => "UNKNOWN",
    }
}

fn optimism_token(s: &str) -> Option<&'static str> {
    match s {
        "ETH" | "WETH" => Some("0x4200000000000000000000000000000000000006"),
        "USDC"         => Some("0x0b2c639c533813f4aa9d7837caf62653d097ff85"),
        "USDC.E"       => Some("0x7f5c764cbc14f9669b88837ca1490cca17c31607"),
        "USDT"         => Some("0x94b008aa00579c1307b0ef2c499ad98a8ce58e58"),
        "DAI"          => Some("0xda10009cbd5d07dd0cecc66161fc93d7c9000da1"),
        "WBTC"         => Some("0x68f180fcce6836688e9084f035309e29bf0a2095"),
        "OP"           => Some("0x4200000000000000000000000000000000000042"),
        _              => None,
    }
}

fn opt_symbol(a: &str) -> &'static str {
    match a {
        "0x4200000000000000000000000000000000000006" => "WETH",
        "0x0b2c639c533813f4aa9d7837caf62653d097ff85" => "USDC",
        "0x7f5c764cbc14f9669b88837ca1490cca17c31607" => "USDC.e",
        "0x94b008aa00579c1307b0ef2c499ad98a8ce58e58" => "USDT",
        "0xda10009cbd5d07dd0cecc66161fc93d7c9000da1" => "DAI",
        "0x68f180fcce6836688e9084f035309e29bf0a2095" => "WBTC",
        "0x4200000000000000000000000000000000000042" => "OP",
        _ => "UNKNOWN",
    }
}

// ── ABI helpers ───────────────────────────────────────────────────────────────

pub fn pad_address(addr: &str) -> String {
    format!("{:0>64}", addr.trim_start_matches("0x").to_lowercase())
}

pub fn pad_u256(val: u128) -> String {
    format!("{:0>64x}", val)
}

pub fn build_approve_calldata(spender: &str, amount: u128) -> String {
    format!("0x095ea7b3{}{}", pad_address(spender), pad_u256(amount))
}

pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
