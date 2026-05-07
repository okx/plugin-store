use anyhow::Result;
use serde_json::Value;

use crate::api;

pub async fn run(
    chain_id: u64,
    market_address: &str,
    time_frame: Option<&str>,
    api_key: Option<&str>,
) -> Result<Value> {
    // Map user-facing time-frame values to Pendle API interval parameter values.
    // The API accepts "hour", "day", "week" — not the display aliases "1D", "1W", "1M".
    let mapped_time_frame = time_frame.map(|tf| match tf {
        "1D" => "hour",
        "1W" => "day",
        "1M" => "week",
        other => other,
    });
    let data = api::get_market(chain_id, market_address, mapped_time_frame, api_key).await?;
    Ok(data)
}
