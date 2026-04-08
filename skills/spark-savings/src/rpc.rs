use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    params: Value,
    id: u64,
}

#[derive(Deserialize)]
struct RpcResponse {
    result: Option<String>,
    error: Option<Value>,
}

/// Perform a raw eth_call.
pub async fn eth_call(rpc_url: &str, to: &str, data: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let req = RpcRequest {
        jsonrpc: "2.0",
        method: "eth_call",
        params: json!([{"to": to, "data": data}, "latest"]),
        id: 1,
    };
    let resp: RpcResponse = client
        .post(rpc_url)
        .json(&req)
        .send()
        .await
        .context("eth_call HTTP failed")?
        .json()
        .await
        .context("eth_call parse failed")?;

    if let Some(err) = resp.error {
        anyhow::bail!("eth_call RPC error: {}", err);
    }
    Ok(resp.result.unwrap_or_default())
}

/// Decode a uint256 from a 32-byte hex result.
pub fn decode_u256(hex: &str) -> anyhow::Result<u128> {
    let hex = hex.trim_start_matches("0x");
    if hex.is_empty() || hex == "0" {
        return Ok(0);
    }
    let padded = format!("{:0>64}", hex);
    let bytes = hex::decode(&padded).context("hex decode failed")?;
    let val = u128::from_be_bytes(bytes[16..32].try_into().context("slice error")?);
    Ok(val)
}

/// Decode a uint256 as a big integer string (for large values).
#[allow(dead_code)]
pub fn decode_u256_string(hex: &str) -> anyhow::Result<String> {
    let hex = hex.trim_start_matches("0x");
    if hex.is_empty() {
        return Ok("0".to_string());
    }
    // Convert to decimal via f64 for display (precision loss is acceptable for display)
    let padded = format!("{:0>64}", hex);
    let val = u128::from_str_radix(&padded[32..], 16).unwrap_or(0);
    Ok(val.to_string())
}

/// Decode a 20-byte address from a 32-byte hex result.
#[allow(dead_code)]
pub fn decode_address(hex: &str) -> anyhow::Result<String> {
    let hex = hex.trim_start_matches("0x");
    if hex.len() < 40 {
        anyhow::bail!("hex too short for address: {}", hex);
    }
    Ok(format!("0x{}", &hex[hex.len() - 40..]))
}

/// Encode a uint256 as 32-byte hex (no 0x prefix).
pub fn encode_u256(val: u128) -> String {
    format!("{:064x}", val)
}

/// Encode an address as 32-byte hex (no 0x prefix, left-padded).
pub fn encode_address(addr: &str) -> String {
    let addr = addr.trim_start_matches("0x").to_lowercase();
    format!("{:0>64}", addr)
}

/// Compute human-readable amount to minimal units.
pub fn to_minimal(amount: f64, decimals: u32) -> u128 {
    let scale = 10u128.pow(decimals);
    (amount * scale as f64) as u128
}

/// Convert minimal units to human-readable f64.
pub fn from_minimal(amount: u128, decimals: u32) -> f64 {
    let scale = 10u128.pow(decimals) as f64;
    amount as f64 / scale
}

/// Compute APY from per-second rate in ray (1e27 precision).
/// Formula: apy = (rate / 1e27)^(seconds_per_year) - 1
pub fn ray_to_apy(ray_val: u128) -> f64 {
    if ray_val == 0 {
        return 0.0;
    }
    let seconds_per_year: f64 = 365.0 * 24.0 * 3600.0;
    let rate = ray_val as f64 / 1e27;
    rate.powf(seconds_per_year) - 1.0
}

/// Poll eth_getTransactionReceipt until mined or timeout (60s).
pub async fn wait_for_tx(rpc_url: &str, tx_hash: &str) -> anyhow::Result<bool> {
    use std::time::{Duration, Instant};
    let client = reqwest::Client::new();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if Instant::now() > deadline {
            anyhow::bail!("Timeout waiting for tx {}", tx_hash);
        }
        let req = json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });
        if let Ok(resp) = client.post(rpc_url).json(&req).send().await {
            if let Ok(body) = resp.json::<Value>().await {
                let receipt = &body["result"];
                if !receipt.is_null() {
                    let status = receipt["status"].as_str().unwrap_or("0x0");
                    return Ok(status == "0x1");
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
