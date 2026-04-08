// Direct eth_call queries to Ethereum RPC — no onchainos needed for reads
// Instadapp Lite vault on-chain data queries

use anyhow::Result;
use serde_json::{json, Value};

/// Execute an eth_call against the Ethereum RPC
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> Result<String> {
    let client = build_client()?;
    let payload = json!({
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
        .json(&payload)
        .send()
        .await?
        .json()
        .await?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }

    Ok(resp["result"]
        .as_str()
        .unwrap_or("0x")
        .to_string())
}

/// Build reqwest client with proxy support
fn build_client() -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
        builder = builder.proxy(reqwest::Proxy::https(&proxy_url)?);
    } else if let Ok(proxy_url) = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")) {
        builder = builder.proxy(reqwest::Proxy::http(&proxy_url)?);
    }
    Ok(builder.build()?)
}

/// Decode a uint256 from a 32-byte hex result (as u128)
pub fn decode_u128(hex_result: &str) -> u128 {
    let clean = hex_result.trim_start_matches("0x");
    if clean.len() < 32 {
        return 0;
    }
    let relevant = &clean[clean.len().saturating_sub(32)..];
    u128::from_str_radix(relevant, 16).unwrap_or(0)
}

/// Format u128 as float with given decimals
pub fn format_units(raw: u128, decimals: u32) -> String {
    let divisor = 10f64.powi(decimals as i32);
    format!("{:.6}", raw as f64 / divisor)
}

/// Query totalSupply() for a vault token (selector: 0x18160ddd)
pub async fn get_total_supply(contract: &str, rpc_url: &str) -> Result<u128> {
    let result = eth_call(contract, "0x18160ddd", rpc_url).await?;
    Ok(decode_u128(&result))
}

/// Query balanceOf(address) for a vault token (selector: 0x70a08231)
pub async fn get_balance_of(contract: &str, owner: &str, rpc_url: &str) -> Result<u128> {
    let owner_clean = owner.trim_start_matches("0x");
    let owner_padded = format!("{:0>64}", owner_clean);
    let data = format!("0x70a08231{}", owner_padded);
    let result = eth_call(contract, &data, rpc_url).await?;
    Ok(decode_u128(&result))
}

/// Query getCurrentExchangePrice() for iETH v1 vault (selector: 0xcc4a0158)
/// Returns (exchangePrice_, newRevenue_) — both uint256
pub async fn get_exchange_price_v1(vault: &str, rpc_url: &str) -> Result<(u128, u128)> {
    let result = eth_call(vault, "0xcc4a0158", rpc_url).await?;
    let clean = result.trim_start_matches("0x");
    if clean.len() < 128 {
        return Ok((1_000_000_000_000_000_000, 0)); // 1e18 fallback
    }
    let price = u128::from_str_radix(&clean[0..64], 16).unwrap_or(0);
    let revenue = u128::from_str_radix(&clean[64..128], 16).unwrap_or(0);
    Ok((price, revenue))
}

/// Query exchangePrice() for iETH v2 vault (selector: 0x9e65741e)
/// Returns a single uint256 exchange price (in stETH units, 1e18 scale)
pub async fn get_exchange_price_v2(vault: &str, rpc_url: &str) -> Result<u128> {
    let result = eth_call(vault, "0x9e65741e", rpc_url).await?;
    Ok(decode_u128(&result))
}

/// Query totalAssets() for iETH v2 vault (selector: 0x01e1d114)
pub async fn get_total_assets(vault: &str, rpc_url: &str) -> Result<u128> {
    let result = eth_call(vault, "0x01e1d114", rpc_url).await?;
    Ok(decode_u128(&result))
}

/// Query netAssets() for iETH v1 vault (selector: 0x0782d421)
/// Returns (netCollateral_, netBorrow_, ...) — we read first two uint256
pub async fn get_net_assets_v1(vault: &str, rpc_url: &str) -> Result<(u128, u128)> {
    let result = eth_call(vault, "0x0782d421", rpc_url).await?;
    let clean = result.trim_start_matches("0x");
    if clean.len() < 128 {
        return Ok((0, 0));
    }
    let net_collateral = u128::from_str_radix(&clean[0..64], 16).unwrap_or(0);
    let net_borrow = u128::from_str_radix(&clean[64..128], 16).unwrap_or(0);
    Ok((net_collateral, net_borrow))
}

/// Query getNetAssets() for iETH v2 vault (selector: 0x08bb5fb0)
/// Returns (totalAssets_, totalDebt_, netAssets_, ...) — first three uint256
pub async fn get_net_assets_v2(vault: &str, rpc_url: &str) -> Result<(u128, u128, u128)> {
    let result = eth_call(vault, "0x08bb5fb0", rpc_url).await?;
    let clean = result.trim_start_matches("0x");
    if clean.len() < 192 {
        return Ok((0, 0, 0));
    }
    let total_assets = u128::from_str_radix(&clean[0..64], 16).unwrap_or(0);
    let total_debt = u128::from_str_radix(&clean[64..128], 16).unwrap_or(0);
    let net_assets = u128::from_str_radix(&clean[128..192], 16).unwrap_or(0);
    Ok((total_assets, total_debt, net_assets))
}
