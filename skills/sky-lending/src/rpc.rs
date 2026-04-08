/// ABI encoding/decoding helpers for Sky Lending (MakerDAO CDP)

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

/// Encode a bytes32 value (already a 64-char hex string) as a 32-byte word.
/// Input: 64-char hex string without 0x prefix. Returns the same, zero-padded to 64 chars.
pub fn encode_bytes32(b32: &str) -> String {
    let b32 = b32.trim_start_matches("0x");
    format!("{:0<64}", b32)
}

/// Encode an int256 as a 32-byte two's complement hex word (no 0x prefix).
/// Positive values: same as uint256. Negative: two's complement.
pub fn encode_int256(val: i128) -> String {
    if val >= 0 {
        format!("{:064x}", val as u128)
    } else {
        // Two's complement: flip bits and add 1 for 256-bit representation
        let abs_val = (-val) as u128;
        let twos_comp = (!abs_val).wrapping_add(1);
        // Fill upper bits with f (sign extension)
        format!("ffffffffffffffffffffffffffffffff{:032x}", twos_comp)
    }
}

/// Build calldata for a single uint256 param.
pub fn calldata_single_uint256(selector: &str, val: u64) -> String {
    format!("0x{}{}", selector, encode_uint256_u64(val))
}

/// Build calldata for a single address param.
pub fn calldata_single_address(selector: &str, addr: &str) -> String {
    format!("0x{}{}", selector, encode_address(addr))
}

/// Build calldata for a single bytes32 param.
#[allow(dead_code)]
pub fn calldata_single_bytes32(selector: &str, b32: &str) -> String {
    format!("0x{}{}", selector, encode_bytes32(b32))
}

/// Build calldata for Vat.urns(bytes32 ilk, address urn).
pub fn calldata_vat_urns(ilk_hex: &str, urn: &str) -> String {
    format!(
        "0x{}{}{}",
        crate::config::SEL_VAT_URNS,
        encode_bytes32(ilk_hex),
        encode_address(urn)
    )
}

/// Build calldata for Vat.ilks(bytes32 ilk).
pub fn calldata_vat_ilks(ilk_hex: &str) -> String {
    format!(
        "0x{}{}",
        crate::config::SEL_VAT_ILKS,
        encode_bytes32(ilk_hex)
    )
}

/// Build calldata for Jug.ilks(bytes32 ilk).
pub fn calldata_jug_ilks(ilk_hex: &str) -> String {
    format!(
        "0x{}{}",
        crate::config::SEL_JUG_ILKS,
        encode_bytes32(ilk_hex)
    )
}

/// Build calldata for DssCdpManager.open(bytes32 ilk, address usr).
pub fn calldata_cdp_open(ilk_hex: &str, usr: &str) -> String {
    format!(
        "0x{}{}{}",
        crate::config::SEL_CDP_OPEN,
        encode_bytes32(ilk_hex),
        encode_address(usr)
    )
}

/// Build calldata for EthJoin.join(address usr) — payable.
pub fn calldata_ethjoin_join(usr: &str) -> String {
    format!("0x{}{}", crate::config::SEL_ETHJOIN_JOIN, encode_address(usr))
}

/// Build calldata for EthJoin.exit(address usr, uint256 wad).
pub fn calldata_ethjoin_exit(usr: &str, wad: u128) -> String {
    format!(
        "0x{}{}{}",
        crate::config::SEL_ETHJOIN_EXIT,
        encode_address(usr),
        encode_uint256_u128(wad)
    )
}

/// Build calldata for DaiJoin.exit(address usr, uint256 wad).
pub fn calldata_daijoin_exit(usr: &str, wad: u128) -> String {
    format!(
        "0x{}{}{}",
        crate::config::SEL_DAIJOIN_EXIT,
        encode_address(usr),
        encode_uint256_u128(wad)
    )
}

/// Build calldata for DaiJoin.join(address usr, uint256 wad).
pub fn calldata_daijoin_join(usr: &str, wad: u128) -> String {
    format!(
        "0x{}{}{}",
        crate::config::SEL_DAIJOIN_JOIN,
        encode_address(usr),
        encode_uint256_u128(wad)
    )
}

/// Build calldata for Vat.frob(bytes32 ilk, address u, address v, address w, int256 dink, int256 dart).
pub fn calldata_vat_frob(
    ilk_hex: &str,
    u: &str,
    v: &str,
    w: &str,
    dink: i128,
    dart: i128,
) -> String {
    format!(
        "0x{}{}{}{}{}{}{}",
        crate::config::SEL_VAT_FROB,
        encode_bytes32(ilk_hex),
        encode_address(u),
        encode_address(v),
        encode_address(w),
        encode_int256(dink),
        encode_int256(dart)
    )
}

/// Build calldata for DAI.approve(address spender, uint256 amount).
pub fn calldata_approve(spender: &str, amount: u128) -> String {
    format!(
        "0x{}{}{}",
        crate::config::SEL_APPROVE,
        encode_address(spender),
        encode_uint256_u128(amount)
    )
}

