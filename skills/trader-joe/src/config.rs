/// Trader Joe Liquidity Book — Arbitrum (42161) configuration

pub const RPC_URL: &str = "https://arb1.arbitrum.io/rpc";

/// LBRouter V2.2 on Arbitrum
pub const LB_ROUTER: &str = "0x18556DA13313f3532c54711497A8FedAC273220E";
/// LBFactory V2.2 on Arbitrum
pub const LB_FACTORY: &str = "0xb43120c4745967fa9b93E79C149E66B0f2D6Fe0c";
/// LBQuoter (multi-version) on Arbitrum
pub const LB_QUOTER: &str = "0xd76019A16606FDa4651f636D9751f500Ed776250";

/// WETH (Wrapped Ether) on Arbitrum
pub const WETH: &str = "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1";
/// USDT (USD₮0) on Arbitrum
pub const USDT: &str = "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9";
/// USDC on Arbitrum
pub const USDC: &str = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831";
/// WBTC on Arbitrum
pub const WBTC: &str = "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f";
/// ARB on Arbitrum
pub const ARB: &str = "0x912CE59144191C1204E64559FE8253a0e49E6548";

/// Resolve a token symbol or hex address to a checksummed hex address.
/// If the input is already a hex address (starts with 0x and is 42 chars), return as-is.
pub fn resolve_token_address(symbol: &str, _chain_id: u64) -> String {
    match symbol.to_uppercase().as_str() {
        "ETH" | "WETH" => WETH,
        "USDT" | "USDT0" | "USD₮0" => USDT,
        "USDC" => USDC,
        "WBTC" => WBTC,
        "ARB" => ARB,
        _ => symbol, // assume already a hex address
    }
    .to_string()
}

/// Returns true if the symbol represents native ETH (not WETH).
pub fn is_native_eth(symbol: &str) -> bool {
    symbol.to_uppercase() == "ETH"
}

/// Deadline: current unix timestamp + secs.
pub fn deadline(secs: u64) -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + secs
}

/// Apply slippage: amount * (10000 - bps) / 10000
/// e.g. slippage_bps=50 means 0.5% slippage.
pub fn apply_slippage(amount: u128, slippage_bps: u128) -> u128 {
    amount * (10000 - slippage_bps) / 10000
}

/// Pad an address (with or without 0x) to 32 bytes hex (no 0x prefix).
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u128 to 32 bytes hex (no 0x prefix).
pub fn pad_u256(val: u128) -> String {
    format!("{:0>64x}", val)
}

/// Encode a uint256[] dynamic array for ABI encoding.
/// Returns the raw hex (no 0x): length + elements.
pub fn encode_u256_array(vals: &[u128]) -> String {
    let mut out = format!("{:0>64x}", vals.len());
    for v in vals {
        out.push_str(&pad_u256(*v));
    }
    out
}

/// Encode a uint8[] (versions) dynamic array for ABI encoding.
/// Each element is padded to 32 bytes.
pub fn encode_u8_array(vals: &[u8]) -> String {
    let mut out = format!("{:0>64x}", vals.len());
    for v in vals {
        out.push_str(&format!("{:0>64x}", v));
    }
    out
}

/// Encode an address[] dynamic array for ABI encoding.
pub fn encode_address_array(addrs: &[&str]) -> String {
    let mut out = format!("{:0>64x}", addrs.len());
    for addr in addrs {
        out.push_str(&pad_address(addr));
    }
    out
}

/// Build ERC-20 approve calldata: approve(address spender, uint256 amount).
/// Selector: 0x095ea7b3
pub fn build_approve_calldata(spender: &str, amount: u128) -> String {
    let spender_padded = pad_address(spender);
    let amount_hex = pad_u256(amount);
    format!("0x095ea7b3{}{}", spender_padded, amount_hex)
}
