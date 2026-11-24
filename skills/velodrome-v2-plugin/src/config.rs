/// Resolve a token symbol or hex address to a hex address on Optimism (chain 10).
/// If the input is already a hex address (starts with 0x), return as-is.
pub fn resolve_token_address(symbol: &str) -> String {
    if symbol.starts_with("0x") || symbol.starts_with("0X") {
        return symbol.to_string();
    }
    match symbol.to_uppercase().as_str() {
        "WETH" | "ETH" => "0x4200000000000000000000000000000000000006",
        "USDC" => "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
        "USDT" => "0x94b008aA00579c1307B0EF2c499aD98a8ce58e58",
        "DAI" => "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1",
        "VELO" => "0x9560e827aF36c94D2Ac33a39bCE1Fe78631088Db",
        "WBTC" => "0x68f180fcCe6836688e9084f035309E29Bf0A2095",
        "OP" => "0x4200000000000000000000000000000000000042",
        "WSTETH" => "0x1F32b1c2345538c0c6f582fCB022739c4A194Ebb",
        "SNX" => "0x8700dAec35aF8Ff88c16BdF0418774CB3D7599B4",
        _ => symbol,
    }
    .to_string()
}

/// RPC URL for Optimism (chain 10).
pub fn rpc_url() -> &'static str {
    "https://optimism-rpc.publicnode.com"
}

/// Velodrome V2 Classic AMM Router on Optimism.
pub fn router_address() -> &'static str {
    "0xa062aE8A9c5e11aaA026fc2670B0D65cCc8B2858"
}

/// Velodrome V2 PoolFactory on Optimism.
pub fn factory_address() -> &'static str {
    "0xF1046053aa5682b4F9a81b5481394DA16BE5FF5a"
}

/// Velodrome V2 Voter on Optimism (used to look up gauge addresses).
pub fn voter_address() -> &'static str {
    "0x41C914ee0c7E1A5edCD0295623e6dC557B5aBf3C"
}

/// Build ERC-20 approve calldata: approve(address,uint256).
/// Selector: 0x095ea7b3
pub fn build_approve_calldata(spender: &str, amount: u128) -> String {
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:0>64x}", amount);
    format!("0x095ea7b3{}{}", spender_padded, amount_hex)
}

/// Pad an address to 32 bytes (no 0x prefix in output).
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u128/u64 value to 32 bytes hex.
pub fn pad_u256(val: u128) -> String {
    format!("{:0>64x}", val)
}

/// Current unix timestamp in seconds.
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Encode a Route struct for ABI encoding.
/// Route { from: address, to: address, stable: bool, factory: address }
/// Each Route = 4 x 32 bytes = 128 bytes
pub fn encode_route(from: &str, to: &str, stable: bool, factory: &str) -> String {
    format!(
        "{}{}{}{}",
        pad_address(from),
        pad_address(to),
        pad_u256(stable as u128),
        pad_address(factory),
    )
}

/// Build calldata for swapExactTokensForTokens with a single-hop route.
/// Selector: 0xcac88ea9
/// swapExactTokensForTokens(uint256 amountIn, uint256 amountOutMin,
///   Route[] routes, address to, uint256 deadline)
///
/// ABI encoding for dynamic array Route[]:
///   [0] amountIn (32 bytes)
///   [1] amountOutMin (32 bytes)
///   [2] offset to routes (= 0xa0 = 160 = 5 x 32)
///   [3] to (32 bytes)
///   [4] deadline (32 bytes)
///   [5] routes.length (32 bytes)  at offset 0xa0
///   [6..] route data (128 bytes per route)
pub fn build_swap_calldata(
    amount_in: u128,
    amount_out_min: u128,
    token_in: &str,
    token_out: &str,
    stable: bool,
    factory: &str,
    recipient: &str,
    deadline: u64,
) -> String {
    // Offset to routes array = 5 static words x 32 bytes = 160 = 0xa0
    let routes_offset = pad_u256(0xa0);
    let route_data = encode_route(token_in, token_out, stable, factory);
    let routes_length = pad_u256(1); // single hop

    format!(
        "0xcac88ea9{}{}{}{}{}{}{}",
        pad_u256(amount_in),
        pad_u256(amount_out_min),
        routes_offset,
        pad_address(recipient),
        pad_u256(deadline as u128),
        routes_length,
        route_data,
    )
}

/// Build calldata for addLiquidity.
/// Selector: 0x5a47ddc3
/// addLiquidity(address tokenA, address tokenB, bool stable,
///   uint256 amountADesired, uint256 amountBDesired,
///   uint256 amountAMin, uint256 amountBMin,
///   address to, uint256 deadline)
pub fn build_add_liquidity_calldata(
    token_a: &str,
    token_b: &str,
    stable: bool,
    amount_a_desired: u128,
    amount_b_desired: u128,
    amount_a_min: u128,
    amount_b_min: u128,
    to: &str,
    deadline: u64,
) -> String {
    format!(
        "0x5a47ddc3{}{}{}{}{}{}{}{}{}",
        pad_address(token_a),
        pad_address(token_b),
        pad_u256(stable as u128),
        pad_u256(amount_a_desired),
        pad_u256(amount_b_desired),
        pad_u256(amount_a_min),
        pad_u256(amount_b_min),
        pad_address(to),
        pad_u256(deadline as u128),
    )
}

/// Build calldata for removeLiquidity.
/// Selector: 0x0dede6c4
/// removeLiquidity(address tokenA, address tokenB, bool stable,
///   uint256 liquidity, uint256 amountAMin, uint256 amountBMin,
///   address to, uint256 deadline)
pub fn build_remove_liquidity_calldata(
    token_a: &str,
    token_b: &str,
    stable: bool,
    liquidity: u128,
    amount_a_min: u128,
    amount_b_min: u128,
    to: &str,
    deadline: u64,
) -> String {
    format!(
        "0x0dede6c4{}{}{}{}{}{}{}{}",
        pad_address(token_a),
        pad_address(token_b),
        pad_u256(stable as u128),
        pad_u256(liquidity),
        pad_u256(amount_a_min),
        pad_u256(amount_b_min),
        pad_address(to),
        pad_u256(deadline as u128),
    )
}
