use anyhow::Result;
use serde_json::Value;

use crate::api;

pub async fn execute() -> Result<Value> {
    let resp = api::get_chains().await?;
    let chains = resp["chains"].as_array().cloned().unwrap_or_default();

    let summary: Vec<Value> = chains
        .iter()
        .filter(|c| c["mainnet"].as_bool().unwrap_or(false))
        .map(|c| {
            serde_json::json!({
                "id": c["id"],
                "name": c["name"],
                "key": c["key"],
                "chainType": c["chainType"],
                "coin": c["coin"],
                "diamondAddress": c["diamondAddress"]
            })
        })
        .collect();

    Ok(serde_json::json!({
        "ok": true,
        "total": summary.len(),
        "chains": summary
    }))
}
