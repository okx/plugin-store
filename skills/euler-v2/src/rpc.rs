use anyhow::Context;

/// Make a raw eth_call via JSON-RPC.
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": data },
            "latest"
        ],
        "id": 1
    });
    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .context("RPC request failed")?
        .json()
        .await
        .context("RPC response parse failed")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }
    let result = resp["result"]
        .as_str()
        .unwrap_or("0x")
        .to_string();
    Ok(result)
}

/// Read ERC-20 balance of `owner` at `token`.
pub async fn erc20_balance_of(token: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let owner_clean = owner.trim_start_matches("0x").to_lowercase();
    let data = format!("0x70a08231{:0>64}", owner_clean);
    let hex = eth_call(token, &data, rpc_url).await?;
    parse_u128_from_hex(&hex)
}

/// Read ERC-20 decimals.
pub async fn erc20_decimals(token: &str, rpc_url: &str) -> anyhow::Result<u8> {
    let hex = eth_call(token, "0x313ce567", rpc_url).await?;
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() {
        return Ok(18);
    }
    let padded = format!("{:0>64}", hex_clean);
    let val = u8::from_str_radix(&padded[padded.len().saturating_sub(2)..], 16).unwrap_or(18);
    Ok(val)
}

/// Read ERC-20 or vault symbol.
pub async fn erc20_symbol(token: &str, rpc_url: &str) -> anyhow::Result<String> {
    let hex = eth_call(token, "0x95d89b41", rpc_url).await?;
    decode_string_from_hex(&hex)
}

/// Read vault name.
#[allow(dead_code)]
pub async fn vault_name(vault: &str, rpc_url: &str) -> anyhow::Result<String> {
    let hex = eth_call(vault, "0x06fdde03", rpc_url).await?;
    decode_string_from_hex(&hex)
}

