// vaults command — list active Yearn vaults with APR and TVL

use crate::api;
use anyhow::Result;
use serde_json::json;

pub async fn execute(chain_id: u64, token_filter: Option<&str>) -> Result<()> {
    let vaults = api::fetch_vaults(chain_id).await?;

    let active: Vec<_> = vaults
        .iter()
        .filter(|v| v.is_active())
        .filter(|v| {
            if let Some(filter) = token_filter {
                let f = filter.to_lowercase();
                v.token.symbol.to_lowercase().contains(&f)
                    || v.name
                        .as_deref()
                        .map(|n| n.to_lowercase().contains(&f))
                        .unwrap_or(false)
            } else {
                true
            }
        })
        .collect();

    let mut vault_list: Vec<_> = active
        .iter()
        .map(|v| {
            json!({
                "address": v.address,
                "name": v.name.as_deref().unwrap_or("Unknown"),
                "symbol": v.symbol.as_deref().unwrap_or(""),
                "version": v.version.as_deref().unwrap_or(""),
                "token": {
                    "symbol": v.token.symbol,
                    "address": v.token.address,
                    "decimals": v.token.decimals
                },
                "net_apr": v.apr_display(),
                "tvl_usd": v.tvl_display()
            })
        })
        .collect();

    // Sort by TVL descending (best effort)
    vault_list.sort_by(|a, b| {
        let ta = a["tvl_usd"].as_str().unwrap_or("$0");
        let tb = b["tvl_usd"].as_str().unwrap_or("$0");
        let va: f64 = ta.trim_start_matches('$').parse().unwrap_or(0.0);
        let vb: f64 = tb.trim_start_matches('$').parse().unwrap_or(0.0);
        vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "data": {
                "chain_id": chain_id,
                "count": vault_list.len(),
                "vaults": vault_list
            }
        }))?
    );
    Ok(())
}
