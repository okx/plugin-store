use anyhow::Result;
use serde_json::Value;

use crate::api;

pub async fn run(
    chain_id: Option<u64>,
    is_active: Option<bool>,
    skip: u64,
    limit: u64,
    search: Option<&str>,
    api_key: Option<&str>,
) -> Result<Value> {
    // When searching, fetch a larger batch for client-side filtering
    let fetch_limit = if search.is_some() { 100 } else { limit };
    let mut data = api::list_markets(chain_id, is_active, skip, fetch_limit, api_key).await?;

    // Pendle API moved impliedApy and liquidity into a nested `details` sub-object.
    // Lift them back to the top level for backwards-compatible output.
    if let Some(arr) = data.get_mut("results").and_then(|v| v.as_array_mut()) {
        for item in arr.iter_mut() {
            if item["impliedApy"].is_null() {
                let v = item["details"]["impliedApy"].clone();
                if !v.is_null() {
                    item["impliedApy"] = v;
                }
            }
            if item["liquidity"].is_null() {
                let v = item["details"]["liquidity"].clone();
                if !v.is_null() {
                    item["liquidity"] = v;
                }
            }
        }
    }

    let Some(term) = search else {
        return Ok(data);
    };

    let term_lower = term.to_lowercase();

    let results = match data["results"].as_array() {
        Some(r) => r,
        None => return Ok(data), // no results array — passthrough
    };

    let filtered: Vec<&Value> = results
        .iter()
        .filter(|m| {
            let name = m["name"].as_str().unwrap_or("").to_lowercase();
            let pt_sym = m["pt"]["symbol"].as_str().unwrap_or("").to_lowercase();
            let yt_sym = m["yt"]["symbol"].as_str().unwrap_or("").to_lowercase();
            let sy_sym = m["sy"]["symbol"].as_str().unwrap_or("").to_lowercase();
            name.contains(&term_lower)
                || pt_sym.contains(&term_lower)
                || yt_sym.contains(&term_lower)
                || sy_sym.contains(&term_lower)
        })
        .take(limit as usize)
        .collect();

    let is_eth_search = matches!(term_lower.as_str(), "eth" | "weth");

    let hint: Option<String> = if is_eth_search && !filtered.is_empty() {
        // Results found but user searched for raw ETH/WETH — clarify these are derivatives
        Some(
            "These are ETH liquid staking/restaking derivative pools — Pendle does not have \
             raw ETH or WETH pools. All ETH yield on Pendle uses derivatives such as weETH, \
             wstETH, rETH, rsETH, ezETH, sfrxETH, or cbETH as the underlying."
                .to_string(),
        )
    } else if filtered.is_empty() && is_eth_search {
        Some(
            "No markets found for 'ETH'/'WETH' directly. Pendle ETH pools use liquid \
             staking/restaking derivatives — try searching for: weETH, wstETH, rETH, \
             rsETH, ezETH, sfrxETH, cbETH."
                .to_string(),
        )
    } else if filtered.is_empty() {
        Some(format!(
            "No markets matched '{}'. Try a broader search term or omit --search to see all markets.",
            term
        ))
    } else {
        None
    };

    let mut resp = serde_json::json!({
        "results": filtered,
        "total": filtered.len(),
        "search": term,
    });

    if let Some(h) = hint {
        resp["hint"] = serde_json::json!(h);
    }

    Ok(resp)
}
