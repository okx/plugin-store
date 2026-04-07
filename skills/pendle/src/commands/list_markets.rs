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
    let data = api::list_markets(chain_id, is_active, skip, limit, api_key).await?;
    Ok(data)
}
