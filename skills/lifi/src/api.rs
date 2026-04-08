use anyhow::Result;
use serde_json::Value;

use crate::config::LIFI_API;

/// Build an HTTP client that honours system proxy environment variables.
/// reqwest by default does not read HTTPS_PROXY in subprocess environments.
pub fn build_client() -> reqwest::Client {
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

/// GET /chains — list all supported chains
pub async fn get_chains() -> Result<Value> {
    let client = build_client();
    let url = format!("{}/chains", LIFI_API);
    let resp = client.get(&url).send().await?.error_for_status()?;
    Ok(resp.json::<Value>().await?)
}

/// GET /tokens — list tokens on specified chains
/// chains: comma-separated chain IDs, e.g. "1,8453,42161"
pub async fn get_tokens(chains: &str) -> Result<Value> {
    let client = build_client();
    let url = format!("{}/tokens?chains={}", LIFI_API, chains);
    let resp = client.get(&url).send().await?.error_for_status()?;
    Ok(resp.json::<Value>().await?)
}

/// GET /quote — get a bridge/swap quote
pub async fn get_quote(
    from_chain: u64,
    to_chain: u64,
    from_token: &str,
    to_token: &str,
    from_amount: &str,
    from_address: &str,
    slippage: f64,
) -> Result<Value> {
    let client = build_client();
    let url = format!(
        "{}/quote?fromChain={}&toChain={}&fromToken={}&toToken={}&fromAmount={}&fromAddress={}&slippage={}",
        LIFI_API, from_chain, to_chain, from_token, to_token, from_amount, from_address, slippage
    );
    let resp = client.get(&url).send().await?.error_for_status()?;
    Ok(resp.json::<Value>().await?)
}

/// GET /status — check cross-chain transfer status
pub async fn get_status(
    tx_hash: &str,
    from_chain: Option<u64>,
    to_chain: Option<u64>,
    bridge: Option<&str>,
) -> Result<Value> {
    let client = build_client();
    let mut url = format!("{}/status?txHash={}", LIFI_API, tx_hash);
    if let Some(fc) = from_chain {
        url.push_str(&format!("&fromChain={}", fc));
    }
    if let Some(tc) = to_chain {
        url.push_str(&format!("&toChain={}", tc));
    }
    if let Some(b) = bridge {
        url.push_str(&format!("&bridge={}", b));
    }
    let resp = client.get(&url).send().await?.error_for_status()?;
    Ok(resp.json::<Value>().await?)
}

/// GET /tools — list available bridges and DEXes
pub async fn get_tools(chains: Option<&str>) -> Result<Value> {
    let client = build_client();
    let mut url = format!("{}/tools", LIFI_API);
    if let Some(c) = chains {
        url.push_str(&format!("?chains={}", c));
    }
    let resp = client.get(&url).send().await?.error_for_status()?;
    Ok(resp.json::<Value>().await?)
}

/// GET /token — get info for a single token
#[allow(dead_code)]
pub async fn get_token(chain: u64, token: &str) -> Result<Value> {
    let client = build_client();
    let url = format!("{}/token?chain={}&token={}", LIFI_API, chain, token);
    let resp = client.get(&url).send().await?.error_for_status()?;
    Ok(resp.json::<Value>().await?)
}
