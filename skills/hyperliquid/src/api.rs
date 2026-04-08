use anyhow::Context;
use serde_json::{json, Value};

/// POST to the Hyperliquid info endpoint.
pub async fn info_post(url: &str, body: Value) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Hyperliquid info HTTP request failed")?;

    let status = resp.status();
    let text = resp.text().await.context("Failed to read response body")?;

    if !status.is_success() {
        anyhow::bail!("Hyperliquid API error {}: {}", status, text);
    }

    serde_json::from_str(&text).context("Failed to parse Hyperliquid info response as JSON")
}

/// Get all mid prices: POST /info {"type":"allMids"}
/// Returns a map of coin -> mid price string, e.g. {"BTC":"67234.5","ETH":"3456.2",...}
pub async fn get_all_mids(info_url: &str) -> anyhow::Result<Value> {
    info_post(info_url, json!({"type": "allMids"})).await
}

/// Get clearinghouse state for a user (perp positions, margin summary).
/// POST /info {"type":"clearinghouseState","user":"0x..."}
pub async fn get_clearinghouse_state(info_url: &str, user: &str) -> anyhow::Result<Value> {
    info_post(
        info_url,
        json!({
            "type": "clearinghouseState",
            "user": user
        }),
    )
    .await
}

/// Get open orders for a user.
/// POST /info {"type":"openOrders","user":"0x..."}
pub async fn get_open_orders(info_url: &str, user: &str) -> anyhow::Result<Value> {
    info_post(
        info_url,
        json!({
            "type": "openOrders",
            "user": user
        }),
    )
    .await
}

/// Get metadata for all perpetual markets (asset index map, etc.).
/// POST /info {"type":"meta"}
pub async fn get_meta(info_url: &str) -> anyhow::Result<Value> {
    info_post(info_url, json!({"type": "meta"})).await
}

/// Look up the asset index for a coin symbol from meta.
/// Returns None if the coin is not found.
pub async fn get_asset_index(info_url: &str, coin: &str) -> anyhow::Result<usize> {
    let meta = get_meta(info_url).await?;
    let universe = meta["universe"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("meta.universe missing or not an array"))?;

    let coin_upper = coin.to_uppercase();
    for (i, asset) in universe.iter().enumerate() {
        if let Some(name) = asset["name"].as_str() {
            if name.to_uppercase() == coin_upper {
                return Ok(i);
            }
        }
    }
    anyhow::bail!("Coin '{}' not found in Hyperliquid universe", coin)
}
