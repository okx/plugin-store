// Aerodrome Slipstream (CLMM) — Base mainnet (chain 8453)

pub const CHAIN_ID: u64 = 8453;

/// Aerodrome Slipstream CLFactory
pub fn cl_factory() -> &'static str {
    "0x5e7bb104d84c7cb9b682aac2f3d509f5f406809a"
}

/// Aerodrome Slipstream SwapRouter
pub fn swap_router() -> &'static str {
    "0xBE6D8f0d05cC4be24d5167a3eF062215bE6D18a5"
}

/// Aerodrome Slipstream Quoter
pub fn quoter() -> &'static str {
    "0x254cF9E1E6e233aa1Ac962cB9B05b2cfeaAE15b0"
}

/// Aerodrome Slipstream NonfungiblePositionManager
pub fn nfpm() -> &'static str {
    "0x827922686190790b37229fd06084350e74485b72"
}

/// Aerodrome Voter (for future gauge/staking commands)
#[allow(dead_code)]
pub fn voter() -> &'static str {
    "0x16613524e02ad97eDfeF371bC883F2F5d6C480A5"
}

/// Primary RPC for Base
pub fn rpc_url() -> &'static str {
    "https://mainnet.base.org"
}

/// Common tick spacings on Aerodrome Slipstream, in ascending order.
/// Try all of these when auto-detecting the best pool for a swap.
pub fn common_tick_spacings() -> &'static [i32] {
    &[1, 50, 100, 200, 2000]
}

/// Resolve a token symbol or hex address to its Base mainnet address.
pub fn resolve_token(symbol: &str) -> String {
    if symbol.starts_with("0x") || symbol.starts_with("0X") {
        return symbol.to_lowercase();
    }
    match symbol.to_uppercase().as_str() {
        "ETH" | "WETH" => "0x4200000000000000000000000000000000000006",
        "USDC"          => "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        "AERO"          => "0x940181a94a35a4569e4529a3cdfb74e38fd98631",
        "CBETH"         => "0x2ae3f1ec7f1f5012cfeab0185bfc7aa3cf0dec22",
        "USDT"          => "0xfde4c96c8593536e31f229ea8f37b2ada2699bb2",
        "DAI"           => "0x50c5725949a6f0c72e6c4a641f24049a917db0cb",
        "WBTC"          => "0x0555e30da8f98308edb960aa94c0db47230d2b9c",
        "CBBTC"         => "0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf",
        "VIRTUAL"       => "0x0b3e328455c4059eeb9e3f84b5543f74e24e7e1b",
        "BRETT"         => "0x532f27101965dd16442e59d40670faf5ebb142e4",
        _ => symbol,
    }
    .to_string()
}

/// Canonical token symbol for display (reverse lookup).
pub fn token_symbol(addr: &str) -> &'static str {
    match addr.to_lowercase().as_str() {
        "0x4200000000000000000000000000000000000006" => "WETH",
        "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" => "USDC",
        "0x940181a94a35a4569e4529a3cdfb74e38fd98631" => "AERO",
        "0x2ae3f1ec7f1f5012cfeab0185bfc7aa3cf0dec22" => "cbETH",
        "0xfde4c96c8593536e31f229ea8f37b2ada2699bb2" => "USDT",
        "0x50c5725949a6f0c72e6c4a641f24049a917db0cb" => "DAI",
        "0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf" => "cbBTC",
        _ => "UNKNOWN",
    }
}

// ── ABI helpers ───────────────────────────────────────────────────────────────

/// Pad a hex address (with or without 0x) to 32 bytes.
pub fn pad_address(addr: &str) -> String {
    format!("{:0>64}", addr.trim_start_matches("0x").to_lowercase())
}

/// Pad a u128 value to 32 bytes hex.
pub fn pad_u256(val: u128) -> String {
    format!("{:0>64x}", val)
}

/// ERC-20 approve calldata: approve(address spender, uint256 amount)
/// Selector: 0x095ea7b3
pub fn build_approve_calldata(spender: &str, amount: u128) -> String {
    format!(
        "0x095ea7b3{}{}",
        pad_address(spender),
        pad_u256(amount)
    )
}

/// Current unix timestamp in seconds.
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
