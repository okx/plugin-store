use anyhow::Result;
use serde_json::Value;

use crate::api;

pub async fn execute(chains: Option<&str>) -> Result<Value> {
    let resp = api::get_tools(chains).await?;

    let bridges: Vec<Value> = resp["bridges"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|b| serde_json::json!({"key": b["key"], "name": b["name"]}))
        .collect();

    let exchanges: Vec<Value> = resp["exchanges"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|e| serde_json::json!({"key": e["key"], "name": e["name"]}))
        .collect();

    Ok(serde_json::json!({
        "ok": true,
        "bridges": bridges,
        "bridgeCount": bridges.len(),
        "exchanges": exchanges,
        "exchangeCount": exchanges.len()
    }))
}
