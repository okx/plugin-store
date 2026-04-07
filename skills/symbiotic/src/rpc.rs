use anyhow::anyhow;

/// Execute an eth_call (read-only) against the given RPC.
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
        "id": 1
    });
    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    if let Some(err) = resp.get("error") {
        return Err(anyhow!("eth_call error: {}", err));
    }
    let hex_result = resp["result"].as_str().unwrap_or("0x").to_string();
    Ok(hex_result)
}

/// Decode a single uint256 (or address) from eth_call hex result.
pub fn decode_uint256(hex: &str) -> u128 {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 64 {
        return 0;
    }
    // Take last 32 bytes (64 hex chars) — handles padded results
    let tail = &clean[clean.len().saturating_sub(64)..];
    u128::from_str_radix(tail, 16).unwrap_or(0)
}

/// Decode an Ethereum address from the last 20 bytes of a 32-byte ABI-encoded result.
pub fn decode_address(hex: &str) -> String {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 40 {
        return "0x0000000000000000000000000000000000000000".to_string();
    }
    // Take last 40 hex chars (20 bytes = EVM address)
    let addr = &clean[clean.len().saturating_sub(40)..];
    format!("0x{}", addr)
}

/// Query active balance of a staker in a vault.
/// activeBalanceOf(address) selector: 0x59f769a9 [verified: cast sig "activeBalanceOf(address)"]
pub async fn active_balance_of(
    vault: &str,
    staker: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let staker_clean = staker.trim_start_matches("0x");
    let data = format!("0x59f769a9{:0>64}", staker_clean);
    let result = eth_call(vault, &data, rpc_url).await?;
    Ok(decode_uint256(&result))
}

/// Query the collateral token address of a vault.
/// collateral() selector: 0xd8dfeb45 [verified: cast sig "collateral()"]
pub async fn vault_collateral(vault: &str, rpc_url: &str) -> anyhow::Result<String> {
    let data = "0xd8dfeb45";
    let result = eth_call(vault, data, rpc_url).await?;
    Ok(decode_address(&result))
}

/// Query current epoch of a vault.
/// currentEpoch() selector: 0x76671808 [verified: cast sig "currentEpoch()"]
pub async fn current_epoch(vault: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(vault, "0x76671808", rpc_url).await?;
    Ok(decode_uint256(&result))
}

/// Query epoch duration of a vault (in seconds).
/// epochDuration() selector: 0x4ff0876a [verified: cast sig "epochDuration()"]
pub async fn epoch_duration(vault: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(vault, "0x4ff0876a", rpc_url).await?;
    Ok(decode_uint256(&result))
}

/// Query total stake in a vault.
/// totalStake() selector: 0x8b0e9f3f [verified: cast sig "totalStake()"]
pub async fn total_stake(vault: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(vault, "0x8b0e9f3f", rpc_url).await?;
    Ok(decode_uint256(&result))
}

/// Query withdrawals of a staker for a specific epoch.
/// withdrawalsOf(uint256,address) selector: 0xf5e7ee0f [verified: cast sig "withdrawalsOf(uint256,address)"]
pub async fn withdrawals_of(
    vault: &str,
    epoch: u128,
    staker: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let staker_clean = staker.trim_start_matches("0x");
    let epoch_hex = format!("{:064x}", epoch);
    let data = format!("0xf5e7ee0f{}{:0>64}", epoch_hex, staker_clean);
    let result = eth_call(vault, &data, rpc_url).await?;
    Ok(decode_uint256(&result))
}

/// Query ERC-20 token symbol via eth_call.
/// symbol() selector: 0x95d89b41
pub async fn token_symbol(token_addr: &str, rpc_url: &str) -> anyhow::Result<String> {
    let result = eth_call(token_addr, "0x95d89b41", rpc_url).await?;
    // ABI-decode string: skip first 64 bytes (offset), next 32 bytes = length, rest = utf8
    let clean = result.trim_start_matches("0x");
    if clean.len() < 128 {
        return Ok("???".to_string());
    }
    let length_hex = &clean[64..128];
    let length = usize::from_str_radix(length_hex, 16).unwrap_or(0).min(32);
    if clean.len() < 128 + length * 2 {
        return Ok("???".to_string());
    }
    let symbol_hex = &clean[128..128 + length * 2];
    let bytes = hex::decode(symbol_hex).unwrap_or_default();
    Ok(String::from_utf8(bytes).unwrap_or("???".to_string()).trim_matches('\0').to_string())
}

/// Query ERC-20 token decimals.
/// decimals() selector: 0x313ce567
pub async fn token_decimals(token_addr: &str, rpc_url: &str) -> anyhow::Result<u8> {
    let result = eth_call(token_addr, "0x313ce567", rpc_url).await?;
    let d = decode_uint256(&result) as u8;
    Ok(d)
}
