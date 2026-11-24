/// Manual ABI encoding helpers.
/// All addresses are left-padded to 32 bytes.
/// All uint256/uint128 values are left-padded to 32 bytes.

fn pad_address(addr: &str) -> [u8; 32] {
    let addr = addr.trim_start_matches("0x");
    let bytes = hex::decode(addr).unwrap_or_default();
    let mut out = [0u8; 32];
    let start = 32 - bytes.len().min(20);
    out[start..].copy_from_slice(&bytes[bytes.len().saturating_sub(20)..]);
    out
}

fn pad_u256(val: u128) -> [u8; 32] {
    let mut out = [0u8; 32];
    let bytes = val.to_be_bytes(); // 16 bytes
    out[16..].copy_from_slice(&bytes);
    out
}

fn pad_u160(val: u128) -> [u8; 32] {
    // u160 fits in u128 for our use case (we only use 0)
    pad_u256(val)
}

/// encode approve(address spender, uint256 amount)
/// selector: 0x095ea7b3
pub fn encode_erc20_approve(spender: &str, amount: u128) -> String {
    let mut data = Vec::with_capacity(4 + 64);
    data.extend_from_slice(&[0x09, 0x5e, 0xa7, 0xb3]);
    data.extend_from_slice(&pad_address(spender));
    data.extend_from_slice(&pad_u256(amount));
    format!("0x{}", hex::encode(data))
}

/// encode allowance(address owner, address spender)
/// selector: 0xdd62ed3e
pub fn encode_allowance(owner: &str, spender: &str) -> String {
    let mut data = Vec::with_capacity(4 + 64);
    data.extend_from_slice(&[0xdd, 0x62, 0xed, 0x3e]);
    data.extend_from_slice(&pad_address(owner));
    data.extend_from_slice(&pad_address(spender));
    format!("0x{}", hex::encode(data))
}

/// encode balanceOf(address account)
/// selector: 0x70a08231
pub fn encode_balance_of(account: &str) -> String {
    let mut data = Vec::with_capacity(4 + 32);
    data.extend_from_slice(&[0x70, 0xa0, 0x82, 0x31]);
    data.extend_from_slice(&pad_address(account));
    format!("0x{}", hex::encode(data))
}

/// encode decimals()
/// selector: 0x313ce567
pub fn encode_decimals() -> String {
    "0x313ce567".to_string()
}

/// encode symbol()
/// selector: 0x95d89b41
pub fn encode_symbol() -> String {
    "0x95d89b41".to_string()
}

/// encode exactInputSingle((address tokenIn, address tokenOut, address recipient,
///   uint256 deadline, uint256 amountIn, uint256 amountOutMinimum, uint160 limitSqrtPrice))
/// selector: 0xbc651188
/// Algebra struct: tokenIn, tokenOut, recipient, deadline, amountIn, amountOutMinimum, limitSqrtPrice
pub fn encode_exact_input_single(
    token_in: &str,
    token_out: &str,
    recipient: &str,
    deadline: u64,
    amount_in: u128,
    amount_out_min: u128,
) -> String {
    let mut data = Vec::with_capacity(4 + 7 * 32);
    data.extend_from_slice(&[0xbc, 0x65, 0x11, 0x88]);
    data.extend_from_slice(&pad_address(token_in));
    data.extend_from_slice(&pad_address(token_out));
    data.extend_from_slice(&pad_address(recipient));
    data.extend_from_slice(&pad_u256(deadline as u128));
    data.extend_from_slice(&pad_u256(amount_in));
    data.extend_from_slice(&pad_u256(amount_out_min));
    data.extend_from_slice(&pad_u160(0)); // limitSqrtPrice = 0 (no price limit)
    format!("0x{}", hex::encode(data))
}

/// encode quoteExactInputSingle(address tokenIn, address tokenOut, uint256 amountIn, uint160 limitSqrtPrice)
/// selector: 0x2d9ebd1d
pub fn encode_quote_exact_input_single(
    token_in: &str,
    token_out: &str,
    amount_in: u128,
) -> String {
    let mut data = Vec::with_capacity(4 + 4 * 32);
    data.extend_from_slice(&[0x2d, 0x9e, 0xbd, 0x1d]);
    data.extend_from_slice(&pad_address(token_in));
    data.extend_from_slice(&pad_address(token_out));
    data.extend_from_slice(&pad_u256(amount_in));
    data.extend_from_slice(&pad_u160(0)); // limitSqrtPrice = 0
    format!("0x{}", hex::encode(data))
}

/// encode WMATIC deposit() — no params, selector: 0xd0e30db0
pub fn encode_wmatic_deposit() -> String {
    "0xd0e30db0".to_string()
}

/// Decode a hex result (0x-prefixed) as u128 from the first 32 bytes
pub fn decode_u128(hex_str: &str) -> anyhow::Result<u128> {
    let s = hex_str.trim_start_matches("0x");
    if s.is_empty() || s == "0" {
        return Ok(0);
    }
    // Take the last 32 bytes (64 hex chars) — right-aligned
    let padded = format!("{:0>64}", s);
    let last32 = &padded[padded.len().saturating_sub(64)..];
    let bytes = hex::decode(last32)
        .map_err(|e| anyhow::anyhow!("hex decode error: {}", e))?;
    // Read last 16 bytes as u128
    let slice = &bytes[bytes.len().saturating_sub(16)..];
    let mut arr = [0u8; 16];
    let copy_start = 16 - slice.len();
    arr[copy_start..].copy_from_slice(slice);
    Ok(u128::from_be_bytes(arr))
}

/// Decode a hex result as u8 (for decimals)
pub fn decode_u8(hex_str: &str) -> anyhow::Result<u8> {
    let val = decode_u128(hex_str)?;
    Ok(val as u8)
}

/// Decode a hex result as an ABI-encoded string (for symbol)
pub fn decode_string(hex_str: &str) -> anyhow::Result<String> {
    let s = hex_str.trim_start_matches("0x");
    if s.len() < 128 {
        // Fallback: try direct ASCII decode of last bytes
        if let Ok(bytes) = hex::decode(s) {
            let trimmed: Vec<u8> = bytes.into_iter().filter(|&b| b >= 0x20 && b <= 0x7e).collect();
            if !trimmed.is_empty() {
                return Ok(String::from_utf8_lossy(&trimmed).to_string());
            }
        }
        return Ok("UNKNOWN".to_string());
    }
    // ABI encoded string: offset (32 bytes) + length (32 bytes) + data
    let bytes = hex::decode(s).map_err(|e| anyhow::anyhow!("hex decode: {}", e))?;
    if bytes.len() < 64 {
        return Ok("UNKNOWN".to_string());
    }
    // length is at offset 32
    let len_bytes = &bytes[32..48];
    let mut len_arr = [0u8; 16];
    len_arr.copy_from_slice(len_bytes);
    let len = u128::from_be_bytes(len_arr) as usize;
    if bytes.len() < 64 + len {
        return Ok("UNKNOWN".to_string());
    }
    let data = &bytes[64..64 + len];
    Ok(String::from_utf8_lossy(data).to_string())
}
