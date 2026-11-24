/// Ethereum mainnet chain ID.
pub const CHAIN_ID: u64 = 1;

/// ether.fi eETH token (ERC-20) on Ethereum mainnet.
pub fn eeth_address() -> &'static str {
    "0x35fA164735182de50811E8e2E824cFb9B6118ac2"
}

/// ether.fi weETH token (ERC-4626 wrapped eETH) on Ethereum mainnet.
pub fn weeth_address() -> &'static str {
    "0xCd5fE23C85820F7B72D0926FC9b05b43E359b7ee"
}

/// ether.fi LiquidityPool — accepts ETH deposits, issues eETH.
pub fn liquidity_pool_address() -> &'static str {
    "0x308861A430be4cce5502d0A12724771Fc6DaF216"
}

/// ether.fi WithdrawRequestNFT — minted by LiquidityPool.requestWithdraw(),
/// burned by claimWithdraw() to release ETH after finalization.
pub fn withdraw_request_nft_address() -> &'static str {
    "0x7d5706f6ef3F89B3951E23e557CDFBC3239D4E2c"
}

/// Ethereum mainnet public RPC endpoint.
pub fn rpc_url() -> &'static str {
    "https://ethereum-rpc.publicnode.com"
}

/// Parse a decimal string amount into the raw u128 integer in smallest units.
/// Uses only integer arithmetic — no f64.
///
/// Examples:
///   parse_units("1.5", 18) = 1_500_000_000_000_000_000
///   parse_units("0.01", 6)  = 10_000
///   parse_units("100", 18)  = 100_000_000_000_000_000_000
pub fn parse_units(amount_str: &str, decimals: u8) -> anyhow::Result<u128> {
    let s = amount_str.trim();
    let (integer_part, frac_part) = if let Some(dot_pos) = s.find('.') {
        let int_s = &s[..dot_pos];
        let frac_s = &s[dot_pos + 1..];
        (int_s, frac_s)
    } else {
        (s, "")
    };

    // Parse integer part
    let int_val: u128 = if integer_part.is_empty() {
        0
    } else {
        integer_part
            .parse::<u128>()
            .map_err(|_| anyhow::anyhow!("Invalid integer part in amount: {}", amount_str))?
    };

    // Multiply integer part by 10^decimals
    let scale: u128 = 10u128
        .checked_pow(decimals as u32)
        .ok_or_else(|| anyhow::anyhow!("Decimals too large: {}", decimals))?;

    let int_wei = int_val
        .checked_mul(scale)
        .ok_or_else(|| anyhow::anyhow!("Overflow in integer part of amount: {}", amount_str))?;

    // Handle fractional part
    let frac_wei = if frac_part.is_empty() {
        0u128
    } else {
        let frac_len = frac_part.len() as u32;
        if frac_len > decimals as u32 {
            // Truncate extra precision
            let truncated = &frac_part[..decimals as usize];
            truncated
                .parse::<u128>()
                .map_err(|_| anyhow::anyhow!("Invalid fractional part in amount: {}", amount_str))?
        } else {
            let frac_val: u128 = frac_part
                .parse::<u128>()
                .map_err(|_| anyhow::anyhow!("Invalid fractional part in amount: {}", amount_str))?;
            // Scale up to fill remaining decimal places
            let remaining = decimals as u32 - frac_len;
            let frac_scale: u128 = 10u128
                .checked_pow(remaining)
                .ok_or_else(|| anyhow::anyhow!("Decimals too large: {}", remaining))?;
            frac_val
                .checked_mul(frac_scale)
                .ok_or_else(|| anyhow::anyhow!("Overflow in fractional part: {}", amount_str))?
        }
    };

    int_wei
        .checked_add(frac_wei)
        .ok_or_else(|| anyhow::anyhow!("Overflow combining integer and fractional: {}", amount_str))
}

/// Format a wei u128 value as a human-readable string with `decimals` decimal places.
/// Trims trailing zeros after the decimal point.
pub fn format_units(wei: u128, decimals: u8) -> String {
    let scale: u128 = 10u128.pow(decimals as u32);
    let int_part = wei / scale;
    let frac_part = wei % scale;
    if frac_part == 0 {
        return format!("{}", int_part);
    }
    let frac_str = format!("{:0>width$}", frac_part, width = decimals as usize);
    let trimmed = frac_str.trim_end_matches('0');
    format!("{}.{}", int_part, trimmed)
}

/// Build ERC-20 approve calldata: approve(address spender, uint256 amount)
/// Selector: 0x095ea7b3
pub fn build_approve_calldata(spender: &str, amount: u128) -> String {
    let spender_padded = format!("{:0>64}", spender.trim_start_matches("0x"));
    let amount_hex = format!("{:0>64x}", amount);
    format!("0x095ea7b3{}{}", spender_padded, amount_hex)
}

/// Pad an address to 32 bytes (no 0x prefix in output).
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u128 value to 32 bytes hex.
pub fn pad_u256(val: u128) -> String {
    format!("{:0>64x}", val)
}
