use serde_json::{json, Value};
use crate::config::{ETH_FALLBACK_RPCS, encode_address, encode_uint256};

fn build_client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
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

/// Low-level eth_call via JSON-RPC with Ethereum fallback endpoints.
pub async fn eth_call(to: &str, calldata: &str) -> anyhow::Result<String> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": calldata },
            "latest"
        ],
        "id": 1
    });

    let mut last_err = anyhow::anyhow!("no RPC endpoints tried");
    for url in ETH_FALLBACK_RPCS {
        let client = build_client();
        match client.post(*url).json(&body).send().await {
            Ok(response) => match response.json::<Value>().await {
                Ok(resp) => {
                    if let Some(err) = resp.get("error") {
                        last_err = anyhow::anyhow!("eth_call error at {}: {}", url, err);
                        continue;
                    }
                    return Ok(resp["result"].as_str().unwrap_or("0x").to_string());
                }
                Err(e) => {
                    last_err = anyhow::anyhow!("eth_call parse failed at {}: {}", url, e);
                }
            },
            Err(e) => {
                last_err = anyhow::anyhow!("eth_call HTTP failed on {}: {}", url, e);
            }
        }
    }
    Err(last_err)
}

/// Decode a single uint256 (first 32 bytes) from eth_call result hex.
pub fn decode_uint256(hex: &str) -> u128 {
    let data = hex.trim_start_matches("0x");
    if data.len() < 64 {
        return 0;
    }
    u128::from_str_radix(&data[..64], 16).unwrap_or(0)
}

// ─── ERC-20 queries ────────────────────────────────────────────────────────

/// balanceOf(address account) → uint256
pub async fn balance_of(token: &str, account: &str) -> anyhow::Result<u128> {
    let calldata = format!("0x70a08231{}", encode_address(account));
    let result = eth_call(token, &calldata).await?;
    Ok(decode_uint256(&result))
}

/// allowance(address owner, address spender) → uint256
pub async fn allowance(token: &str, owner: &str, spender: &str) -> anyhow::Result<u128> {
    let calldata = format!("0xdd62ed3e{}{}", encode_address(owner), encode_address(spender));
    let result = eth_call(token, &calldata).await?;
    Ok(decode_uint256(&result))
}

// ─── ERC4626 Vault queries ─────────────────────────────────────────────────

/// exchangePrice() → uint256
/// Returns current exchange rate (1e18 = 1:1), increases as yield accrues.
pub async fn exchange_price(vault: &str) -> anyhow::Result<u128> {
    let result = eth_call(vault, "0x9e65741e").await?;
    Ok(decode_uint256(&result))
}

/// totalAssets() → uint256
pub async fn total_assets(vault: &str) -> anyhow::Result<u128> {
    let result = eth_call(vault, "0x01e1d114").await?;
    Ok(decode_uint256(&result))
}

/// totalSupply() → uint256
pub async fn total_supply(vault: &str) -> anyhow::Result<u128> {
    let result = eth_call(vault, "0x18160ddd").await?;
    Ok(decode_uint256(&result))
}

/// convertToAssets(uint256 shares) → uint256
pub async fn convert_to_assets(vault: &str, shares: u128) -> anyhow::Result<u128> {
    let calldata = format!("0x07a2d13a{}", encode_uint256(shares));
    let result = eth_call(vault, &calldata).await?;
    Ok(decode_uint256(&result))
}

/// previewDeposit(uint256 assets) → uint256 shares
pub async fn preview_deposit(vault: &str, assets: u128) -> anyhow::Result<u128> {
    let calldata = format!("0xef8b30f7{}", encode_uint256(assets));
    let result = eth_call(vault, &calldata).await?;
    Ok(decode_uint256(&result))
}

/// previewRedeem(uint256 shares) → uint256 assets
pub async fn preview_redeem(vault: &str, shares: u128) -> anyhow::Result<u128> {
    let calldata = format!("0x4cdad506{}", encode_uint256(shares));
    let result = eth_call(vault, &calldata).await?;
    Ok(decode_uint256(&result))
}
