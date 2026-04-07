use anyhow::Result;
use serde_json::Value;

use crate::api;

pub async fn run(
    chain_id: u64,
    market_address: &str,
    time_frame: Option<&str>,
    api_key: Option<&str>,
) -> Result<Value> {
    let data = api::get_market(chain_id, market_address, time_frame, api_key).await?;
    Ok(data)
}
