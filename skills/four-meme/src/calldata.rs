/// ABI-encode: pad an address to 32 bytes
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x").to_lowercase();
    format!("{:0>64}", clean)
}

/// ABI-encode: pad a u128 to 32 bytes
pub fn pad_u128(val: u128) -> String {
    format!("{:064x}", val)
}

/// ABI-encode: pad a u64 to 32 bytes
pub fn pad_u64(val: u64) -> String {
    format!("{:064x}", val)
}

/// Build calldata for:
///   buyTokenAMAP(address token, uint256 funds, uint256 minAmount) payable
/// Selector: 0x87f27655
/// Used when buying for msg.sender with BNB
pub fn build_buy_amap_calldata(token: &str, funds_wei: u128, min_amount: u128) -> String {
    let selector = "87f27655";
    format!(
        "0x{}{}{}{}",
        selector,
        pad_address(token),
        pad_u128(funds_wei),
        pad_u128(min_amount)
    )
}

/// Build calldata for:
///   sellToken(address token, uint256 amount, uint256 minFunds)
/// Selector: 0x3e11741f
pub fn build_sell_calldata(token: &str, amount: u128, min_funds_wei: u128) -> String {
    let selector = "3e11741f";
    format!(
        "0x{}{}{}{}",
        selector,
        pad_address(token),
        pad_u128(amount),
        pad_u128(min_funds_wei)
    )
}

/// Build calldata for:
///   sellToken(address token, uint256 amount)
/// Selector: 0xf464e7db (no slippage protection)
pub fn build_sell_no_min_calldata(token: &str, amount: u128) -> String {
    let selector = "f464e7db";
    format!(
        "0x{}{}{}",
        selector,
        pad_address(token),
        pad_u128(amount)
    )
}

/// Build calldata for ERC20 approve:
///   approve(address spender, uint256 amount)
/// Selector: 0x095ea7b3
pub fn build_approve_calldata(spender: &str, amount: u128) -> String {
    let selector = "095ea7b3";
    format!(
        "0x{}{}{}",
        selector,
        pad_address(spender),
        pad_u128(amount)
    )
}

/// Build calldata for:
///   tryBuy(address token, uint256 amount, uint256 funds)
/// Selector: 0xe21b103a
/// amount=0 means spend `funds` BNB to get as many tokens as possible
pub fn build_try_buy_calldata(token: &str, amount: u128, funds: u128) -> String {
    let selector = "e21b103a";
    format!(
        "0x{}{}{}{}",
        selector,
        pad_address(token),
        pad_u128(amount),
        pad_u128(funds)
    )
}

/// Build calldata for:
///   trySell(address token, uint256 amount)
/// Selector: 0xc6f43e8c
pub fn build_try_sell_calldata(token: &str, amount: u128) -> String {
    let selector = "c6f43e8c";
    format!(
        "0x{}{}{}",
        selector,
        pad_address(token),
        pad_u128(amount)
    )
}

/// Build calldata for:
///   getTokenInfo(address token)
/// Selector: 0x1f69565f
pub fn build_get_token_info_calldata(token: &str) -> String {
    let selector = "1f69565f";
    format!("0x{}{}", selector, pad_address(token))
}

/// Parse a BNB string like "0.001" into wei (u128), no float used.
/// Truncates to gwei precision (9 decimal places) to avoid alignment errors.
pub fn parse_bnb_to_wei(amount_bnb: &str) -> u128 {
    let parts: Vec<&str> = amount_bnb.split('.').collect();
    let whole: u128 = parts[0].parse().unwrap_or(0);
    let frac_str = parts.get(1).copied().unwrap_or("0");
    // Pad or truncate to 18 decimal places
    let frac_pad = format!("{:0<18}", frac_str);
    let frac18: u128 = frac_pad[..18].parse().unwrap_or(0);
    let total_wei = whole * 1_000_000_000_000_000_000u128 + frac18;
    // Align to gwei (1e9) — round down
    let gwei = 1_000_000_000u128;
    (total_wei / gwei) * gwei
}

/// Format wei as a human-readable BNB string (no float)
pub fn format_wei_as_bnb(wei: u128) -> String {
    let whole = wei / 1_000_000_000_000_000_000u128;
    let frac = wei % 1_000_000_000_000_000_000u128;
    if frac == 0 {
        format!("{}", whole)
    } else {
        // Print up to 9 significant decimal digits (gwei precision)
        let frac_str = format!("{:018}", frac);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, &trimmed[..trimmed.len().min(9)])
    }
}

/// Format token amount (18 decimals) as human-readable
pub fn format_token_amount(raw: u128) -> String {
    let whole = raw / 1_000_000_000_000_000_000u128;
    let frac = raw % 1_000_000_000_000_000_000u128;
    if frac == 0 {
        format!("{}", whole)
    } else {
        let frac_str = format!("{:018}", frac);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, &trimmed[..trimmed.len().min(6)])
    }
}

/// Apply slippage to a minimum amount (bps = basis points, e.g. 100 = 1%)
pub fn apply_slippage_min(amount: u128, slippage_bps: u64) -> u128 {
    if slippage_bps == 0 || amount == 0 {
        return amount;
    }
    // min = amount * (10000 - slippage_bps) / 10000
    amount
        .saturating_mul(10000u128.saturating_sub(slippage_bps as u128))
        / 10000u128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bnb() {
        // 0.001 BNB = 1e15 wei (gwei-aligned)
        assert_eq!(parse_bnb_to_wei("0.001"), 1_000_000_000_000_000u128);
        // 1.5 BNB
        assert_eq!(parse_bnb_to_wei("1.5"), 1_500_000_000_000_000_000u128);
        // 0.0015 BNB = 1500000000000000 wei
        assert_eq!(parse_bnb_to_wei("0.0015"), 1_500_000_000_000_000u128);
    }

    #[test]
    fn test_format_wei() {
        assert_eq!(format_wei_as_bnb(1_000_000_000_000_000u128), "0.001");
        assert_eq!(format_wei_as_bnb(1_500_000_000_000_000_000u128), "1.5");
    }
}
