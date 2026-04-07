/// ABI encoding/decoding helpers

/// Pad a hex address (with or without 0x) to a 32-byte (64 hex char) left-zero-padded word.
pub fn encode_address(addr: &str) -> String {
    let addr = addr.trim_start_matches("0x").trim_start_matches("0X");
    format!("{:0>64}", addr)
}

/// Encode a u128 as a 32-byte big-endian hex word (no 0x prefix).
pub fn encode_uint256_u128(val: u128) -> String {
    format!("{:064x}", val)
}

/// Decode a single uint256 from ABI-encoded return data.
pub fn decode_uint256(hex: &str) -> anyhow::Result<u128> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.is_empty() || hex == "0" {
        return Ok(0);
    }
    if hex.len() < 64 {
        anyhow::bail!("Return data too short for uint256: '{}'", hex);
    }
    let word = &hex[hex.len() - 64..];
    Ok(u128::from_str_radix(word, 16)?)
}

/// Decode three consecutive uint256 values (e.g. userReceipts tuple).
pub fn decode_tuple3_u128(hex: &str) -> anyhow::Result<(u128, u128, u128)> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 192 {
        return Ok((0, 0, 0));
    }
    let a = u128::from_str_radix(&hex[0..64], 16)?;
    let b = u128::from_str_radix(&hex[64..128], 16)?;
    let c = u128::from_str_radix(&hex[128..192], 16)?;
    Ok((a, b, c))
}

/// Decode two consecutive uint256 values.
pub fn decode_tuple2_u128(hex: &str) -> anyhow::Result<(u128, u128)> {
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() < 128 {
        return Ok((0, 0));
    }
    let a = u128::from_str_radix(&hex[0..64], 16)?;
    let b = u128::from_str_radix(&hex[64..128], 16)?;
    Ok((a, b))
}

/// Direct JSON-RPC eth_call to publicnode.
pub fn eth_call(to: &str, calldata: &str) -> anyhow::Result<String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": calldata },
            "latest"
        ],
        "id": 1
    });

    let client = build_client()?;
    let resp: serde_json::Value = client
        .post(crate::config::RPC_URL)
        .json(&body)
        .send()?
        .json()?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call RPC error: {}", err);
    }

    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// Build reqwest blocking client with proxy support.
pub fn build_client() -> anyhow::Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder();
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
        if !proxy_url.is_empty() {
            builder = builder.proxy(reqwest::Proxy::https(&proxy_url)?);
        }
    }
    if let Ok(proxy_url) = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")) {
        if !proxy_url.is_empty() {
            builder = builder.proxy(reqwest::Proxy::http(&proxy_url)?);
        }
    }
    Ok(builder.build()?)
}
