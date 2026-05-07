// src/commands/list_tokens.rs — list recently deployed Clanker tokens
use crate::api;
use anyhow::Result;
use serde_json::Value;

pub async fn run(
    page: u32,
    limit: u32,
    sort: &str,
    chain_id: Option<u64>,
) -> Result<()> {
    let result: Value = api::list_tokens(page, limit, sort, chain_id).await?;

    // API returns { data: [...], total: ..., cursor: ... }
    // (Note: top-level "data" array, NOT "tokens")
    let tokens = {
        let raw = result["data"].as_array().cloned().unwrap_or_default();
        if let Some(cid) = chain_id {
            raw.into_iter()
                .filter(|t| t["chain_id"].as_u64().map(|c| c == cid).unwrap_or(true))
                .collect::<Vec<_>>()
        } else {
            raw
        }
    };
    let total = result["total"].as_u64().unwrap_or(0);
    // API uses cursor-based pagination; derive has_more from whether cursor is present
    let has_more = result["cursor"].is_string() && !result["cursor"].as_str().unwrap_or("").is_empty();

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "tokens": tokens.iter().map(|t| {
                serde_json::json!({
                    "contract_address": t["contract_address"],
                    "name": t["name"],
                    "symbol": t["symbol"],
                    "chain_id": t["chain_id"],
                    "deployed_at": t["deployed_at"],
                    "img_url": t["img_url"],
                    "pool_address": t["pool_address"],
                    "description": t["description"],
                })
            }).collect::<Vec<_>>(),
            "total": total,
            "has_more": has_more,
            "page": page,
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
