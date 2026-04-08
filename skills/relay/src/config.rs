pub const API_BASE: &str = "https://api.relay.link";

/// Native ETH address (used as currency address for ETH)
pub const ETH_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

/// Parse a human-readable token amount string into raw integer units.
/// E.g. parse_units("1.5", 18) == 1_500_000_000_000_000_000
pub fn parse_units(amount_str: &str, decimals: u8) -> anyhow::Result<u128> {
    let s = amount_str.trim();
    if s.is_empty() {
        anyhow::bail!("Empty amount string");
    }
    let d = decimals as u32;
    let multiplier = 10u128.pow(d);
    if let Some(dot_pos) = s.find('.') {
        let whole: u128 = s[..dot_pos].parse().map_err(|_| anyhow::anyhow!("Invalid whole part in: {}", s))?;
        let frac_str = &s[dot_pos + 1..];
        let frac_len = frac_str.len() as u32;
        let frac: u128 = frac_str.parse().map_err(|_| anyhow::anyhow!("Invalid fractional part in: {}", s))?;
        if frac_len > d {
            anyhow::bail!("Too many decimal places (max {})", d);
        }
        let frac_scaled = frac * 10u128.pow(d - frac_len);
        Ok(whole * multiplier + frac_scaled)
    } else {
        let whole: u128 = s.parse().map_err(|_| anyhow::anyhow!("Invalid integer amount: {}", s))?;
        Ok(whole * multiplier)
    }
}
