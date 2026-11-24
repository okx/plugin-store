use anyhow::Context;

pub const CHAIN_ID: u64 = 1; // Ethereum mainnet
pub const RPC_URL: &str = "https://ethereum.publicnode.com";

/// Make a raw eth_call and return the result hex string.
pub async fn eth_call(to: &str, data: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "id": 1,
        "params": [{"to": to, "data": data}, "latest"]
    });
    let resp: serde_json::Value = client
        .post(RPC_URL)
        .json(&body)
        .send().await
        .context("RPC request failed")?
        .json().await
        .context("Failed to parse RPC response")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("RPC error: {}", err);
    }
    resp.get("result")
        .and_then(|r| r.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))
}

/// Decode a 32-byte ABI word at offset `slot` (0-indexed) from a hex string result.
/// Returns the word as a hex string (no 0x prefix, 64 chars).
pub fn decode_word(result: &str, slot: usize) -> Option<String> {
    let result = result.trim_start_matches("0x");
    let start = slot * 64;
    if result.len() < start + 64 {
        return None;
    }
    Some(result[start..start + 64].to_string())
}

/// Decode an address from a 32-byte word (last 20 bytes).
pub fn decode_address(word: &str) -> String {
    let word = word.trim_start_matches("0x");
    if word.len() >= 40 {
        format!("0x{}", &word[word.len() - 40..])
    } else {
        "0x0000000000000000000000000000000000000000".to_string()
    }
}

/// Decode a u128 from a 32-byte word.
pub fn decode_uint(word: &str) -> u128 {
    let word = word.trim_start_matches("0x");
    u128::from_str_radix(&word[word.len().saturating_sub(32)..], 16).unwrap_or(0)
}
