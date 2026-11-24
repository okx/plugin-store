use tiny_keccak::{Hasher, Keccak};

/// Compute the 4-byte function selector from a canonical signature string.
pub fn selector(sig: &str) -> [u8; 4] {
    let mut k = Keccak::v256();
    k.update(sig.as_bytes());
    let mut out = [0u8; 32];
    k.finalize(&mut out);
    [out[0], out[1], out[2], out[3]]
}

/// Encode an Ethereum address as 32 bytes (left-padded).
pub fn encode_address(addr: &str) -> [u8; 32] {
    let addr = addr.trim_start_matches("0x").trim_start_matches("0X");
    let bytes = hex::decode(addr).unwrap_or_default();
    let mut out = [0u8; 32];
    let start = 32 - bytes.len().min(20);
    out[start..start + bytes.len().min(20)].copy_from_slice(&bytes[..bytes.len().min(20)]);
    out
}

/// Encode a u128 as a 32-byte big-endian uint256.
pub fn encode_uint256(n: u128) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[16..].copy_from_slice(&n.to_be_bytes());
    out
}

/// Encode a signed integer (i128) as a 32-byte ABI int256 (two's complement).
/// Positive values are zero-padded; negative values are 0xFF-padded (sign extension).
pub fn encode_int256(n: i128) -> [u8; 32] {
    let mut out = [0u8; 32];
    if n >= 0 {
        out[16..].copy_from_slice(&(n as u128).to_be_bytes());
    } else {
        out[..16].fill(0xff); // sign-extend upper 16 bytes
        // lower 16 bytes: two's complement of abs(n) in 128-bit space
        let abs_val = n.unsigned_abs();
        let twos = u128::MAX.wrapping_sub(abs_val).wrapping_add(1);
        out[16..].copy_from_slice(&twos.to_be_bytes());
    }
    out
}

/// Encode a dynamic uint256[] array: offset(0x20) + length + elements.
pub fn encode_uint256_array(ids: &[u64]) -> Vec<u8> {
    let mut out = Vec::new();
    // offset word = 0x20 (this is used when the array is the sole argument)
    let mut offset = [0u8; 32];
    offset[31] = 0x20;
    out.extend_from_slice(&offset);
    // length
    out.extend_from_slice(&encode_uint256(ids.len() as u128));
    // elements
    for id in ids {
        out.extend_from_slice(&encode_uint256(*id as u128));
    }
    out
}

/// Encode a dynamic address[] array: offset(0x20) + length + elements.
pub fn encode_address_array(addrs: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut offset = [0u8; 32];
    offset[31] = 0x20;
    out.extend_from_slice(&offset);
    out.extend_from_slice(&encode_uint256(addrs.len() as u128));
    for addr in addrs {
        out.extend_from_slice(&encode_address(addr));
    }
    out
}

/// Build calldata: 4-byte selector + raw payload bytes.
pub fn calldata_raw(sel: [u8; 4], payload: &[u8]) -> String {
    let mut bytes = sel.to_vec();
    bytes.extend_from_slice(payload);
    format!("0x{}", hex::encode(bytes))
}

/// Build calldata: selector + concatenated 32-byte slots.
pub fn calldata(sel: [u8; 4], slots: &[[u8; 32]]) -> String {
    let mut bytes = sel.to_vec();
    for slot in slots {
        bytes.extend_from_slice(slot);
    }
    format!("0x{}", hex::encode(bytes))
}

/// Decode a 32-byte word at slot index from hex result string (no 0x prefix).
pub fn word(data: &str, slot: usize) -> Option<[u8; 32]> {
    let start = slot * 64;
    if data.len() < start + 64 {
        return None;
    }
    let mut out = [0u8; 32];
    hex::decode_to_slice(&data[start..start + 64], &mut out).ok()?;
    Some(out)
}

/// Decode an address from a 32-byte word.
pub fn word_to_address(w: &[u8; 32]) -> String {
    format!("0x{}", hex::encode(&w[12..]))
}

/// Decode a u128 from the lower 16 bytes of a 32-byte word.
pub fn word_to_u128(w: &[u8; 32]) -> u128 {
    u128::from_be_bytes(w[16..].try_into().unwrap_or([0u8; 16]))
}

/// Decode a bool from a 32-byte word.
pub fn word_to_bool(w: &[u8; 32]) -> bool {
    w[31] != 0
}

/// Decode a signed integer from a 32-byte ABI int256 word.
/// Handles values that fit within i128 range (suitable for Fluid rate fields).
pub fn word_to_i128(w: &[u8; 32]) -> i128 {
    let low = u128::from_be_bytes(w[16..].try_into().unwrap_or([0u8; 16]));
    low as i128
}

/// Decode a UTF-8 ABI-encoded string result (offset+length+bytes layout).
pub fn decode_string(result: &str) -> String {
    let data = result.trim_start_matches("0x");
    if data.len() < 128 {
        return String::new();
    }
    let offset = usize::from_str_radix(&data[0..64], 16).unwrap_or(0) * 2;
    if data.len() < offset + 64 {
        return String::new();
    }
    let length = usize::from_str_radix(&data[offset..offset + 64], 16).unwrap_or(0);
    let char_start = offset + 64;
    if data.len() < char_start + length * 2 {
        return String::new();
    }
    hex::decode(&data[char_start..char_start + length * 2])
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_default()
}

/// Decode a uint8 from a 32-byte word (e.g. decimals()).
pub fn word_to_u8(w: &[u8; 32]) -> u8 {
    w[31]
}

/// Format a raw token amount with the given decimals to a human-readable string.
pub fn format_amount(raw: u128, decimals: u8) -> String {
    let scale = 10u128.pow(decimals as u32);
    let whole = raw / scale;
    let frac = raw % scale;
    if frac == 0 {
        whole.to_string()
    } else {
        let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, trimmed)
    }
}

/// Parse a human-readable amount string to raw u128 with given decimals.
pub fn parse_amount(s: &str, decimals: u8) -> anyhow::Result<u128> {
    if s == "0" || s.is_empty() {
        anyhow::bail!("Amount must be greater than 0");
    }
    let (whole, frac) = if let Some(dot) = s.find('.') {
        let w: u128 = s[..dot].parse()
            .map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?;
        let frac_str = &s[dot + 1..];
        if frac_str.len() > decimals as usize {
            anyhow::bail!("Amount '{}' has {} decimal places but token supports only {}", s, frac_str.len(), decimals);
        }
        let padded = format!("{:0<width$}", frac_str, width = decimals as usize);
        let f: u128 = padded.parse()
            .map_err(|_| anyhow::anyhow!("Invalid fractional amount: '{}'", s))?;
        (w, f)
    } else {
        let w: u128 = s.parse()
            .map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?;
        (w, 0u128)
    };
    let scale = 10u128.pow(decimals as u32);
    Ok(whole * scale + frac)
}

/// Validate that a string is a well-formed 20-byte Ethereum address (0x + 40 hex chars).
pub fn validate_address(addr: &str, field: &str) -> anyhow::Result<()> {
    let stripped = addr.strip_prefix("0x").or_else(|| addr.strip_prefix("0X"))
        .ok_or_else(|| anyhow::anyhow!("{} '{}' must start with 0x", field, addr))?;
    if stripped.len() != 40 {
        anyhow::bail!(
            "{} '{}' is not a valid Ethereum address (expected 0x + 40 hex chars, got {} chars after 0x).\n\
             Use `fluid vaults` to browse valid vault addresses.",
            field, addr, stripped.len()
        );
    }
    if !stripped.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!("{} '{}' contains non-hex characters", field, addr);
    }
    Ok(())
}
