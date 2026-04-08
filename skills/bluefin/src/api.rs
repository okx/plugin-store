use anyhow::Context;
use serde_json::Value;

use crate::config::API_BASE;

/// Perform a GET request to the Bluefin REST API.
/// Returns the parsed JSON response.
pub async fn get(path: &str, query: &[(&str, &str)]) -> anyhow::Result<Value> {
    let client = reqwest::Client::builder()
        .user_agent("bluefin-plugin/0.1.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("Failed to build HTTP client")?;

    let url = format!("{}{}", API_BASE, path);

    let mut req = client.get(&url);
    for (k, v) in query {
        req = req.query(&[(k, v)]);
    }

    let resp = req
        .send()
        .await
        .with_context(|| format!("HTTP GET {} failed", url))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("Failed to read response body")?;

    if !status.is_success() {
        if status.as_u16() == 503 {
            anyhow::bail!(
                "Bluefin API is unavailable (503) for {}. Check https://status.bluefin.io for outages.",
                url
            );
        }
        anyhow::bail!("Bluefin API error {} for {}: {}", status, url, text);
    }

    serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse JSON response from {}", url))
}

/// Fetch all market tickers (no symbol filter → returns array).
pub async fn get_all_tickers() -> anyhow::Result<Value> {
    get("/ticker", &[]).await
}

/// Fetch ticker for a single symbol.
pub async fn get_ticker(symbol: &str) -> anyhow::Result<Value> {
    get("/ticker", &[("symbol", symbol)]).await
}

/// Fetch exchange info (all markets, contract config).
#[allow(dead_code)]
pub async fn get_exchange_info() -> anyhow::Result<Value> {
    get("/exchangeInfo", &[]).await
}

/// Fetch market data overview.
pub async fn get_market_data() -> anyhow::Result<Value> {
    get("/marketData", &[]).await
}

/// Fetch list of all trading symbols.
#[allow(dead_code)]
pub async fn get_symbols() -> anyhow::Result<Value> {
    get("/marketData/symbols", &[]).await
}

/// Fetch open positions for a wallet address.
pub async fn get_user_positions(address: &str) -> anyhow::Result<Value> {
    get("/userPosition", &[("userAddress", address)]).await
}

/// Fetch account info for a wallet address.
pub async fn get_account(address: &str) -> anyhow::Result<Value> {
    get("/account", &[("address", address)]).await
}

/// Fetch orderbook for a symbol.
pub async fn get_orderbook(symbol: &str, depth: Option<u32>) -> anyhow::Result<Value> {
    let depth_str;
    let mut query = vec![("symbol", symbol)];
    if let Some(d) = depth {
        depth_str = d.to_string();
        query.push(("limit", depth_str.as_str()));
    }
    get("/orderbook", &query).await
}

/// Fetch current funding rate for a symbol.
pub async fn get_funding_rate(symbol: &str) -> anyhow::Result<Value> {
    get("/fundingRate", &[("symbol", symbol)]).await
}
