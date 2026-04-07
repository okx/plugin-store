/// ABI calldata encoding for Fluid Protocol contracts.

/// Encode a 20-byte address as a 32-byte hex slot (left-zero-padded, no 0x prefix).
pub fn encode_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Encode a u128 as a 32-byte hex slot (no 0x prefix).
pub fn encode_u256(val: u128) -> String {
    format!("{:064x}", val)
}

/// Encode a bool as a 32-byte slot.
pub fn encode_bool(val: bool) -> String {
    if val {
        format!("{:064x}", 1u32)
    } else {
        format!("{:064x}", 0u32)
    }
}

/// ERC-4626 deposit(uint256 assets, address receiver)
/// Selector: 0x6e553f65
pub fn encode_ftoken_deposit(assets: u128, receiver: &str) -> String {
    format!(
        "0x6e553f65{}{}",
        encode_u256(assets),
        encode_address(receiver),
    )
}

/// ERC-4626 withdraw(uint256 assets, address receiver, address owner)
/// Selector: 0xb460af94
pub fn encode_ftoken_withdraw(assets: u128, receiver: &str, owner: &str) -> String {
    format!(
        "0xb460af94{}{}{}",
        encode_u256(assets),
        encode_address(receiver),
        encode_address(owner),
    )
}

/// ERC-4626 redeem(uint256 shares, address receiver, address owner)
/// Selector: 0xba087652
pub fn encode_ftoken_redeem(shares: u128, receiver: &str, owner: &str) -> String {
    format!(
        "0xba087652{}{}{}",
        encode_u256(shares),
        encode_address(receiver),
        encode_address(owner),
    )
}

/// ERC-20 approve(address spender, uint256 amount)
/// Selector: 0x095ea7b3
pub fn encode_approve(spender: &str, amount: u128) -> String {
    let spender_clean = spender.trim_start_matches("0x");
    format!(
        "0x095ea7b3{:0>64}{:064x}",
        spender_clean,
        amount,
    )
}

/// Fluid DEX swapIn(bool swap0to1, uint256 amountIn, uint256 amountOutMin, address to)
/// Selector: 0x2668dfaa
pub fn encode_swap_in(swap0to1: bool, amount_in: u128, amount_out_min: u128, to: &str) -> String {
    format!(
        "0x2668dfaa{}{}{}{}",
        encode_bool(swap0to1),
        encode_u256(amount_in),
        encode_u256(amount_out_min),
        encode_address(to),
    )
}

/// Fluid DEX swapOut(bool swap0to1, uint256 amountOut, uint256 amountInMax, address to)
/// Selector: 0x286f0e61
#[allow(dead_code)]
pub fn encode_swap_out(swap0to1: bool, amount_out: u128, amount_in_max: u128, to: &str) -> String {
    format!(
        "0x286f0e61{}{}{}{}",
        encode_bool(swap0to1),
        encode_u256(amount_out),
        encode_u256(amount_in_max),
        encode_address(to),
    )
}

/// LendingResolver getFTokensEntireData()
/// Selector: 0xe26533a3
pub fn encode_get_ftokens_entire_data() -> String {
    "0xe26533a3".to_string()
}

/// LendingResolver getUserPositions(address user)
/// Selector: 0x2a6bc2dd
pub fn encode_get_user_positions(user: &str) -> String {
    format!("0x2a6bc2dd{}", encode_address(user))
}

/// Parse human-readable amount to raw token amount given decimals.
pub fn parse_amount(amount_str: &str, decimals: u8) -> anyhow::Result<u128> {
    let parts: Vec<&str> = amount_str.split('.').collect();
    match parts.len() {
        1 => {
            let whole: u128 = parts[0].parse()?;
            Ok(whole * 10u128.pow(decimals as u32))
        }
        2 => {
            let whole: u128 = parts[0].parse()?;
            let frac_str = parts[1];
            let frac_len = frac_str.len() as u32;
            let frac: u128 = frac_str.parse()?;
            if frac_len > decimals as u32 {
                anyhow::bail!("Too many decimal places: {} (max {})", frac_len, decimals);
            }
            let frac_scaled = frac * 10u128.pow(decimals as u32 - frac_len);
            Ok(whole * 10u128.pow(decimals as u32) + frac_scaled)
        }
        _ => anyhow::bail!("Invalid amount: {}", amount_str),
    }
}

/// Format raw token amount to human-readable string.
pub fn format_amount(raw: u128, decimals: u8) -> String {
    if decimals == 0 {
        return raw.to_string();
    }
    let divisor = 10u128.pow(decimals as u32);
    let whole = raw / divisor;
    let frac = raw % divisor;
    if frac == 0 {
        format!("{}", whole)
    } else {
        let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
        let frac_trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, frac_trimmed)
    }
}

/// Compute annual percentage yield from rate per second (ray = 1e27 base).
/// rate is stored as tokens per 1e15 seconds (Fluid specific) — simplified to just pass through
#[allow(dead_code)]
pub fn format_apy(rate_per_year_ray: u128) -> String {
    // supplyRate is in 1e2 = percentage * 100, e.g. 450 = 4.50%
    let pct = rate_per_year_ray as f64 / 1e4;
    format!("{:.4}%", pct)
}
