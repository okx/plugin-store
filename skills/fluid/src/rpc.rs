use anyhow::Context;

/// Make a raw eth_call via JSON-RPC.
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
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
        .context("Missing result field in RPC response")?
        .to_string();
    Ok(result)
}

/// Read ERC-20 balance of `owner` at `token`.
pub async fn erc20_balance_of(token: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    // balanceOf(address) selector = 0x70a08231
    let owner_clean = owner.trim_start_matches("0x");
    let data = format!("0x70a08231{:0>64}", owner_clean);
    let hex = eth_call(token, &data, rpc_url).await?;
    parse_u128_from_hex(&hex)
}

/// Read ERC-20 decimals.
pub async fn erc20_decimals(token: &str, rpc_url: &str) -> anyhow::Result<u8> {
    // decimals() selector = 0x313ce567
    let hex = eth_call(token, "0x313ce567", rpc_url).await?;
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() {
        return Ok(18);
    }
    let padded = format!("{:0>64}", hex_clean);
    let val = u8::from_str_radix(&padded[padded.len().saturating_sub(2)..], 16).unwrap_or(18);
    Ok(val)
}

/// Read ERC-20 symbol.
pub async fn erc20_symbol(token: &str, rpc_url: &str) -> anyhow::Result<String> {
    // symbol() selector = 0x95d89b41
    let hex = eth_call(token, "0x95d89b41", rpc_url).await?;
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.len() < 128 {
        return Ok("UNKNOWN".to_string());
    }
    let len_hex = &hex_clean[64..96];
    let len = usize::from_str_radix(len_hex, 16).unwrap_or(0);
    if len == 0 || hex_clean.len() < 128 + len * 2 {
        return Ok("UNKNOWN".to_string());
    }
    let data_hex = &hex_clean[96..96 + len * 2];
    let bytes = hex::decode(data_hex).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Read fToken share balance (ERC-20 balanceOf).
pub async fn ftoken_share_balance(ftoken: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    erc20_balance_of(ftoken, owner, rpc_url).await
}

/// convertToAssets(uint256 shares) on ERC-4626 fToken.
/// Selector: 0x07a2d13a
pub async fn ftoken_convert_to_assets(ftoken: &str, shares: u128, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x07a2d13a{:064x}", shares);
    let hex = eth_call(ftoken, &data, rpc_url).await?;
    parse_u128_from_hex(&hex)
}

/// Parse a u128 from a hex string returned by eth_call.
pub fn parse_u128_from_hex(hex: &str) -> anyhow::Result<u128> {
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() || hex_clean == "0" {
        return Ok(0);
    }
    let padded = format!("{:0>64}", hex_clean);
    // Take last 32 hex chars (16 bytes = u128)
    let tail = &padded[padded.len().saturating_sub(32)..];
    Ok(u128::from_str_radix(tail, 16)?)
}

/// Parse a u256 (as hex) — return as a string since u256 > u128 possible
#[allow(dead_code)]
pub fn parse_u256_as_string(hex: &str) -> String {
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() {
        return "0".to_string();
    }
    // Try as u128 first
    if let Ok(v) = u128::from_str_radix(hex_clean, 16) {
        return v.to_string();
    }
    format!("0x{}", hex_clean)
}
