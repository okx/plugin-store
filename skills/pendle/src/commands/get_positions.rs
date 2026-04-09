use anyhow::Result;
use serde_json::Value;

use crate::api;
use crate::onchainos;

pub async fn run(
    user: Option<&str>,
    chain_id: u64,
    filter_usd: Option<f64>,
    api_key: Option<&str>,
) -> Result<Value> {
    let address = match user {
        Some(addr) => addr.to_string(),
        None => {
            let resolved = onchainos::resolve_wallet(chain_id)?;
            if resolved.is_empty() {
                anyhow::bail!("Cannot resolve wallet address. Pass --user or ensure onchainos is logged in.");
            }
            resolved
        }
    };

    let data = api::get_positions(&address, filter_usd, api_key).await?;
    Ok(data)
}
