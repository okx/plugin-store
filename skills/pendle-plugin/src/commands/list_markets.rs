use anyhow::Result;
use serde_json::Value;

use crate::api;

pub async fn run(
    chain_id: Option<u64>,
    is_active: Option<bool>,
    skip: u64,
    limit: u64,
    api_key: Option<&str>,
) -> Result<Value> {
    let mut data = api::list_markets(chain_id, is_active, skip, limit, api_key).await?;

    // M1: Pendle API moved impliedApy and liquidity into a nested `details` sub-object.
    // Lift them to the top level of each result when the top-level fields are null.
    if let Some(arr) = data.get_mut("results").and_then(|v| v.as_array_mut()) {
        for item in arr.iter_mut() {
            // impliedApy
            if item["impliedApy"].is_null() {
                let v = item["details"]["impliedApy"].clone();
                if !v.is_null() {
                    item["impliedApy"] = v;
                }
            }
            // liquidity (preserve nested structure, just surface it)
            if item["liquidity"].is_null() {
                let v = item["details"]["liquidity"].clone();
                if !v.is_null() {
                    item["liquidity"] = v;
                }
            }
        }
    }

    Ok(data)
}
