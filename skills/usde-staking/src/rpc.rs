/// ABI encoding/decoding helpers

/// Pad an address (with or without 0x) to 32-byte zero-padded hex word (no 0x prefix).
pub fn encode_address(addr: &str) -> String {
    let addr = addr.trim_start_matches("0x").trim_start_matches("0X");
    format!("{:0>64}", addr)
}

/// Encode a u128 value as a 32-byte big-endian hex word (no 0x prefix).
pub fn encode_uint256(val: u128) -> String {
    format!("{:064x}", val)
}

/// Build calldata for a single uint256 parameter function.
pub fn calldata_uint256(selector: &str, val: u128) -> String {
    format!("0x{}{}", selector, encode_uint256(val))
}

/// Build calldata for a single address parameter function.
pub fn calldata_address(selector: &str, addr: &str) -> String {
    format!("0x{}{}", selector, encode_address(addr))
}

/// Build calldata for approve(address spender, uint256 amount).
pub fn calldata_approve(spender: &str, amount: u128) -> String {
    format!(
        "0x{}{}{}",
        crate::config::SEL_APPROVE,
        encode_address(spender),
        encode_uint256(amount)
    )
}

/// Build calldata for ERC-4626 deposit(uint256 assets, address receiver).
pub fn calldata_deposit(assets: u128, receiver: &str) -> String {
    format!(
        "0x{}{}{}",
        crate::config::SEL_DEPOSIT,
        encode_uint256(assets),
        encode_address(receiver)
    )
}

/// Build calldata for cooldownShares(uint256 shares).
pub fn calldata_cooldown_shares(shares: u128) -> String {
    calldata_uint256(crate::config::SEL_COOLDOWN_SHARES, shares)
}

/// Build calldata for cooldownAssets(uint256 assets).
pub fn calldata_cooldown_assets(assets: u128) -> String {
    calldata_uint256(crate::config::SEL_COOLDOWN_ASSETS, assets)
}

/// Build calldata for unstake(address receiver).
pub fn calldata_unstake(receiver: &str) -> String {
    calldata_address(crate::config::SEL_UNSTAKE, receiver)
}

/// Build calldata for balanceOf(address).
pub fn calldata_balance_of(addr: &str) -> String {
    calldata_address(crate::config::SEL_BALANCE_OF, addr)
}

/// Build calldata for convertToAssets(uint256 shares).
pub fn calldata_convert_to_assets(shares: u128) -> String {
    calldata_uint256(crate::config::SEL_CONVERT_TO_ASSETS, shares)
}

/// Build calldata for previewDeposit(uint256 assets).
pub fn calldata_preview_deposit(assets: u128) -> String {
    calldata_uint256(crate::config::SEL_PREVIEW_DEPOSIT, assets)
}

/// Build calldata for cooldowns(address).
pub fn calldata_cooldowns(addr: &str) -> String {
    calldata_address(crate::config::SEL_COOLDOWNS, addr)
}

/// Build calldata for totalAssets().
pub fn calldata_total_assets() -> String {
    format!("0x{}", crate::config::SEL_TOTAL_ASSETS)
}

/// Build calldata for cooldownDuration().
pub fn calldata_cooldown_duration() -> String {
    format!("0x{}", crate::config::SEL_COOLDOWN_DURATION)
}

/// Decode a uint256 from ABI-encoded return data.
pub fn decode_uint256(hex: &str) -> anyhow::Result<u128> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.is_empty() || hex == "0" {
        return Ok(0);
    }
    if hex.len() < 64 {
        // Pad short response
        return Ok(u128::from_str_radix(hex, 16)?);
    }
    let word = &hex[hex.len() - 64..];
    Ok(u128::from_str_radix(word, 16)?)
}

/// Decode two consecutive uint256 values from ABI-encoded return data.
/// Returns (first, second).
pub fn decode_two_uint256(hex: &str) -> anyhow::Result<(u128, u128)> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 128 {
        return Ok((0, 0));
    }
    let first = u128::from_str_radix(&hex[0..64], 16)?;
    let second = u128::from_str_radix(&hex[64..128], 16)?;
    Ok((first, second))
}

/// Extract return data string from onchainos/eth_call response.
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

/// Convert a raw u128 token amount (18 decimals) to a human-readable f64.
pub fn wei_to_float(amount: u128) -> f64 {
    amount as f64 / 1e18
}

/// Convert a human-readable float to u128 token amount (18 decimals).
pub fn float_to_wei(amount: f64) -> u128 {
    (amount * 1e18) as u128
}
