/// `polymarket history` — show trade activity for the active wallet.
///
/// Queries the Polymarket Data API `/activity` endpoint. In POLY_PROXY mode,
/// automatically uses the proxy wallet address; otherwise uses the EOA.
///
/// Covers: buys, sells, redeems. Does NOT include deposit/withdrawal transfers
/// (those are on-chain USDC.e transfers — see Polygonscan for the full on-chain record).

use anyhow::{Context, Result};
use reqwest::Client;

pub async fn run(limit: u32, address: Option<&str>) -> Result<()> {
    let client = Client::new();

    // Resolve wallet: proxy wallet in POLY_PROXY mode, else EOA.
    let wallet_addr = if let Some(a) = address {
        a.to_string()
    } else {
        let eoa = crate::onchainos::get_wallet_address().await?;
        let creds = crate::config::load_credentials().ok().flatten();
        let use_proxy = creds.as_ref().and_then(|c| {
            if c.mode == crate::config::TradingMode::PolyProxy {
                c.proxy_wallet.clone()
            } else {
                None
            }
        });
        use_proxy.unwrap_or(eoa)
    };

    let url = format!(
        "{}/activity?user={}&limit={}&offset=0",
        crate::config::Urls::DATA,
        wallet_addr,
        limit,
    );

    let resp: serde_json::Value = client
        .get(&url)
        .send()
        .await
        .context("fetching activity from Data API")?
        .json()
        .await
        .context("parsing activity response")?;

    // Data API returns either a plain array or {"data": [...]}
    let items: Vec<serde_json::Value> = if resp.is_array() {
        resp.as_array().cloned().unwrap_or_default()
    } else {
        resp["data"].as_array().cloned().unwrap_or_default()
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "wallet": wallet_addr,
                "count": items.len(),
                "activity": items,
                "note": "Covers buys, sells, and redeems. For deposit/withdrawal history, check Polygonscan for USDC.e transfers: https://polygonscan.com/token/0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174?a=<wallet>",
            }
        }))?
    );

    Ok(())
}
