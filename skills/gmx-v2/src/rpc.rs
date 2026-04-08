use anyhow::Context;
use serde_json::{json, Value};

/// Direct eth_call — bypasses onchainos, queries the chain directly
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

/// Decode a bytes32 from eth_call result at a given 32-byte slot offset
pub fn decode_bytes32(hex_data: &str, slot: usize) -> Option<String> {
    let data = hex_data.trim_start_matches("0x");
    let start = slot * 64;
    if data.len() < start + 64 {
        return None;
    }
    Some(format!("0x{}", &data[start..start + 64]))
}

/// Decode an address from eth_call result at a given 32-byte slot offset
pub fn decode_address(hex_data: &str, slot: usize) -> Option<String> {
    let data = hex_data.trim_start_matches("0x");
    let start = slot * 64;
    if data.len() < start + 64 {
        return None;
    }
    // Address is last 20 bytes (40 hex chars) of 32-byte slot
    let padded = &data[start..start + 64];
    if padded.len() < 40 {
        return None;
    }
    Some(format!("0x{}", &padded[padded.len() - 40..]))
}

/// Decode a uint256 from eth_call result at a given 32-byte slot offset
pub fn decode_u256_str(hex_data: &str, slot: usize) -> Option<String> {
    let data = hex_data.trim_start_matches("0x");
    let start = slot * 64;
    if data.len() < start + 64 {
        return None;
    }
    let hex_val = &data[start..start + 64];
    // Return as decimal string
    if let Ok(val) = u128::from_str_radix(hex_val, 16) {
        Some(val.to_string())
    } else {
        // Very large number — return hex
        Some(format!("0x{}", hex_val))
    }
}
