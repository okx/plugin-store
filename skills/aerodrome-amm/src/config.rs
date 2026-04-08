/// Resolve a token symbol or hex address to a hex address on Base (chain 8453).
/// If the input is already a hex address (starts with 0x), return as-is.
pub fn resolve_token_address(symbol: &str) -> String {
    if symbol.starts_with("0x") || symbol.starts_with("0X") {
        return symbol.to_string();
    }
    match symbol.to_uppercase().as_str() {
        "WETH" | "ETH" => "0x4200000000000000000000000000000000000006",
        "USDC" => "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "CBBTC" => "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf",
        "AERO" => "0x940181a94A35A4569E4529A3CDfB74e38FD98631",
        "DAI" => "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb",
        "USDT" => "0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2",
        "WSTETH" => "0xc1CBa3fCea344f92D9239c08C0568f6F2F0ee452",
        _ => symbol,
    }
    .to_string()
}

/// RPC URL for Base (chain 8453).
pub fn rpc_url() -> &'static str {
    "https://base-rpc.publicnode.com"
}

/// Aerodrome Classic AMM Router on Base.
/// NOTE: Different from Slipstream CL Router (0xBE6D8f0d...).
pub fn router_address() -> &'static str {
    "0xcF77a3Ba9A5CA399B7c97c74d54e5b1Beb874E43"
}

/// Aerodrome PoolFactory on Base.
pub fn factory_address() -> &'static str {
    "0x420DD381b31aEf6683db6B902084cB0FFECe40Da"
}

/// Aerodrome Voter on Base (used to look up gauge addresses).
pub fn voter_address() -> &'static str {
    "0x16613524E02ad97eDfeF371bC883F2F5d6C480A5"
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
/// Each Route = 4 × 32 bytes = 128 bytes
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
///   [2] offset to routes (= 0xa0 = 160 = 5 × 32)
///   [3] to (32 bytes)
///   [4] deadline (32 bytes)
///   [5] routes.length (32 bytes)  ← at offset 0xa0
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
    // Offset to routes array = 5 static words × 32 bytes = 160 = 0xa0
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
