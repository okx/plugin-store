use anyhow::Result;
use serde_json::Value;

use crate::api;

/// Strip the optional "chainId-" prefix from a Pendle address string.
/// Pendle API returns addresses as "42161-0xabc..." — callers expect just "0xabc...".
fn strip_chain_prefix(addr: &str) -> &str {
    if let Some(pos) = addr.find("-0x") {
        &addr[pos + 1..]
    } else {
        addr
    }
}

fn extract_addr(v: &Value) -> String {
    // Pendle API encodes addresses as plain strings "chainId-0x..." at the market level
    v.as_str().map(strip_chain_prefix).unwrap_or("").to_string()
}

/// Returns a clean summary of token addresses for a Pendle market.
/// Fetches market data from the Pendle API and extracts the PT, YT, SY, LP,
/// and underlying asset addresses needed for trading commands.
pub async fn run(chain_id: u64, market: &str, api_key: Option<&str>) -> Result<Value> {
    let market_lower = market.to_lowercase();

    // Paginate in 100-result pages (Pendle API cap) to find the target market
    let mut found_market: Option<serde_json::Value> = None;
    let mut skip = 0u64;
    loop {
        let data = api::list_markets(Some(chain_id), None, skip, 100, api_key).await?;

        let results = data["results"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Unexpected response from Pendle markets API"))?;

        if let Some(m) = results.iter().find(|m| {
            m["address"]
                .as_str()
                .map(|a| strip_chain_prefix(a).to_lowercase() == market_lower)
                .unwrap_or(false)
        }) {
            found_market = Some(m.clone());
            break;
        }

        let total = data["total"].as_u64().unwrap_or(0);
        skip += results.len() as u64;
        if skip >= total || results.is_empty() {
            break;
        }
    }

    let m = found_market.ok_or_else(|| {
        anyhow::anyhow!(
            "Market {} not found on chain {}. Use list-markets to discover available markets.",
            market, chain_id
        )
    })?;

    let pt_address = extract_addr(&m["pt"]);
    let yt_address = extract_addr(&m["yt"]);
    let sy_address = extract_addr(&m["sy"]);
    let underlying_address = extract_addr(&m["underlyingAsset"]);
    let expiry = m["expiry"].as_str().unwrap_or("");
    let name = m["name"].as_str().unwrap_or("");
    let implied_apy = m["impliedApy"].as_f64().map(|v| format!("{:.4}", v));

    Ok(serde_json::json!({
        "ok": true,
        "chain_id": chain_id,
        "market": market,
        "name": name,
        "expiry": expiry,
        "implied_apy": implied_apy,
        "addresses": {
            "market_lp": market,
            "pt": pt_address,
            "yt": yt_address,
            "sy": sy_address,
            "underlying": underlying_address,
        },
        "usage": {
            "buy-pt":           format!("pendle --chain {} buy-pt --pt-address {} --token-in {} --amount-in <WEI>", chain_id, pt_address, underlying_address),
            "sell-pt":          format!("pendle --chain {} sell-pt --pt-address {} --token-out {} --amount-in <WEI>", chain_id, pt_address, underlying_address),
            "buy-yt":           format!("pendle --chain {} buy-yt --yt-address {} --token-in {} --amount-in <WEI>", chain_id, yt_address, underlying_address),
            "sell-yt":          format!("pendle --chain {} sell-yt --yt-address {} --token-out {} --amount-in <WEI>", chain_id, yt_address, underlying_address),
            "add-liquidity":    format!("pendle --chain {} add-liquidity --lp-address {} --token-in {} --amount-in <WEI>", chain_id, market, underlying_address),
            "remove-liquidity": format!("pendle --chain {} remove-liquidity --lp-address {} --token-out {} --lp-amount-in <WEI>", chain_id, market, underlying_address),
            "mint-py":          format!("pendle --chain {} mint-py --pt-address {} --yt-address {} --token-in {} --amount-in <WEI>", chain_id, pt_address, yt_address, underlying_address),
        }
    }))
}
