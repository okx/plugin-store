/// ABI encoding helpers — hand-rolled for minimal dependency footprint.

/// Pad a hex address (with or without 0x) to a 32-byte (64 hex char) left-zero-padded word.
pub fn encode_address(addr: &str) -> String {
    let addr = addr.trim_start_matches("0x").trim_start_matches("0X");
    format!("{:0>64}", addr)
}

/// Build calldata for a no-arg function: `selector()`.
pub fn calldata_noarg(selector: &str) -> String {
    format!("0x{}", selector)
}

/// Build calldata for a single-address param function: `selector(address)`.
pub fn calldata_single_address(selector: &str, addr: &str) -> String {
    format!("0x{}{}", selector, encode_address(addr))
}

/// Decode a single uint256 from ABI-encoded return data (32-byte hex, optional 0x prefix).
pub fn decode_uint256(hex: &str) -> anyhow::Result<u128> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 64 {
        anyhow::bail!("Return data too short for uint256: '{}'", hex);
    }
    // Take the last 32 bytes (64 hex chars) to handle 0-padded responses
    let word = &hex[hex.len() - 64..];
    Ok(u128::from_str_radix(word, 16)?)
}

/// Extract the raw hex return value from an onchainos/eth_call response.
pub fn extract_return_data(result: &serde_json::Value) -> anyhow::Result<String> {
    if let Some(s) = result["data"]["result"].as_str() {
        return Ok(s.to_string());
    }
    if let Some(s) = result["data"]["returnData"].as_str() {
        return Ok(s.to_string());
    }
    if let Some(s) = result["result"].as_str() {
        return Ok(s.to_string());
    }
    anyhow::bail!("Could not extract return data from: {}", result)
}

/// Format a u128 wei value as a human-readable ETH string (e.g. "1.234567890123456789").
pub fn format_eth(wei: u128) -> String {
    let whole = wei / 1_000_000_000_000_000_000u128;
    let frac = wei % 1_000_000_000_000_000_000u128;
    if frac == 0 {
        format!("{}", whole)
    } else {
        // Format with 18 decimal places, strip trailing zeros
        let frac_str = format!("{:018}", frac);
        let frac_trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, frac_trimmed)
    }
}

/// Parse a human-readable ETH amount string to wei (u128).
pub fn parse_eth_to_wei(amount: &str) -> anyhow::Result<u128> {
    let amount = amount.trim();
    if let Some(dot_pos) = amount.find('.') {
        let whole_str = &amount[..dot_pos];
        let frac_str = &amount[dot_pos + 1..];
        let whole: u128 = if whole_str.is_empty() { 0 } else { whole_str.parse()? };
        // Pad or truncate to 18 decimal places
        let frac_padded = format!("{:0<18}", frac_str);
        let frac_truncated = &frac_padded[..18];
        let frac: u128 = frac_truncated.parse()?;
        Ok(whole * 1_000_000_000_000_000_000u128 + frac)
    } else {
        let whole: u128 = amount.parse()?;
        Ok(whole * 1_000_000_000_000_000_000u128)
    }
}