/// Decode a single uint256 from ABI-encoded return data (32-byte hex string, optional 0x prefix).
/// Values that exceed u128::MAX are saturated to u128::MAX.
pub fn decode_uint256(hex: &str) -> anyhow::Result<u128> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 64 {
        anyhow::bail!("Return data too short for uint256: '{}'", hex);
    }
    let word = &hex[hex.len() - 64..];
    // Check if upper 16 bytes are zero (fits in u128)
    let upper = &word[..32];
    if upper != "00000000000000000000000000000000" {
        // Value exceeds u128; return saturated max
        return Ok(u128::MAX);
    }
    let lower = &word[32..];
    Ok(u128::from_str_radix(lower, 16)?)
}

/// Decode a single uint256 as f64 (for large values like RAD that exceed u128).
pub fn decode_uint256_f64(hex: &str) -> anyhow::Result<f64> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 64 {
        anyhow::bail!("Return data too short for uint256: '{}'", hex);
    }
    let word = &hex[hex.len() - 64..];
    // Split 256-bit word into four 64-bit chunks and reconstruct as f64
    let chunk_a = u64::from_str_radix(&word[0..16], 16)? as f64;
    let chunk_b = u64::from_str_radix(&word[16..32], 16)? as f64;
    let chunk_c = u64::from_str_radix(&word[32..48], 16)? as f64;
    let chunk_d = u64::from_str_radix(&word[48..64], 16)? as f64;
    // value = a*2^192 + b*2^128 + c*2^64 + d
    let pow64 = 1.8446744073709552e19_f64;  // 2^64
    let pow128 = pow64 * pow64;
    let pow192 = pow128 * pow64;
    Ok(chunk_a * pow192 + chunk_b * pow128 + chunk_c * pow64 + chunk_d)
}

/// Decode a single address (last 20 bytes) from ABI return data.
pub fn decode_address(hex: &str) -> anyhow::Result<String> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 64 {
        anyhow::bail!("Return data too short for address: '{}'", hex);
    }
    let word = &hex[hex.len() - 64..];
    let addr = &word[24..]; // last 20 bytes = 40 hex chars
    Ok(format!("0x{}", addr))
}

/// Decode bytes32 from return data — return as hex string without 0x.
pub fn decode_bytes32(hex: &str) -> anyhow::Result<String> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 64 {
        anyhow::bail!("Return data too short for bytes32: '{}'", hex);
    }
    Ok(hex[..64].to_string())
}

/// Decode two uint256 values from ABI return data (e.g. Vat.urns returns (ink, art)).
/// Values that exceed u128::MAX are saturated.
pub fn decode_two_uint256(hex: &str) -> anyhow::Result<(u128, u128)> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 128 {
        anyhow::bail!("Return data too short for two uint256: '{}'", hex);
    }
    let a = decode_uint256(&hex[0..64])?;
    let b = decode_uint256(&hex[64..128])?;
    Ok((a, b))
}

/// Decode five uint256 values from ABI return data as f64 (for RAY/RAD scale values).
/// e.g. Vat.ilks returns (Art, rate, spot, line, dust) — line and dust are in RAD and may exceed u128.
pub fn decode_five_uint256_f64(hex: &str) -> anyhow::Result<(f64, f64, f64, f64, f64)> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 320 {
        anyhow::bail!("Return data too short for five uint256: len={} data='{}'", hex.len(), &hex[..hex.len().min(40)]);
    }
    let a = decode_uint256_f64(&hex[0..64])?;
    let b = decode_uint256_f64(&hex[64..128])?;
    let c = decode_uint256_f64(&hex[128..192])?;
    let d = decode_uint256_f64(&hex[192..256])?;
    let e = decode_uint256_f64(&hex[256..320])?;
    Ok((a, b, c, d, e))
}

/// Decode five uint256 values from ABI return data (e.g. Vat.ilks returns (Art,rate,spot,line,dust)).
/// Values that exceed u128::MAX are saturated to u128::MAX.
pub fn decode_five_uint256(hex: &str) -> anyhow::Result<(u128, u128, u128, u128, u128)> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 320 {
        anyhow::bail!("Return data too short for five uint256: '{}'", hex);
    }
    let a = decode_uint256(&hex[0..64])?;
    let b = decode_uint256(&hex[64..128])?;
    let c = decode_uint256(&hex[128..192])?;
    let d = decode_uint256(&hex[192..256])?;
    let e = decode_uint256(&hex[256..320])?;
    Ok((a, b, c, d, e))
}

/// Decode two uint256 values from Jug.ilks() return (duty, rho).
pub fn decode_jug_ilks(hex: &str) -> anyhow::Result<(u128, u128)> {
    decode_two_uint256(hex)
}

/// Convert a bytes32 hex string to a human-readable ilk name (strip null bytes).
pub fn bytes32_to_str(b32: &str) -> String {
    let b32 = b32.trim_start_matches("0x");
    let mut result = String::new();
    for i in (0..b32.len()).step_by(2) {
        if i + 2 <= b32.len() {
            if let Ok(byte) = u8::from_str_radix(&b32[i..i + 2], 16) {
                if byte != 0 {
                    result.push(byte as char);
                }
            }
        }
    }
    result
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
