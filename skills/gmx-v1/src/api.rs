use anyhow::Result;
use serde_json::Value;

fn build_client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .or_else(|_| std::env::var("http_proxy"))
    {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap_or_default()
}

/// Fetch current oracle prices for all tokens from GMX V1 API.
pub async fn get_prices(api_base: &str) -> Result<Value> {
    let url = format!("{}/prices/tickers", api_base);
    let resp = build_client().get(&url).send().await?.json::<Value>().await?;
    Ok(resp)
}

/// Fetch open perpetual positions for a wallet address.
pub async fn get_positions(api_base: &str, account: &str) -> Result<Value> {
    let url = format!("{}/positions?account={}", api_base, account);
    let resp = build_client().get(&url).send().await?.json::<Value>().await?;
    Ok(resp)
}
