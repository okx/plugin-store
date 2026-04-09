use anyhow::Result;
use serde_json::Value;

use crate::api;

pub async fn run(
    chain_id: u64,
    market_address: &str,
    time_frame: Option<&str>,
    api_key: Option<&str>,
) -> Result<Value> {
    // Map user-facing time frame aliases to Pendle API values
    let mapped_tf: Option<String> = time_frame.map(|tf| match tf {
        "1D" | "day" => "day".to_string(),
        "1W" | "week" => "week".to_string(),
        "1M" | "month" => "month".to_string(),
        "1H" | "hour" => "hour".to_string(),
        other => other.to_string(), // pass through if already in correct format
    });
    let data = api::get_market(chain_id, market_address, mapped_tf.as_deref(), api_key).await?;
    Ok(data)
}
