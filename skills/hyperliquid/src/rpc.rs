/// Minimal JSON-RPC eth_call helpers for Arbitrum (read-only EVM queries).

pub const ARBITRUM_RPC: &str = "https://arbitrum-one-rpc.publicnode.com";

/// Pad a 20-byte Ethereum address to 32-byte ABI encoding.
pub fn pad_address(addr: &str) -> String {
    let a = addr.trim_start_matches("0x");
    format!("{:0>64}", a)
}

/// Pad a u128 value to 32-byte ABI encoding.
pub fn pad_u256(val: u128) -> String {
    format!("{:064x}", val)
}

/// eth_call helper: sends a JSON-RPC eth_call to the given RPC URL.
async fn eth_call(rpc: &str, to: &str, data: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"]
    });
    let resp: serde_json::Value = client
        .post(rpc)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    let result = resp["result"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("eth_call failed: {:?}", resp["error"]))?
        .to_string();
    Ok(result)
}

/// Query ERC-20 balanceOf(address) → u128 (token units).
pub async fn erc20_balance(token: &str, owner: &str, rpc: &str) -> anyhow::Result<u128> {
    // balanceOf(address) selector: 0x70a08231
    let data = format!("0x70a08231{}", pad_address(owner));
    let hex = eth_call(rpc, token, &data).await?;
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.is_empty() || trimmed == "0".repeat(trimmed.len()).as_str() {
        return Ok(0);
    }
    let val = u128::from_str_radix(&trimmed[trimmed.len().saturating_sub(32)..], 16)
        .unwrap_or(0);
    Ok(val)
}

/// Query ERC-2612 nonces(address) → u64 (permit nonce for signing).
pub async fn usdc_permit_nonce(token: &str, owner: &str, rpc: &str) -> anyhow::Result<u64> {
    // nonces(address) selector: 0x7ecebe00
    let data = format!("0x7ecebe00{}", pad_address(owner));
    let hex = eth_call(rpc, token, &data).await?;
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.is_empty() {
        return Ok(0);
    }
    let val = u64::from_str_radix(&trimmed[trimmed.len().saturating_sub(16)..], 16)
        .unwrap_or(0);
    Ok(val)
}

/// Query ERC-20 allowance(owner, spender) → u128.
pub async fn erc20_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc: &str,
) -> anyhow::Result<u128> {
    // allowance(address,address) selector: 0xdd62ed3e
    let data = format!("0xdd62ed3e{}{}", pad_address(owner), pad_address(spender));
    let hex = eth_call(rpc, token, &data).await?;
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.is_empty() {
        return Ok(0);
    }
    let val = u128::from_str_radix(&trimmed[trimmed.len().saturating_sub(32)..], 16)
        .unwrap_or(0);
    Ok(val)
}
