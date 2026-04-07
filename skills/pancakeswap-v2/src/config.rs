/// Resolve a token symbol or hex address to a hex address.
/// If the input is already a hex address (starts with 0x and is 42 chars), return as-is.
pub fn resolve_token_address(symbol: &str, chain_id: u64) -> String {
    match (symbol.to_uppercase().as_str(), chain_id) {
        // BSC (56) — BNB and WBNB both resolve to WBNB for ABI usage
        ("BNB", 56) | ("WBNB", 56) => "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c",
        ("USDT", 56) | ("BSC-USD", 56) => "0x55d398326f99059fF775485246999027B3197955",
        ("USDC", 56) => "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d",
        ("BUSD", 56) => "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56",
        ("CAKE", 56) => "0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82",
        ("BTCB", 56) => "0x7130d2A12B9BCbFAe4f2634d864A1Ee1Ce3Ead9c",
        ("ETH", 56) => "0x2170Ed0880ac9A755fd29B2688956BD959F933F8",
        _ => symbol, // assume already a hex address
    }
    .to_string()
}

/// Returns true if the given symbol is native BNB (not WBNB ERC-20).
pub fn is_native_bnb(symbol: &str) -> bool {
    symbol.to_uppercase() == "BNB"
}

pub const ROUTER_V2: &str = "0x10ED43C718714eb63d5aA57B78B54704E256024E";
pub const FACTORY_V2: &str = "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73";
pub const WBNB: &str = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c";
pub const BSC_RPC: &str = "https://bsc-rpc.publicnode.com";
pub const CHAIN_ID: u64 = 56;

/// Deadline: current unix timestamp + 20 minutes.
pub fn deadline() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + 1200
}

/// Apply 0.5% slippage (995/1000).
pub fn apply_slippage(amount: u128) -> u128 {
    amount * 995 / 1000
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

/// Encode an address[] dynamic array for ABI encoding.
/// Returns the raw hex (no 0x) for the array portion: length + elements.
pub fn encode_address_array(addrs: &[&str]) -> String {
    let mut out = String::new();
    // length
    out.push_str(&format!("{:0>64x}", addrs.len()));
    // elements
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
