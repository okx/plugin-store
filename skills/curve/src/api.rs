// api.rs — Curve REST API client
use serde::Deserialize;

const CURVE_API_BASE: &str = "https://api.curve.finance/api";

// Custom deserializer for fields that may be number or string in JSON
mod deser_number_or_string {
    use serde::{self, Deserialize, Deserializer};
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        use serde_json::Value;
        Ok(match Option::<Value>::deserialize(d)? {
            Some(Value::String(s)) => Some(s),
            Some(Value::Number(n)) => Some(n.to_string()),
            _ => None,
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoinInfo {
    pub address: String,
    pub symbol: String,
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub decimals: Option<String>,
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub usd_price: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PoolData {
    pub id: String,
    pub address: String,
    pub name: String,
    pub coins: Vec<CoinInfo>,
    #[serde(default)]
    pub usd_total: Option<f64>,
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub virtual_price: Option<String>,
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub fee: Option<String>,
    #[serde(default, rename = "gaugeCrvApy")]
    pub gauge_crv_apy: Option<Vec<Option<f64>>>,
    #[serde(default, rename = "latestDailyApyPcent")]
    pub latest_daily_apy_pcent: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct PoolsApiResponse {
    data: Option<PoolsData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PoolsData {
    pool_data: Option<Vec<PoolData>>,
}

/// Fetch all pools for a given chain and registry
pub async fn get_pools(chain_name: &str, registry_id: &str) -> anyhow::Result<Vec<PoolData>> {
    let url = format!("{}/getPools/{}/{}", CURVE_API_BASE, chain_name, registry_id);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "curve-plugin/0.1.0")
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("Curve API error {}: {}", status, url);
    }
    let api_resp: PoolsApiResponse = resp.json().await?;
    Ok(api_resp
        .data
        .and_then(|d| d.pool_data)
        .unwrap_or_default())
}

/// Fetch pools from main + factory registries and combine
pub async fn get_all_pools(chain_name: &str) -> anyhow::Result<Vec<PoolData>> {
    let registries = ["main", "crypto", "factory", "factory-crypto"];
    let client = reqwest::Client::new();
    let mut all = Vec::new();
    for registry in &registries {
        let url = format!("{}/getPools/{}/{}", CURVE_API_BASE, chain_name, registry);
        let resp = client
            .get(&url)
            .header("User-Agent", "curve-plugin/0.1.0")
            .send()
            .await;
        match resp {
            Ok(r) if r.status().is_success() => {
                if let Ok(parsed) = r.json::<PoolsApiResponse>().await {
                    if let Some(data) = parsed.data.and_then(|d| d.pool_data) {
                        all.extend(data);
                    }
                }
            }
            _ => {} // skip failed registries gracefully
        }
    }
    Ok(all)
}

/// Find a pool by address (case-insensitive) from a list of pools
pub fn find_pool_by_address<'a>(pools: &'a [PoolData], address: &str) -> Option<&'a PoolData> {
    let addr_lower = address.to_lowercase();
    pools.iter().find(|p| p.address.to_lowercase() == addr_lower)
}

/// Find pools that contain both token_in and token_out, sorted by TVL descending.
pub fn find_pools_for_pair<'a>(
    pools: &'a [PoolData],
    token_in: &str,
    token_out: &str,
) -> Vec<&'a PoolData> {
    let tin = token_in.to_lowercase();
    let tout = token_out.to_lowercase();
    let mut matched: Vec<&'a PoolData> = pools
        .iter()
        .filter(|p| {
            let has_in = p.coins.iter().any(|c| c.address.to_lowercase() == tin);
            let has_out = p.coins.iter().any(|c| c.address.to_lowercase() == tout);
            has_in && has_out
        })
        .collect();
    // Sort by TVL descending so matching_pools[0] is always the deepest pool
    matched.sort_by(|a, b| {
        b.usd_total
            .unwrap_or(0.0)
            .partial_cmp(&a.usd_total.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matched
}

/// Get coin index within a pool
pub fn coin_index(pool: &PoolData, token_addr: &str) -> Option<usize> {
    let addr_lower = token_addr.to_lowercase();
    pool.coins
        .iter()
        .position(|c| c.address.to_lowercase() == addr_lower)
}
