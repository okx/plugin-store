// src/commands/search_tokens.rs — search Clanker tokens by creator address or Farcaster username
use crate::api;
use anyhow::Result;
use serde_json::Value;

pub async fn run(
    query: &str,
    limit: u32,
    offset: u32,
    sort: &str,
    trusted_only: bool,
) -> Result<()> {
    let result: Value = api::search_creator(query, limit, offset, sort, trusted_only).await?;

    let tokens = result["tokens"].as_array().cloned().unwrap_or_default();
    let total = result["total"].as_u64().unwrap_or(0);

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "query": query,
            "tokens": tokens.iter().map(|t| {
                serde_json::json!({
                    "contract_address": t["contract_address"],
                    "name": t["name"],
                    "symbol": t["symbol"],
                    "chain_id": t["chain_id"],
                    "deployed_at": t["deployed_at"],
                    "trust_status": t["trustStatus"],
                })
            }).collect::<Vec<_>>(),
            "total": total,
            "user": result["user"],
            "searched_address": result["searchedAddress"],
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
