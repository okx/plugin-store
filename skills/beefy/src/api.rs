// Beefy Finance REST API client
// API base: https://api.beefy.finance
//
// Endpoints:
//   GET /vaults  - list all vaults with metadata
//   GET /apy     - APY per vault id (simple)
//   GET /apy/breakdown - detailed APY breakdown
//   GET /tvl     - TVL per chain per vault id
//
// Vault object key fields:
//   id, name, token, tokenAddress, earnContractAddress, chain,
//   status ("active"|"eol"|"paused"), platformId, assets[], strategyTypeId

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{BEEFY_API_BASE, chain_id_to_beefy_name};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BeefyVault {
    pub id: String,
    pub name: Option<String>,
    pub token: Option<String>,
    #[serde(rename = "tokenAddress")]
    pub token_address: Option<String>,
    #[serde(rename = "tokenDecimals")]
    pub token_decimals: Option<u32>,
    #[serde(rename = "earnContractAddress")]
    pub earn_contract_address: Option<String>,
    pub chain: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "platformId")]
    pub platform_id: Option<String>,
    pub assets: Option<Vec<String>>,
    #[serde(rename = "strategyTypeId")]
    pub strategy_type_id: Option<String>,
}

impl BeefyVault {
    pub fn is_active(&self) -> bool {
        self.status.as_deref() == Some("active")
    }
}

fn build_client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    // Respect system proxy settings (needed in sandbox environments)
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
        if let Ok(proxy) = reqwest::Proxy::https(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    if let Ok(proxy_url) = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")) {
        if let Ok(proxy) = reqwest::Proxy::http(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap_or_default()
}

/// Fetch all vaults and filter by chain
pub async fn fetch_vaults(chain_id: u64) -> Result<Vec<BeefyVault>> {
    let chain_name = chain_id_to_beefy_name(chain_id)
        .ok_or_else(|| anyhow::anyhow!("Unsupported chain ID: {}", chain_id))?;

    let client = build_client();
    let url = format!("{}/vaults", BEEFY_API_BASE);
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Beefy API error: {}", resp.status());
    }

    let all: Vec<BeefyVault> = resp.json().await.map_err(|e| {
        anyhow::anyhow!("Failed to parse vaults: {}", e)
    })?;

    Ok(all.into_iter().filter(|v| v.chain.as_deref() == Some(chain_name)).collect())
}

/// Fetch APY data for all vaults
pub async fn fetch_apy() -> Result<std::collections::HashMap<String, Option<f64>>> {
    let client = build_client();
    let url = format!("{}/apy", BEEFY_API_BASE);
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Beefy APY API error: {}", resp.status());
    }

    // APY values can be numbers, strings, or null
    let raw: std::collections::HashMap<String, Value> = resp.json().await?;
    let result = raw.into_iter().map(|(k, v)| {
        let apy = match &v {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.parse::<f64>().ok(),
            _ => None,
        };
        (k, apy)
    }).collect();
    Ok(result)
}

/// Fetch TVL data per chain
pub async fn fetch_tvl(chain_id: u64) -> Result<std::collections::HashMap<String, f64>> {
    let client = build_client();
    let url = format!("{}/tvl", BEEFY_API_BASE);
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Beefy TVL API error: {}", resp.status());
    }

    let raw: std::collections::HashMap<String, Value> = resp.json().await?;

    // TVL is keyed by chain ID as string: {"8453": {"vault-id": tvl_value}}
    let chain_key = chain_id.to_string();
    if let Some(chain_data) = raw.get(&chain_key) {
        if let Some(obj) = chain_data.as_object() {
            let result = obj.iter().filter_map(|(k, v)| {
                let tvl = match v {
                    Value::Number(n) => n.as_f64(),
                    _ => None,
                }?;
                Some((k.clone(), tvl))
            }).collect();
            return Ok(result);
        }
    }
    Ok(std::collections::HashMap::new())
}

/// Find a vault by ID or earn contract address
pub fn find_vault<'a>(vaults: &'a [BeefyVault], query: &str) -> Option<&'a BeefyVault> {
    let q_lower = query.to_lowercase();
    // Exact ID match
    if let Some(v) = vaults.iter().find(|v| v.id.to_lowercase() == q_lower) {
        return Some(v);
    }
    // Earn contract address match
    if q_lower.starts_with("0x") {
        if let Some(v) = vaults.iter().find(|v| {
            v.earn_contract_address.as_deref().map(|a| a.to_lowercase() == q_lower).unwrap_or(false)
        }) {
            return Some(v);
        }
    }
    // Partial ID match (active first)
    if let Some(v) = vaults.iter().filter(|v| v.is_active()).find(|v| v.id.to_lowercase().contains(&q_lower)) {
        return Some(v);
    }
    None
}
