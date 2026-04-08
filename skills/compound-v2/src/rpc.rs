// src/rpc.rs — Direct eth_call queries (no onchainos required for reads)
use anyhow::Context;
use serde_json::{json, Value};

/// Low-level eth_call
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
        anyhow::bail!("RPC error: {}", err);
    }
    Ok(resp["result"]
        .as_str()
        .unwrap_or("0x")
        .to_string())
}

/// Parse a uint256 from a 32-byte ABI-encoded hex result
pub fn parse_u128(hex_result: &str) -> anyhow::Result<u128> {
    let clean = hex_result.trim_start_matches("0x");
    if clean.is_empty() || clean == "0" {
        return Ok(0);
    }
    // Take last 32 hex chars (16 bytes) to fit u128
    let trimmed = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    Ok(u128::from_str_radix(trimmed, 16).context("parse u128 failed")?)
}

/// Pad an address to 32 bytes (remove 0x, left-pad with zeros)
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u128 to 32 bytes
pub fn pad_u128(val: u128) -> String {
    format!("{:064x}", val)
}

// ── Compound V2 read calls ────────────────────────────────────────────────────

/// cToken.supplyRatePerBlock() → u128 (scaled by 1e18)
pub async fn supply_rate_per_block(ctoken: &str, rpc_url: &str) -> anyhow::Result<u128> {
    // selector: 0xae9d70b0
    let result = eth_call(ctoken, "0xae9d70b0", rpc_url).await?;
    parse_u128(&result)
}

/// cToken.borrowRatePerBlock() → u128 (scaled by 1e18)
pub async fn borrow_rate_per_block(ctoken: &str, rpc_url: &str) -> anyhow::Result<u128> {
    // selector: 0xf8f9da28
    let result = eth_call(ctoken, "0xf8f9da28", rpc_url).await?;
    parse_u128(&result)
}

/// cToken.exchangeRateCurrent() → u128 (underlying per cToken, scaled by 1e18)
pub async fn exchange_rate_current(ctoken: &str, rpc_url: &str) -> anyhow::Result<u128> {
    // selector: 0xbd6d894d
    let result = eth_call(ctoken, "0xbd6d894d", rpc_url).await?;
    parse_u128(&result)
}

/// cToken.balanceOf(address) → u128 (cToken units, 8 decimals)
pub async fn balance_of(ctoken: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    // selector: 0x70a08231
    let data = format!("0x70a08231{}", pad_address(wallet));
    let result = eth_call(ctoken, &data, rpc_url).await?;
    parse_u128(&result)
}

/// cToken.borrowBalanceCurrent(address) → u128 (underlying units)
pub async fn borrow_balance_current(ctoken: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    // selector: 0x17bfdfbc
    let data = format!("0x17bfdfbc{}", pad_address(wallet));
    let result = eth_call(ctoken, &data, rpc_url).await?;
    parse_u128(&result)
}

/// ERC-20 balanceOf(address) → u128
pub async fn erc20_balance_of(token: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    // selector: 0x70a08231 (same as cToken.balanceOf)
    let data = format!("0x70a08231{}", pad_address(wallet));
    let result = eth_call(token, &data, rpc_url).await?;
    parse_u128(&result)
}

/// Convert rate per block to APR percentage
/// APR% = rate_per_block * blocks_per_year / 1e18 * 100
pub fn rate_to_apr_pct(rate_per_block: u128, blocks_per_year: u128) -> f64 {
    (rate_per_block as f64) * (blocks_per_year as f64) / 1e18 * 100.0
}

/// Format underlying balance given cToken amount and exchange rate
/// underlying = ctoken_balance * exchange_rate / 1e18
pub fn ctoken_to_underlying(ctoken_balance: u128, exchange_rate: u128) -> f64 {
    // exchange_rate is scaled by 1e18
    // result in underlying raw units (need to further divide by underlying decimals)
    (ctoken_balance as f64) * (exchange_rate as f64) / 1e18
}
