/// ABI encoding helpers

/// Pad a hex address (with or without 0x) to a 32-byte (64 hex char) left-zero-padded word.
pub fn encode_address(addr: &str) -> String {
    let addr = addr.trim_start_matches("0x").trim_start_matches("0X");
    format!("{:0>64}", addr)
}

/// Encode a u128 as a 32-byte big-endian hex word (no 0x prefix).
pub fn encode_uint256_u128(val: u128) -> String {
    format!("{:064x}", val)
}

/// Build calldata for `balanceOf(address)`.
pub fn calldata_balance_of(selector: &str, addr: &str) -> String {
    format!("0x{}{}", selector, encode_address(addr))
}

/// Build calldata for `allowance(address owner, address spender)`.
pub fn calldata_allowance(owner: &str, spender: &str) -> String {
    format!(
        "0xdd62ed3e{}{}",
        encode_address(owner),
        encode_address(spender)
    )
}

/// Build calldata for `approve(address spender, uint256 amount)`.
pub fn calldata_approve(spender: &str, amount: u128) -> String {
    format!(
        "0x095ea7b3{}{}",
        encode_address(spender),
        encode_uint256_u128(amount)
    )
}

/// Build calldata for `deposit(address _collateralToken, uint256 _amount)`.
pub fn calldata_deposit_token(token: &str, amount: u128) -> String {
    format!(
        "0x47e7ef24{}{}",
        encode_address(token),
        encode_uint256_u128(amount)
    )
}

/// Decode a single uint256 from ABI-encoded return data (hex string, optional 0x prefix).
pub fn decode_uint256(hex: &str) -> anyhow::Result<u128> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.is_empty() {
        anyhow::bail!("Empty return data");
    }
    if hex.len() < 64 {
        // Pad to at least 64 chars
        return Ok(u128::from_str_radix(hex, 16)?);
    }
    // Take the rightmost 32 bytes (64 hex chars) — handles leading padding
    let word = &hex[hex.len() - 64..];
    Ok(u128::from_str_radix(word, 16)?)
}

/// Extract the raw hex return value from an eth_call result.
pub fn extract_return_data(result: &serde_json::Value) -> anyhow::Result<String> {
    if let Some(s) = result["data"]["result"].as_str() {
        return Ok(s.to_string());
    }
    if let Some(s) = result["result"].as_str() {
        return Ok(s.to_string());
    }
    anyhow::bail!("Could not extract return data from: {}", result)
}
