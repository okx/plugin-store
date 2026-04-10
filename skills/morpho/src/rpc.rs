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
/// Returns raw u128 balance.
pub async fn erc20_balance_of(
    token: &str,
    owner: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    // balanceOf(address) selector = 0x70a08231
    let owner_clean = owner.trim_start_matches("0x");
    let data = format!("0x70a08231{:0>64}", owner_clean);
    let hex = eth_call(token, &data, rpc_url).await?;
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() || hex_clean == "0" {
        return Ok(0);
    }
    let padded = format!("{:0>64}", hex_clean);
    let val = u128::from_str_radix(&padded[padded.len() - 32..], 16)?;
    Ok(val)
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
    let val = u8::from_str_radix(&padded[padded.len() - 2..], 16).unwrap_or(18);
    Ok(val)
}

/// Read ERC-20 symbol.
/// Handles both dynamic string ABI encoding (ERC-20 standard) and bytes32 encoding
/// used by older tokens (USDC, USDT deployed pre-ERC-20 string standard).
pub async fn erc20_symbol(token: &str, rpc_url: &str) -> anyhow::Result<String> {
    // symbol() selector = 0x95d89b41
    let hex = eth_call(token, "0x95d89b41", rpc_url).await?;
    let hex_clean = hex.trim_start_matches("0x");

    // bytes32 encoding: exactly 64 hex chars (32 bytes), null-padded ASCII
    // Used by USDC, USDT, and other tokens deployed before the string standard
    if hex_clean.len() == 64 {
        let bytes = hex::decode(hex_clean).unwrap_or_default();
        let trimmed: Vec<u8> = bytes.into_iter().take_while(|&b| b != 0).collect();
        if !trimmed.is_empty() {
            return Ok(String::from_utf8_lossy(&trimmed).to_string());
        }
        return Ok("UNKNOWN".to_string());
    }

    // Dynamic string ABI encoding: offset(32) + length(32) + data
    // Each ABI slot is 32 bytes = 64 hex chars
    // [0..64]   = offset pointer (always 0x20)
    // [64..128] = string byte length (32-byte slot)
    // [128..]   = string data (padded to 32-byte boundary)
    if hex_clean.len() < 128 {
        return Ok("UNKNOWN".to_string());
    }
    let len_hex = &hex_clean[64..128];
    let len = usize::from_str_radix(len_hex, 16).unwrap_or(0);
    if len == 0 || hex_clean.len() < 128 + len * 2 {
        return Ok("UNKNOWN".to_string());
    }
    let data_hex = &hex_clean[128..128 + len * 2];
    let bytes = hex::decode(data_hex).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Read vault share balance (ERC-20 balanceOf, same encoding).
pub async fn vault_share_balance(
    vault: &str,
    owner: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    erc20_balance_of(vault, owner, rpc_url).await
}

/// convertToAssets(shares) on ERC-4626 vault.
pub async fn vault_convert_to_assets(
    vault: &str,
    shares: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    // convertToAssets(uint256) selector = 0x07a2d13a
    let shares_hex = format!("{:064x}", shares);
    let data = format!("0x07a2d13a{}", shares_hex);
    let hex = eth_call(vault, &data, rpc_url).await?;
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() {
        return Ok(0);
    }
    let padded = format!("{:0>64}", hex_clean);
    let val = u128::from_str_radix(&padded[padded.len() - 32..], 16)?;
    Ok(val)
}
