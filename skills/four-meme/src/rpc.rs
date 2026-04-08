use anyhow::Context;
use serde_json::{json, Value};

/// Execute an eth_call against the BSC RPC
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": data },
            "latest"
        ],
        "id": 1
    });
    let resp: Value = client
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

/// Decode a uint256 from a hex eth_call result at slot `index` (each slot = 32 bytes / 64 hex chars)
/// Returns the lower 128 bits (sufficient for BNB and token amounts in normal ranges)
pub fn decode_uint256(hex_data: &str, slot: usize) -> u128 {
    let data = hex_data.trim_start_matches("0x");
    let start = slot * 64;
    if data.len() < start + 64 {
        return 0;
    }
    let chunk = &data[start..start + 64];
    // Parse last 32 hex chars (16 bytes = u128 max)
    let lower = &chunk[32..];
    u128::from_str_radix(lower, 16).unwrap_or(0)
}

/// Decode an address (last 20 bytes) from slot index
pub fn decode_address(hex_data: &str, slot: usize) -> String {
    let data = hex_data.trim_start_matches("0x");
    let start = slot * 64;
    if data.len() < start + 64 {
        return crate::config::ZERO_ADDRESS.to_string();
    }
    let chunk = &data[start..start + 64];
    if chunk.len() < 40 {
        return crate::config::ZERO_ADDRESS.to_string();
    }
    format!("0x{}", &chunk[chunk.len() - 40..])
}

/// Decode bool from slot index
pub fn decode_bool(hex_data: &str, slot: usize) -> bool {
    decode_uint256(hex_data, slot) != 0
}
