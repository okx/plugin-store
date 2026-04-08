// EVM RPC helpers for Beefy vault interactions

use anyhow::Result;
use serde_json::{json, Value};

use crate::config::rpc_url;

async fn eth_call(rpc: &str, to: &str, data: &str) -> Result<String> {
    let mut builder = reqwest::Client::builder();
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
        if let Ok(proxy) = reqwest::Proxy::https(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    let client = builder.build().unwrap_or_default();

    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
        "id": 1
    });

    let resp = client.post(rpc).json(&body).send().await?;
    let json: Value = resp.json().await?;

    if let Some(err) = json.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }
    Ok(json["result"].as_str().unwrap_or("0x").to_string())
}

/// Get mooToken balance of a wallet in a Beefy vault
/// balanceOf(address) selector: 0x70a08231
pub async fn get_moo_balance(chain_id: u64, vault_addr: &str, wallet: &str) -> Result<u128> {
    let rpc = rpc_url(chain_id);
    // balanceOf(address) = 0x70a08231
    let wallet_padded = format!("{:0>64}", &wallet.trim_start_matches("0x").to_lowercase());
    let calldata = format!("0x70a08231{}", wallet_padded);
    let result = eth_call(rpc, vault_addr, &calldata).await?;
    if result == "0x" || result.is_empty() {
        return Ok(0);
    }
    Ok(u128::from_str_radix(result.trim_start_matches("0x"), 16).unwrap_or(0))
}

/// Get pricePerFullShare for a Beefy vault
/// pricePerFullShare() selector: 0x77c7b8fc
pub async fn get_price_per_full_share(chain_id: u64, vault_addr: &str) -> Result<u128> {
    let rpc = rpc_url(chain_id);
    let result = eth_call(rpc, vault_addr, "0x77c7b8fc").await?;
    if result == "0x" || result.is_empty() {
        return Ok(1_000_000_000_000_000_000); // 1e18 default
    }
    Ok(u128::from_str_radix(result.trim_start_matches("0x"), 16).unwrap_or(1_000_000_000_000_000_000))
}

/// Get ERC-20 token decimals
/// decimals() selector: 0x313ce567
pub async fn get_decimals(chain_id: u64, token_addr: &str) -> Result<u32> {
    let rpc = rpc_url(chain_id);
    let result = eth_call(rpc, token_addr, "0x313ce567").await?;
    if result == "0x" || result.is_empty() {
        return Ok(18);
    }
    Ok(u32::from_str_radix(result.trim_start_matches("0x"), 16).unwrap_or(18))
}

/// Get ERC-20 allowance
/// allowance(address,address) selector: 0xdd62ed3e
pub async fn get_allowance(chain_id: u64, token_addr: &str, owner: &str, spender: &str) -> Result<u128> {
    let rpc = rpc_url(chain_id);
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x").to_lowercase());
    let spender_padded = format!("{:0>64}", spender.trim_start_matches("0x").to_lowercase());
    let calldata = format!("0xdd62ed3e{}{}", owner_padded, spender_padded);
    let result = eth_call(rpc, token_addr, &calldata).await?;
    if result == "0x" || result.is_empty() {
        return Ok(0);
    }
    Ok(u128::from_str_radix(result.trim_start_matches("0x"), 16).unwrap_or(0))
}

/// Get vault total supply (mooTokens)
/// totalSupply() selector: 0x18160ddd
pub async fn get_total_supply(chain_id: u64, vault_addr: &str) -> Result<u128> {
    let rpc = rpc_url(chain_id);
    let result = eth_call(rpc, vault_addr, "0x18160ddd").await?;
    if result == "0x" || result.is_empty() {
        return Ok(0);
    }
    Ok(u128::from_str_radix(result.trim_start_matches("0x"), 16).unwrap_or(0))
}

/// Get vault balance (total underlying assets in vault)
/// balance() selector: 0xb69ef8a8
pub async fn get_vault_balance(chain_id: u64, vault_addr: &str) -> Result<u128> {
    let rpc = rpc_url(chain_id);
    let result = eth_call(rpc, vault_addr, "0xb69ef8a8").await?;
    if result == "0x" || result.is_empty() {
        return Ok(0);
    }
    Ok(u128::from_str_radix(result.trim_start_matches("0x"), 16).unwrap_or(0))
}
