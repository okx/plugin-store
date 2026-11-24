/// ABI encoding helpers — hand-rolled to avoid heavy alloy dependency

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
pub fn encode_uint256_u64(val: u64) -> String {
    format!("{:064x}", val)
}

/// Encode a dynamic uint256[] array.
/// Returns the ABI tail (length word + element words), no selector, no offset word.
pub fn encode_uint256_array(values: &[u128]) -> String {
    let mut out = encode_uint256_u128(values.len() as u128);
    for v in values {
        out.push_str(&encode_uint256_u128(*v));
    }
    out
}

/// Build calldata for balanceOf(address) / sharesOf(address) — single address param.
pub fn calldata_single_address(selector: &str, addr: &str) -> String {
    format!("0x{}{}", selector, encode_address(addr))
}

/// Build calldata for `requestWithdrawals(uint256[] _amounts, address _owner)`.
/// Layout: selector | offset_to_amounts(0x40) | owner | amounts_length | amounts[0..]
pub fn calldata_request_withdrawals(amounts: &[u128], owner: &str) -> String {
    let offset = encode_uint256_u128(0x40); // offset to _amounts = 64 bytes
    let owner_word = encode_address(owner);
    let arr = encode_uint256_array(amounts);
    format!("0xd6681042{}{}{}", offset, owner_word, arr)
}

/// Build calldata for `getWithdrawalStatus(uint256[] requestIds)`.
pub fn calldata_get_withdrawal_status(ids: &[u128]) -> String {
    let offset = encode_uint256_u128(0x20); // single dynamic param starts at 0x20
    let arr = encode_uint256_array(ids);
    format!("0xb8c4b85a{}{}", offset, arr)
}

/// Build calldata for `getWithdrawalRequests(address owner)`.
pub fn calldata_get_withdrawal_requests(addr: &str) -> String {
    format!("0x7d031b65{}", encode_address(addr))
}

/// Build calldata for `findCheckpointHints(uint256[] requestIds, uint256 firstIndex, uint256 lastIndex)`.
pub fn calldata_find_checkpoint_hints(ids: &[u128], first: u64, last: u64) -> String {
    // Layout: selector | offset_to_ids(0x60) | firstIndex | lastIndex | ids_length | ids...
    let offset = encode_uint256_u128(0x60);
    let first_word = encode_uint256_u64(first);
    let last_word = encode_uint256_u64(last);
    let arr = encode_uint256_array(ids);
    format!("0x62abe3fa{}{}{}{}", offset, first_word, last_word, arr)
}

/// Build calldata for `claimWithdrawals(uint256[] requestIds, uint256[] hints)`.
pub fn calldata_claim_withdrawals(ids: &[u128], hints: &[u128]) -> String {
    // Two dynamic arrays. offsets: ids at 0x40, hints at 0x40 + 0x20 + ids.len()*0x20
    let ids_offset: u128 = 0x40;
    let hints_offset: u128 = 0x40 + 0x20 + (ids.len() as u128) * 0x20;
    let ids_offset_word = encode_uint256_u128(ids_offset);
    let hints_offset_word = encode_uint256_u128(hints_offset);
    let ids_arr = encode_uint256_array(ids);
    let hints_arr = encode_uint256_array(hints);
    format!(
        "0xe3afe0a3{}{}{}{}",
        ids_offset_word, hints_offset_word, ids_arr, hints_arr
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

/// Decode a single uint256 from ABI-encoded return data (32-byte hex string, optional 0x prefix).
pub fn decode_uint256(hex: &str) -> anyhow::Result<u128> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 64 {
        anyhow::bail!("Return data too short for uint256: '{}'", hex);
    }
    // Take the last 32 bytes (64 hex chars)
    let word = &hex[hex.len() - 64..];
    Ok(u128::from_str_radix(word, 16)?)
}

/// Decode a uint256[] from ABI-encoded return data.
/// Layout: offset(32) | length(32) | elements...
pub fn decode_uint256_array(hex: &str) -> anyhow::Result<Vec<u128>> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 128 {
        return Ok(vec![]);
    }
    // offset is first 64 chars, length is next 64 chars
    let len_word = &hex[64..128];
    let count = usize::from_str_radix(len_word, 16)?;
    let mut result = Vec::with_capacity(count);
    for i in 0..count {
        let start = 128 + i * 64;
        if start + 64 > hex.len() {
            break;
        }
        let word = &hex[start..start + 64];
        result.push(u128::from_str_radix(word, 16)?);
    }
    Ok(result)
}

/// Extract the raw hex return value from an onchainos response.
pub fn extract_return_data(result: &serde_json::Value) -> anyhow::Result<String> {
    // onchainos returns data.result or data.returnData or similar
    if let Some(s) = result["data"]["result"].as_str() {
        return Ok(s.to_string());
    }
    if let Some(s) = result["data"]["returnData"].as_str() {
        return Ok(s.to_string());
    }
    if let Some(s) = result["result"].as_str() {
        return Ok(s.to_string());
    }
    // Fallback: try to stringify data field
    anyhow::bail!("Could not extract return data from: {}", result)
}
