use anyhow::Context;
use serde_json::{json, Value};

/// Build an HTTP client that respects proxy environment variables
pub fn build_client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .http1_only()
        .timeout(std::time::Duration::from_secs(30));
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .or_else(|_| std::env::var("http_proxy"))
    {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap_or_default()
}

/// Low-level eth_call via JSON-RPC with fallback RPC URLs
pub async fn eth_call_with_fallback(
    rpc_urls: &[&str],
    to: &str,
    calldata: &str,
) -> anyhow::Result<String> {
    let mut last_err = anyhow::anyhow!("No RPC URLs configured");
    for &rpc_url in rpc_urls {
        match eth_call(rpc_url, to, calldata).await {
            Ok(r) => return Ok(r),
            Err(e) => {
                last_err = e;
            }
        }
    }
    Err(last_err)
}

/// Low-level eth_call via JSON-RPC
pub async fn eth_call(rpc_url: &str, to: &str, calldata: &str) -> anyhow::Result<String> {
    let client = build_client();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": calldata },
            "latest"
        ],
        "id": 1
    });
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .context("eth_call HTTP request failed")?
        .json()
        .await
        .context("eth_call response parse failed")?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call RPC error: {}", err);
    }
    let result = resp["result"].as_str().unwrap_or("0x").to_string();
    Ok(result)
}

/// Decode a uint256 from a 32-byte hex eth_call result
pub fn decode_uint256(hex: &str) -> u128 {
    let data = hex.trim_start_matches("0x");
    if data.len() < 64 {
        return 0;
    }
    // Take last 32 chars of first word (low 128 bits — sufficient for ERC4626 amounts)
    u128::from_str_radix(&data[data.len().saturating_sub(32)..], 16).unwrap_or(0)
}

/// Decode an address from a 32-byte hex eth_call result (last 20 bytes / 40 hex chars)
#[allow(dead_code)]
pub fn decode_address(hex: &str) -> String {
    let data = hex.trim_start_matches("0x");
    if data.len() < 40 {
        return "0x0000000000000000000000000000000000000000".to_string();
    }
    format!("0x{}", &data[data.len() - 40..])
}

/// Decode a UTF-8 string from an ABI-encoded eth_call result (dynamic bytes/string)
/// Layout: offset(32) + length(32) + data(padded)
#[allow(dead_code)]
pub fn decode_string(hex: &str) -> String {
    let data = hex.trim_start_matches("0x");
    if data.len() < 128 {
        return String::new();
    }
    // word[1] is string length
    let len = usize::from_str_radix(&data[64..128], 16).unwrap_or(0);
    if len == 0 || data.len() < 128 + len * 2 {
        return String::new();
    }
    let bytes_hex = &data[128..128 + len * 2];
    let bytes = (0..bytes_hex.len() / 2)
        .filter_map(|i| u8::from_str_radix(&bytes_hex[i * 2..i * 2 + 2], 16).ok())
        .collect::<Vec<u8>>();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Pad a 20-byte Ethereum address to 32 bytes (ABI encoding)
pub fn pad_address(addr: &str) -> String {
    let stripped = addr.strip_prefix("0x").unwrap_or(addr);
    format!("{:0>64}", stripped)
}

/// Pad a u128 value to 32 bytes (ABI encoding)
pub fn pad_u128(val: u128) -> String {
    format!("{:064x}", val)
}

// ─── ERC4626 / ERC20 read helpers ────────────────────────────────────────────

/// Call balanceOf(address) → shares (uint256)
/// Selector: 0x70a08231
pub async fn balance_of(rpc_urls: &[&str], token: &str, owner: &str) -> anyhow::Result<u128> {
    let calldata = format!("0x70a08231{}", pad_address(owner));
    let result = eth_call_with_fallback(rpc_urls, token, &calldata).await?;
    Ok(decode_uint256(&result))
}

/// Call totalAssets() → uint256
/// Selector: 0x01e1d114
pub async fn total_assets(rpc_urls: &[&str], vault: &str) -> anyhow::Result<u128> {
    let result = eth_call_with_fallback(rpc_urls, vault, "0x01e1d114").await?;
    Ok(decode_uint256(&result))
}

/// Call convertToAssets(uint256 shares) → uint256 assets
/// Selector: 0x07a2d13a
pub async fn convert_to_assets(
    rpc_urls: &[&str],
    vault: &str,
    shares: u128,
) -> anyhow::Result<u128> {
    let calldata = format!("0x07a2d13a{}", pad_u128(shares));
    let result = eth_call_with_fallback(rpc_urls, vault, &calldata).await?;
    Ok(decode_uint256(&result))
}

/// Call previewDeposit(uint256 assets) → uint256 shares
/// Selector: 0xef8b30f7
pub async fn preview_deposit(
    rpc_urls: &[&str],
    vault: &str,
    assets: u128,
) -> anyhow::Result<u128> {
    let calldata = format!("0xef8b30f7{}", pad_u128(assets));
    let result = eth_call_with_fallback(rpc_urls, vault, &calldata).await?;
    Ok(decode_uint256(&result))
}

/// Call previewWithdraw(uint256 assets) → uint256 shares
/// Selector: 0x0a28a477
pub async fn preview_withdraw(
    rpc_urls: &[&str],
    vault: &str,
    assets: u128,
) -> anyhow::Result<u128> {
    let calldata = format!("0x0a28a477{}", pad_u128(assets));
    let result = eth_call_with_fallback(rpc_urls, vault, &calldata).await?;
    Ok(decode_uint256(&result))
}

/// Call symbol() → string
/// Selector: 0x95d89b41
#[allow(dead_code)]
pub async fn get_symbol(rpc_urls: &[&str], token: &str) -> anyhow::Result<String> {
    let result = eth_call_with_fallback(rpc_urls, token, "0x95d89b41").await?;
    Ok(decode_string(&result))
}

/// Call asset() → address (ERC4626)
/// Selector: 0x38d52e0f
#[allow(dead_code)]
pub async fn get_asset(rpc_urls: &[&str], vault: &str) -> anyhow::Result<String> {
    let result = eth_call_with_fallback(rpc_urls, vault, "0x38d52e0f").await?;
    Ok(decode_address(&result))
}
