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
    if clean.len() < 64 {
        anyhow::bail!("Result too short: {}", hex_result);
    }
    let val = u128::from_str_radix(&clean[clean.len() - 32..], 16)
        .context("parse u128 failed")?;
    Ok(val)
}

/// Parse a bool from a 32-byte ABI-encoded hex result
pub fn parse_bool(hex_result: &str) -> bool {
    let clean = hex_result.trim_start_matches("0x");
    clean.ends_with('1')
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

// ── Comet read calls ──────────────────────────────────────────────────────────

/// Comet.getUtilization() → u128 (1e18 scaled)
pub async fn get_utilization(comet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(comet, "0x7eb71131", rpc_url).await?;
    parse_u128(&result)
}

/// Comet.getSupplyRate(uint256) → u64 (per-second, 1e18 scaled)
pub async fn get_supply_rate(comet: &str, utilization: u128, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0xd955759d{}", pad_u128(utilization));
    let result = eth_call(comet, &data, rpc_url).await?;
    parse_u128(&result)
}

/// Comet.getBorrowRate(uint256) → u64 (per-second, 1e18 scaled)
pub async fn get_borrow_rate(comet: &str, utilization: u128, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x9fa83b5a{}", pad_u128(utilization));
    let result = eth_call(comet, &data, rpc_url).await?;
    parse_u128(&result)
}

/// Comet.totalSupply() → u128
pub async fn get_total_supply(comet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(comet, "0x18160ddd", rpc_url).await?;
    parse_u128(&result)
}

/// Comet.totalBorrow() → u128
pub async fn get_total_borrow(comet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(comet, "0x8285ef40", rpc_url).await?;
    parse_u128(&result)
}

/// Comet.balanceOf(address) → u128 (supply balance of base asset)
pub async fn get_balance_of(comet: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x70a08231{}", pad_address(wallet));
    let result = eth_call(comet, &data, rpc_url).await?;
    parse_u128(&result)
}

/// Comet.borrowBalanceOf(address) → u128 (borrow balance including accrued interest)
pub async fn get_borrow_balance_of(comet: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x374c49b4{}", pad_address(wallet));
    let result = eth_call(comet, &data, rpc_url).await?;
    parse_u128(&result)
}

/// Comet.collateralBalanceOf(address account, address asset) → u128
pub async fn get_collateral_balance_of(
    comet: &str,
    wallet: &str,
    asset: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let data = format!(
        "0x5c2549ee{}{}",
        pad_address(wallet),
        pad_address(asset)
    );
    let result = eth_call(comet, &data, rpc_url).await?;
    parse_u128(&result)
}

/// Comet.isBorrowCollateralized(address) → bool
pub async fn is_borrow_collateralized(comet: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<bool> {
    let data = format!("0x38aa813f{}", pad_address(wallet));
    let result = eth_call(comet, &data, rpc_url).await?;
    Ok(parse_bool(&result))
}

/// Comet.baseBorrowMin() → u128
pub async fn get_base_borrow_min(comet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(comet, "0x300e6beb", rpc_url).await?;
    parse_u128(&result)
}

/// ERC-20 balanceOf(address) → u128
pub async fn get_erc20_balance(token: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x70a08231{}", pad_address(wallet));
    let result = eth_call(token, &data, rpc_url).await?;
    parse_u128(&result)
}

// ── CometRewards read calls ────────────────────────────────────────────────────

/// CometRewards.getRewardOwed(address comet, address account) → (token, owed)
/// Returns the owed COMP amount (u128). Returns 0 if no rewards.
pub async fn get_reward_owed(
    rewards: &str,
    comet: &str,
    wallet: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let data = format!(
        "0x41e0cad6{}{}",
        pad_address(comet),
        pad_address(wallet)
    );
    let result = eth_call(rewards, &data, rpc_url).await?;
    // Returns (address token, uint256 owed) — 2 x 32 bytes; owed is second word
    let clean = result.trim_start_matches("0x");
    if clean.len() < 128 {
        return Ok(0);
    }
    let owed_hex = &clean[64..128];
    Ok(u128::from_str_radix(owed_hex, 16).unwrap_or(0))
}

/// Convert per-second rate (1e18 scaled) to APR percentage
pub fn rate_to_apr_pct(rate_per_sec: u128) -> f64 {
    (rate_per_sec as f64 / 1e18) * 31_536_000.0 * 100.0
}
