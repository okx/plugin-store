// Direct eth_call queries to Ethereum RPC — no onchainos needed for reads

use anyhow::Result;
use serde_json::{json, Value};

/// Execute an eth_call against the Ethereum RPC
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> Result<String> {
    let client = reqwest::Client::new();
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

/// Decode a uint256 from a 32-byte hex result
pub fn decode_u128(hex_result: &str) -> u128 {
    let clean = hex_result.trim_start_matches("0x");
    if clean.len() < 64 {
        return 0;
    }
    let relevant = &clean[clean.len().saturating_sub(32)..];
    u128::from_str_radix(relevant, 16).unwrap_or(0)
}

/// Query balanceOf(address) for a vault token
pub async fn get_balance_of(
    contract: &str,
    owner: &str,
    rpc_url: &str,
) -> Result<u128> {
    let owner_clean = owner.trim_start_matches("0x");
    let owner_padded = format!("{:0>64}", owner_clean);
    let data = format!("0x70a08231{}", owner_padded);
    let result = eth_call(contract, &data, rpc_url).await?;
    Ok(decode_u128(&result))
}

/// Query pricePerShare() for a Yearn vault
pub async fn get_price_per_share(vault: &str, rpc_url: &str) -> Result<u128> {
    let result = eth_call(vault, "0x99530b06", rpc_url).await?;
    Ok(decode_u128(&result))
}

/// Query totalAssets() for a vault
pub async fn get_total_assets(vault: &str, rpc_url: &str) -> Result<u128> {
    let result = eth_call(vault, "0x01e1d114", rpc_url).await?;
    Ok(decode_u128(&result))
}