/// Get underlying asset address of an EVault.
/// asset() -> address  selector: 0x38d52e0f
pub async fn vault_asset(vault: &str, rpc_url: &str) -> anyhow::Result<String> {
    let hex = eth_call(vault, "0x38d52e0f", rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 40 {
        anyhow::bail!("Invalid asset() response: {}", hex);
    }
    Ok(format!("0x{}", &clean[clean.len() - 40..]))
}

/// totalAssets() -> uint256  selector: 0x01e1d114
pub async fn vault_total_assets(vault: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex = eth_call(vault, "0x01e1d114", rpc_url).await?;
    parse_u128_from_hex(&hex)
}

/// balanceOf(address) -> uint256 (shares)  selector: 0x70a08231
pub async fn vault_balance_of(vault: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    erc20_balance_of(vault, owner, rpc_url).await
}

/// debtOf(address) -> uint256  selector: 0xd283e75f
pub async fn vault_debt_of(vault: &str, account: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let acc_clean = account.trim_start_matches("0x").to_lowercase();
    let data = format!("0xd283e75f{:0>64}", acc_clean);
    let hex = eth_call(vault, &data, rpc_url).await?;
    parse_u128_from_hex(&hex)
}

/// convertToAssets(uint256 shares) -> uint256  selector: 0x07a2d13a
pub async fn vault_convert_to_assets(vault: &str, shares: u128, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x07a2d13a{:064x}", shares);
    let hex = eth_call(vault, &data, rpc_url).await?;
    parse_u128_from_hex(&hex)
}

/// interestRate() -> uint256 (per-second borrow rate in 1e27 ray)  selector: 0x7c3a00fd
pub async fn vault_interest_rate(vault: &str, rpc_url: &str) -> anyhow::Result<u128> {
    // interestRate() selector: keccak256("interestRate()")[0:4] = 0x7c3a00fd
    let hex = eth_call(vault, "0x7c3a00fd", rpc_url).await?;
    parse_u128_from_hex(&hex)
}

/// Get list of vault addresses from eVaultFactory.
/// getProxyListSlice(uint256 start, uint256 end) -> address[]
/// selector: 0xc0e96df6
pub async fn factory_get_vaults(factory: &str, start: u64, end: u64, rpc_url: &str) -> anyhow::Result<Vec<String>> {
    let data = format!(
        "0xc0e96df6{:064x}{:064x}",
        start, end
    );
    let hex = eth_call(factory, &data, rpc_url).await?;
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.len() < 128 {
        return Ok(vec![]);
    }

    // ABI decode dynamic address array:
    // [0..64]   offset to array (= 32 bytes = 0x20)
    // [64..128] array length
    // [128..]   array elements (each 32 bytes, right-aligned address)
    let offset = usize::from_str_radix(&hex_clean[0..64], 16).unwrap_or(32);
    let len_pos = offset * 2;
    if hex_clean.len() < len_pos + 64 {
        return Ok(vec![]);
    }
    let length = usize::from_str_radix(&hex_clean[len_pos..len_pos + 64], 16).unwrap_or(0);
    let mut addrs = Vec::with_capacity(length);
    for i in 0..length {
        let pos = len_pos + 64 + i * 64;
        if pos + 64 > hex_clean.len() {
            break;
        }
        let slot = &hex_clean[pos..pos + 64];
        addrs.push(format!("0x{}", &slot[24..]));
    }
    Ok(addrs)
}

/// Parse a u128 from a 32-byte hex eth_call result.
pub fn parse_u128_from_hex(hex: &str) -> anyhow::Result<u128> {
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() || hex_clean == "0" {
        return Ok(0);
    }
    let padded = format!("{:0>64}", hex_clean);
    let tail = &padded[padded.len().saturating_sub(32)..];
    Ok(u128::from_str_radix(tail, 16).unwrap_or(0))
}

/// Parse a 32-byte hex slot as an Ethereum address (right-aligned).
#[allow(dead_code)]
pub fn parse_address_from_hex(hex: &str) -> String {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 40 {
        return "0x0000000000000000000000000000000000000000".to_string();
    }
    format!("0x{}", &clean[clean.len() - 40..])
}

/// Decode ABI-encoded string from eth_call result.
fn decode_string_from_hex(hex: &str) -> anyhow::Result<String> {
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.len() < 128 {
        return Ok("UNKNOWN".to_string());
    }
    // offset (32 bytes), length (32 bytes), data
    let offset = usize::from_str_radix(&hex_clean[0..64], 16).unwrap_or(32);
    let len_pos = offset * 2;
    if hex_clean.len() < len_pos + 64 {
        return Ok("UNKNOWN".to_string());
    }
    let len = usize::from_str_radix(&hex_clean[len_pos..len_pos + 64], 16).unwrap_or(0);
    if len == 0 {
        return Ok("".to_string());
    }
    let data_start = len_pos + 64;
    let data_end = data_start + len * 2;
    if data_end > hex_clean.len() {
        return Ok("UNKNOWN".to_string());
    }
    let bytes = hex::decode(&hex_clean[data_start..data_end]).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Convert per-second borrow rate (1e27 ray) to APR percentage.
pub fn ray_to_apr_pct(ray: u128) -> f64 {
    // rate * seconds_per_year / 1e27
    let seconds_per_year: f64 = 365.0 * 24.0 * 3600.0;
    (ray as f64) * seconds_per_year / 1e27 * 100.0
}

/// Format a raw token amount to human-readable string.
pub fn format_amount(raw: u128, decimals: u8) -> String {
    if decimals == 0 {
        return raw.to_string();
    }
    let d = decimals as u32;
    let divisor = 10u128.pow(d);
    let whole = raw / divisor;
    let frac = raw % divisor;
    if frac == 0 {
        format!("{}", whole)
    } else {
        let frac_str = format!("{:0>width$}", frac, width = d as usize);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, trimmed)
    }
}

/// Parse human-readable amount string to raw u128.
pub fn parse_amount(s: &str, decimals: u8) -> anyhow::Result<u128> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("Empty amount string");
    }
    let d = decimals as u32;
    let multiplier = 10u128.pow(d);
    if let Some(dot_pos) = s.find('.') {
        let whole: u128 = s[..dot_pos].parse().context("Invalid whole part")?;
        let frac_str = &s[dot_pos + 1..];
        let frac_len = frac_str.len() as u32;
        let frac: u128 = frac_str.parse().context("Invalid fractional part")?;
        if frac_len > d {
            anyhow::bail!("Too many decimal places (max {})", d);
        }
        let frac_scaled = frac * 10u128.pow(d - frac_len);
        Ok(whole * multiplier + frac_scaled)
    } else {
        let whole: u128 = s.parse().context("Invalid integer amount")?;
        Ok(whole * multiplier)
    }
}
