// src/rpc.rs — Direct eth_call queries
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
    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// Parse a uint256 from a 32-byte ABI-encoded hex result
pub fn parse_u128(hex_result: &str) -> anyhow::Result<u128> {
    let clean = hex_result.trim_start_matches("0x");
    if clean.is_empty() || clean == "0" {
        return Ok(0);
    }
    let trimmed = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    u128::from_str_radix(trimmed, 16).context("parse u128 failed")
}

/// Pad an address to 32 bytes
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u128 to 32 bytes
#[allow(dead_code)]
pub fn pad_u128(val: u128) -> String {
    format!("{:064x}", val)
}

// ── Moonwell mToken read calls ────────────────────────────────────────────────

/// mToken.supplyRatePerTimestamp() → u128 (scaled by 1e18, per second)
/// selector: 0xd3bd2c72
pub async fn supply_rate_per_timestamp(mtoken: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(mtoken, "0xd3bd2c72", rpc_url).await?;
    parse_u128(&result)
}

/// mToken.borrowRatePerTimestamp() → u128 (scaled by 1e18, per second)
/// selector: 0xcd91801c
pub async fn borrow_rate_per_timestamp(mtoken: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(mtoken, "0xcd91801c", rpc_url).await?;
    parse_u128(&result)
}

/// mToken.exchangeRateCurrent() → u128 (underlying per mToken, scaled by 1e18)
/// selector: 0xbd6d894d
pub async fn exchange_rate_current(mtoken: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(mtoken, "0xbd6d894d", rpc_url).await?;
    parse_u128(&result)
}

/// mToken.balanceOf(address) → u128 (mToken units, 8 decimals)
/// selector: 0x70a08231
pub async fn balance_of(mtoken: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x70a08231{}", pad_address(wallet));
    let result = eth_call(mtoken, &data, rpc_url).await?;
    parse_u128(&result)
}

/// mToken.borrowBalanceCurrent(address) → u128 (underlying units)
/// selector: 0x17bfdfbc
pub async fn borrow_balance_current(mtoken: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x17bfdfbc{}", pad_address(wallet));
    let result = eth_call(mtoken, &data, rpc_url).await?;
    parse_u128(&result)
}

/// ERC-20 balanceOf(address) → u128
/// selector: 0x70a08231
#[allow(dead_code)]
pub async fn erc20_balance_of(token: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x70a08231{}", pad_address(wallet));
    let result = eth_call(token, &data, rpc_url).await?;
    parse_u128(&result)
}

/// Convert rate per second to APR percentage
/// APR% = rate_per_second * seconds_per_year / 1e18 * 100
pub fn rate_to_apr_pct(rate_per_second: u128, seconds_per_year: u128) -> f64 {
    (rate_per_second as f64) * (seconds_per_year as f64) / 1e18 * 100.0
}

/// Format underlying balance given mToken amount and exchange rate
/// underlying_raw = mtoken_balance * exchange_rate / 1e18
pub fn mtoken_to_underlying_raw(mtoken_balance: u128, exchange_rate: u128) -> f64 {
    (mtoken_balance as f64) * (exchange_rate as f64) / 1e18
}
