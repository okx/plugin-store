// rates command — show APR/APY for Yearn vaults

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
                    || v.address.to_lowercase() == f
            } else {
                true
            }
        })
        .collect();

    let rates: Vec<_> = active
        .iter()
        .map(|v| {
            let apr = v.apr.as_ref();
            let points = apr.and_then(|a| a.points.as_ref());
            let fees = apr.and_then(|a| a.fees.as_ref());

            json!({
                "address": v.address,
                "name": v.name.as_deref().unwrap_or("Unknown"),
                "token": v.token.symbol,
                "net_apr": v.apr_display(),
                "net_apr_raw": apr.and_then(|a| a.net_apr).unwrap_or(0.0),
                "history": {
                    "week_ago": points.and_then(|p| p.week_ago)
                        .map(|v| format!("{:.2}%", v * 100.0))
                        .unwrap_or_else(|| "N/A".to_string()),
                    "month_ago": points.and_then(|p| p.month_ago)
                        .map(|v| format!("{:.2}%", v * 100.0))
                        .unwrap_or_else(|| "N/A".to_string()),
                    "inception": points.and_then(|p| p.inception)
                        .map(|v| format!("{:.2}%", v * 100.0))
                        .unwrap_or_else(|| "N/A".to_string()),
                },
                "fees": {
                    "performance": fees.and_then(|f| f.performance)
                        .map(|v| format!("{:.0}%", v * 100.0))
                        .unwrap_or_else(|| "N/A".to_string()),
                    "management": fees.and_then(|f| f.management)
                        .map(|v| format!("{:.0}%", v * 100.0))
                        .unwrap_or_else(|| "N/A".to_string()),
                }
            })
        })
        .collect();

    // Sort by APR descending
    let mut rates = rates;
    rates.sort_by(|a, b| {
        let va = a["net_apr_raw"].as_f64().unwrap_or(0.0);
        let vb = b["net_apr_raw"].as_f64().unwrap_or(0.0);
        vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "data": {
                "chain_id": chain_id,
                "count": rates.len(),
                "rates": rates
            }
        }))?
    );
    Ok(())
}
