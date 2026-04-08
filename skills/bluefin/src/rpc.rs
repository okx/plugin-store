use anyhow::Context;
use serde_json::{json, Value};

#[allow(dead_code)]
const SUI_RPC: &str = "https://fullnode.mainnet.sui.io";

/// Call the Sui JSON-RPC endpoint.
#[allow(dead_code)]
pub async fn sui_rpc(method: &str, params: Value) -> anyhow::Result<Value> {
    let client = reqwest::Client::builder()
        .user_agent("bluefin-plugin/0.1.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("Failed to build HTTP client")?;

    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });

    let resp = client
        .post(SUI_RPC)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("Sui RPC call {} failed", method))?;

    let status = resp.status();
    let text = resp.text().await.context("Failed to read RPC response")?;

    if !status.is_success() {
        anyhow::bail!("Sui RPC HTTP error {}: {}", status, text);
    }

    let val: Value = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse Sui RPC response: {}", text))?;

    if let Some(err) = val.get("error") {
        anyhow::bail!("Sui RPC error: {}", err);
    }

    Ok(val["result"].clone())
}

/// Get SUI balance for an address.
#[allow(dead_code)]
pub async fn get_sui_balance(address: &str) -> anyhow::Result<Value> {
    sui_rpc(
        "suix_getBalance",
        json!([address, "0x2::sui::SUI"]),
    )
    .await
}
