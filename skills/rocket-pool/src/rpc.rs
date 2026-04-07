/// ABI encoding / decoding helpers — hand-rolled to avoid heavy alloy dependency

/// Pad a hex address (with or without 0x) to a 32-byte (64 hex char) left-zero-padded word.
pub fn encode_address(addr: &str) -> String {
    let addr = addr.trim_start_matches("0x").trim_start_matches("0X");
    format!("{:0>64}", addr)
}

/// Encode a u128 as a 32-byte big-endian hex word (no 0x prefix).
pub fn encode_uint256_u128(val: u128) -> String {
    format!("{:064x}", val)
}

/// Decode a single uint256 from ABI-encoded return data (32-byte hex string, optional 0x prefix).
pub fn decode_uint256(hex: &str) -> anyhow::Result<u128> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 64 {
        anyhow::bail!("Return data too short for uint256: '{}'", hex);
    }
    // Take the last 32 bytes (64 hex chars) — handles both 32-byte and longer returns
    let word = &hex[hex.len() - 64..];
    Ok(u128::from_str_radix(word, 16)?)
}

/// Extract address from ABI-encoded return data (20-byte address padded to 32 bytes).
pub fn decode_address(hex: &str) -> anyhow::Result<String> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 64 {
        anyhow::bail!("Return data too short for address: '{}'", hex);
    }
    // Address is the last 20 bytes of a 32-byte word (chars 24..64)
    let word = &hex[hex.len() - 64..];
    let addr = &word[24..]; // skip 12 bytes (24 hex chars) of padding
    Ok(format!("0x{}", addr))
}

/// Extract the raw hex return value from an onchainos/RPC response.
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
