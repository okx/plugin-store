/// ABI encoding helpers for Kelp plugin — hand-rolled, no alloy dependency

/// Pad a hex address (with or without 0x) to a 32-byte (64 hex char) left-zero-padded word.
pub fn encode_address(addr: &str) -> String {
    let addr = addr.trim_start_matches("0x").trim_start_matches("0X");
    format!("{:0>64}", addr)
}

/// Encode a u128 as a 32-byte big-endian hex word (no 0x prefix).
pub fn encode_uint256_u128(val: u128) -> String {
    format!("{:064x}", val)
}

/// Encode a u64 as a 32-byte big-endian hex word (no 0x prefix).
#[allow(dead_code)]
pub fn encode_uint256_u64(val: u64) -> String {
    format!("{:064x}", val)
}

/// Build calldata for a single-address parameter function (e.g. balanceOf(address)).
pub fn calldata_single_address(selector: &str, addr: &str) -> String {
    format!("0x{}{}", selector, encode_address(addr))
}

/// Build calldata for a function with single uint256 parameter.
#[allow(dead_code)]
pub fn calldata_single_uint256(selector: &str, val: u128) -> String {
    format!("0x{}{}", selector, encode_uint256_u128(val))
}

/// Build calldata for rsETHPrice() — no parameters.
pub fn calldata_no_params(selector: &str) -> String {
    format!("0x{}", selector)
}

/// Build calldata for getRsETHAmountToMint(address asset, uint256 amount).
pub fn calldata_get_rseth_amount(asset_addr: &str, amount: u128) -> String {
    format!(
        "0x{}{}{}",
        crate::config::SEL_GET_RSETH_AMOUNT,
        encode_address(asset_addr),
        encode_uint256_u128(amount)
    )
}

/// Build calldata for getAssetCurrentPrice(address).
#[allow(dead_code)]
pub fn calldata_get_asset_price(asset_addr: &str) -> String {
    calldata_single_address(crate::config::SEL_ASSET_PRICE, asset_addr)
}

/// Build calldata for depositETH(uint256 minRSETHAmountExpected, string referralId).
/// Uses empty string for referralId and specified minRSETH (usually 0).
///
/// ABI layout:
///   selector (4 bytes)
///   minRSETHAmountExpected (32 bytes)
///   offset to string data (32 bytes) = 0x40
///   string length (32 bytes) = 0
pub fn calldata_deposit_eth(min_rseth: u128) -> String {
    let min_rseth_word = encode_uint256_u128(min_rseth);
    // offset to string = 64 bytes (2 * 32) from start of params
    let string_offset = encode_uint256_u128(0x40);
    // empty string: length = 0, no data words
    let string_length = encode_uint256_u128(0);
    format!(
        "0x{}{}{}{}",
        crate::config::SEL_DEPOSIT_ETH,
        min_rseth_word,
        string_offset,
        string_length
    )
}

/// Build calldata for initiateWithdrawal(address asset, uint256 rsEthAmount).
pub fn calldata_initiate_withdrawal(asset_addr: &str, rs_eth_amount: u128) -> String {
    format!(
        "0x{}{}{}",
        crate::config::SEL_INITIATE_WITHDRAWAL,
        encode_address(asset_addr),
        encode_uint256_u128(rs_eth_amount)
    )
}

/// Build calldata for completeWithdrawal(address asset).
#[allow(dead_code)]
pub fn calldata_complete_withdrawal(asset_addr: &str) -> String {
    calldata_single_address(crate::config::SEL_COMPLETE_WITHDRAWAL, asset_addr)
}

/// Decode a single uint256 from ABI-encoded return data (32-byte hex, optional 0x prefix).
pub fn decode_uint256(hex: &str) -> anyhow::Result<u128> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 64 {
        anyhow::bail!("Return data too short for uint256: '{}'", hex);
    }
    // Take the last 32 bytes (64 hex chars) of the first word
    let word = &hex[hex.len() - 64..];
    Ok(u128::from_str_radix(word, 16)?)
}

/// Extract the raw hex return value from an onchainos or eth_call response.
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
