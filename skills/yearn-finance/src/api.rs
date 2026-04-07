// yDaemon REST API client for Yearn Finance
// API base: https://ydaemon.yearn.fi
//
// Verified response structure (from live API call 2026-04-05):
// GET /1/vaults/all?limit=200
// Returns array of vault objects with fields:
//   address, name, symbol, kind, version, decimals, chainID,
//   token: { address, symbol, decimals, name },
//   apr: { netAPR, fees: { performance, management }, points: { weekAgo, monthAgo, inception } },
//   tvl: { tvl, totalAssets },
//   info: { isRetired, isHidden }

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VaultToken {
    pub address: String,
    pub symbol: String,
    pub name: Option<String>,
    pub decimals: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VaultAprFees {
    pub performance: Option<f64>,
    pub management: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VaultAprPoints {
    #[serde(rename = "weekAgo")]
    pub week_ago: Option<f64>,
    #[serde(rename = "monthAgo")]
    pub month_ago: Option<f64>,
    pub inception: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VaultApr {
    #[serde(rename = "netAPR")]
    pub net_apr: Option<f64>,
    pub fees: Option<VaultAprFees>,
    pub points: Option<VaultAprPoints>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VaultTvl {
    pub tvl: Option<f64>,
    #[serde(rename = "totalAssets")]
    pub total_assets: Option<Value>, // can be number or string
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VaultInfo {
    #[serde(rename = "isRetired", default)]
    pub is_retired: bool,
    #[serde(rename = "isHidden", default)]
    pub is_hidden: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Vault {
    pub address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub kind: Option<String>,
    pub version: Option<String>,
    pub decimals: Option<u32>,
    #[serde(rename = "chainID")]
    pub chain_id: Option<u64>,
    pub token: VaultToken,
    pub apr: Option<VaultApr>,
    pub tvl: Option<VaultTvl>,
    pub info: Option<VaultInfo>,
}

impl Vault {
    /// Returns true if the vault is active (not retired and not hidden)
    pub fn is_active(&self) -> bool {
        if let Some(info) = &self.info {
            !info.is_retired && !info.is_hidden
        } else {
            true
        }
    }

    /// Returns net APR as a human-readable string (e.g. "3.29%")
    pub fn apr_display(&self) -> String {
        self.apr
            .as_ref()
            .and_then(|a| a.net_apr)
            .map(|v| format!("{:.2}%", v * 100.0))
            .unwrap_or_else(|| "N/A".to_string())
    }

    /// Returns TVL in USD as a human-readable string
    pub fn tvl_display(&self) -> String {
        self.tvl
            .as_ref()
            .and_then(|t| t.tvl)
            .map(|v| format!("${:.2}", v))
            .unwrap_or_else(|| "N/A".to_string())
    }
}

/// Fetch all vaults for a given chain from yDaemon API
pub async fn fetch_vaults(chain_id: u64) -> Result<Vec<Vault>> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://ydaemon.yearn.fi/{}/vaults/all?limit=500",
        chain_id
    );
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("yDaemon API error: {}", resp.status());
    }

    let vaults: Vec<Vault> = resp.json().await.map_err(|e| {
        anyhow::anyhow!("Failed to parse vault list from yDaemon: {}", e)
    })?;

    Ok(vaults)
}

/// Fetch a single vault by address from yDaemon API
pub async fn fetch_vault(chain_id: u64, vault_address: &str) -> Result<Vault> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://ydaemon.yearn.fi/{}/vaults/{}",
        chain_id, vault_address
    );
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("yDaemon API error fetching vault {}: {}", vault_address, resp.status());
    }

    let vault: Vault = resp.json().await.map_err(|e| {
        anyhow::anyhow!("Failed to parse vault from yDaemon: {}", e)
    })?;

    Ok(vault)
}

/// Resolve vault address by symbol or partial name (case-insensitive)
pub fn find_vault_by_token<'a>(vaults: &'a [Vault], query: &str) -> Option<&'a Vault> {
    let q = query.to_lowercase();
    // Try exact symbol match first
    for v in vaults.iter().filter(|v| v.is_active()) {
        if v.token.symbol.to_lowercase() == q {
            return Some(v);
        }
        if v.symbol.as_deref().map(|s| s.to_lowercase() == q).unwrap_or(false) {
            return Some(v);
        }
    }
    // Partial name/symbol match
    for v in vaults.iter().filter(|v| v.is_active()) {
        if v.token.symbol.to_lowercase().contains(&q)
            || v.name.as_deref().map(|n| n.to_lowercase().contains(&q)).unwrap_or(false)
        {
            return Some(v);
        }
    }
    None
}

/// Resolve vault address from a string (address or symbol)
pub fn find_vault_by_address_or_symbol<'a>(
    vaults: &'a [Vault],
    query: &str,
) -> Option<&'a Vault> {
    let q_lower = query.to_lowercase();
    // Check if it looks like an address
    if q_lower.starts_with("0x") && q_lower.len() >= 40 {
        return vaults.iter().find(|v| v.address.to_lowercase() == q_lower);
    }
    find_vault_by_token(vaults, query)
}
