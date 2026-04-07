//! CIAN REST API client for APY, TVL, and position data.

use serde::Deserialize;
use crate::config::API_BASE_URL;

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

#[derive(Deserialize, Debug, Clone)]
pub struct PoolInfo {
    #[serde(default)]
    pub chain_id: u64,
    #[serde(default)]
    pub pool_name: String,
    #[serde(default)]
    pub pool_address: String,
    #[serde(default)]
    pub pool_type: String,
    #[serde(default)]
    pub apy: String,
    #[serde(default)]
    pub apy_7: String,
    #[serde(default)]
    pub apy_instant_7: String,
    #[serde(default)]
    pub apy_esti_by_points_7: String,
    #[serde(default)]
    pub apy_eco_earn_7: String,
    #[serde(default)]
    pub tvl_usd: String,
    #[serde(default)]
    pub net_tvl_usd: String,
}

#[derive(Deserialize, Debug)]
struct ApiResponse<T> {
    #[serde(default)]
    pub code: String,
    pub data: T,
}

/// Fetch all Ethereum Yield Layer pools from CIAN REST API.
/// Returns empty vec if the API is unavailable (404 / non-JSON response).
pub async fn fetch_pools() -> anyhow::Result<Vec<PoolInfo>> {
    let url = format!("{}/pool/home", API_BASE_URL);
    let client = build_client();
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        eprintln!("[warn] CIAN API returned HTTP {}: {}", resp.status(), url);
        return Ok(vec![]);
    }
    let text = resp.text().await?;
    let body: ApiResponse<Vec<PoolInfo>> = match serde_json::from_str(&text) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[warn] CIAN API response not valid JSON: {}. Body: {}", e, &text[..text.len().min(200)]);
            return Ok(vec![]);
        }
    };
    if body.code != "ok" && !body.code.is_empty() {
        eprintln!("[warn] CIAN API code: {}", body.code);
    }
    Ok(body.data)
}

#[derive(Deserialize, Debug, Default)]
pub struct UserVaultPosition {
    #[serde(default)]
    pub share_balance: String,
    #[serde(default)]
    pub share_value_usd: String,
    #[serde(default)]
    pub underlying_amount: String,
    #[serde(default)]
    pub underlying_token: String,
    #[serde(default)]
    pub pending_redeem_shares: String,
    #[serde(default)]
    pub apy: String,
}

/// Fetch user position in a specific vault from CIAN REST API.
pub async fn fetch_user_position(vault_address: &str, user_address: &str) -> anyhow::Result<serde_json::Value> {
    let url = format!(
        "{}/pool/home/vault/user/{}?user_address={}",
        API_BASE_URL, vault_address, user_address
    );
    let client = build_client();
    let resp = client.get(&url).send().await?;
    let body: serde_json::Value = resp.json().await?;
    Ok(body)
}
