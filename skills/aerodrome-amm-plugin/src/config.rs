// Aerodrome AMM (volatile/stable pools) — Base mainnet (chain 8453)

pub const CHAIN_ID: u64 = 8453;

/// Aerodrome V2 Pool Factory
/// Verified: getPool(WETH,USDC,false) returns 0xcdac0d6c...
pub fn factory() -> &'static str {
    "0x420DD381b31aEf6683db6B902084cB0FFECe40Da"
}

/// Aerodrome V2 Router
pub fn router() -> &'static str {
    "0xcF77a3Ba9A5CA399B7c97c74d54e5b1Beb874E43"
}

/// Primary RPC for Base
pub fn rpc_url() -> &'static str {
    "https://base-rpc.publicnode.com"
}

/// Resolve a token symbol or hex address to its Base mainnet address.
pub fn resolve_token(symbol: &str) -> String {
    if symbol.starts_with("0x") || symbol.starts_with("0X") {
        return symbol.to_lowercase();
    }
    match symbol.to_uppercase().as_str() {
        "ETH" | "WETH" => "0x4200000000000000000000000000000000000006",
        "USDC"         => "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        "AERO"         => "0x940181a94a35a4569e4529a3cdfb74e38fd98631",
        "CBETH"        => "0x2ae3f1ec7f1f5012cfeab0185bfc7aa3cf0dec22",
        "USDT"         => "0xfde4c96c8593536e31f229ea8f37b2ada2699bb2",
        "DAI"          => "0x50c5725949a6f0c72e6c4a641f24049a917db0cb",
        "WBTC"         => "0x0555e30da8f98308edb960aa94c0db47230d2b9c",
        "CBBTC"        => "0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf",
        "VIRTUAL"      => "0x0b3e328455c4059eeb9e3f84b5543f74e24e7e1b",
        "BRETT"        => "0x532f27101965dd16442e59d40670faf5ebb142e4",
        "EURC"         => "0x60a3e35cc302bfa44cb288bc5a4f316fdb1adb42",
        _              => symbol,
    }
    .to_string()
}

/// Like resolve_token but returns an error for unknown symbols or malformed addresses.
pub fn resolve_token_validated(symbol: &str) -> anyhow::Result<String> {
    let resolved = resolve_token(symbol);
    if !resolved.starts_with("0x") {
        anyhow::bail!(
            "Unknown token '{}'. Use a supported symbol (WETH, USDC, AERO, USDT, DAI, \
             cbETH, cbBTC, EURC) or provide a full ERC-20 address (0x + 40 hex chars).",
            symbol
        );
    }
    let hex_part = &resolved[2..];
    if hex_part.len() != 40 || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!(
            "Invalid token address '{}'. Expected 0x + 40 hex characters, got 0x + {} chars.",
            symbol,
            hex_part.len()
        );
    }
    Ok(resolved)
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
        "0x60a3e35cc302bfa44cb288bc5a4f316fdb1adb42" => "EURC",
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

/// Pad a bool to 32 bytes hex (false=0, true=1).
pub fn pad_bool(val: bool) -> String {
    format!("{:0>64x}", val as u8)
}

/// ERC-20 approve calldata: approve(address spender, uint256 amount)
/// Selector: 0x095ea7b3
pub fn build_approve_calldata(spender: &str, amount: u128) -> String {
    format!("0x095ea7b3{}{}", pad_address(spender), pad_u256(amount))
}

/// Current unix timestamp in seconds.
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
