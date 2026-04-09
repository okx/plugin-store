use anyhow::Result;
use serde_json::Value;

use crate::api;

pub async fn run(
    chain_id: Option<u64>,
    ids: Option<&str>,
    asset_type: Option<&str>,
    api_key: Option<&str>,
) -> Result<Value> {
    let data = api::get_asset_prices(chain_id, ids, asset_type, api_key).await?;
    Ok(data)
}
