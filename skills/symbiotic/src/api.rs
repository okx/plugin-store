/// Symbiotic REST API client
/// Base URL: https://app.symbiotic.fi/api/v2
///
/// Real API response sample (vault object):
/// {
///   "address": "0xC329400492c6ff2438472D4651Ad17389fCb843a",
///   "token": { "address": "0x7f39...", "decimals": 18, "usdPrice": 2516.47, "symbol": "wstETH", "meta": {...} },
///   "legacy": true,
///   "meta": { "name": "wstETH", "description": "...", "tags": [...], "links": [...] },
///   "totalSupply": "23553925961750249323677",
///   "restricted": false,
///   "slashable": false,
///   "tvl": 59272873.71,
///   "vaultRewardsApr": 0.0237,
///   "vaultRewards": [...],
///   "points": [...],
///   "earned": []
/// }

use serde::Deserialize;
use crate::config::SYMBIOTIC_API;

#[derive(Debug, Deserialize, Clone)]
pub struct TokenMeta {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VaultToken {
    pub address: String,
    pub decimals: u8,
    #[serde(rename = "usdPrice")]
    pub usd_price: Option<f64>,
    pub symbol: String,
    pub meta: Option<TokenMeta>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VaultMeta {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RewardInfo {
    pub apr: Option<f64>,
    #[serde(rename = "rewardToken")]
    pub reward_token: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Vault {
    pub address: String,
    pub token: VaultToken,
    pub meta: Option<VaultMeta>,
    pub tvl: Option<f64>,
    #[serde(rename = "vaultRewardsApr")]
    pub vault_rewards_apr: Option<f64>,
    #[serde(rename = "vaultRewards")]
    pub vault_rewards: Option<Vec<RewardInfo>>,
    pub restricted: Option<bool>,
    pub slashable: Option<bool>,
    pub legacy: Option<bool>,
    #[serde(rename = "totalSupply")]
    pub total_supply: Option<String>,
}

/// Fetch all vaults from Symbiotic API.
pub async fn fetch_vaults(limit: u64) -> anyhow::Result<Vec<Vault>> {
    let client = reqwest::Client::new();
    let url = format!("{}/vaults?limit={}", SYMBIOTIC_API, limit);
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;
    let vaults: Vec<Vault> = resp.json().await
        .map_err(|e| anyhow::anyhow!("Failed to parse vaults response: {}", e))?;
    Ok(vaults)
}

/// Find a vault by collateral token symbol (case-insensitive).
/// Prefers non-legacy vaults (IVault interface) over legacy DefaultCollateral vaults.
pub fn find_vault_by_token<'a>(vaults: &'a [Vault], token_symbol: &str) -> Option<&'a Vault> {
    let sym_lower = token_symbol.to_lowercase();
    // First try non-legacy, unrestricted vaults
    if let Some(v) = vaults.iter().find(|v| {
        v.token.symbol.to_lowercase() == sym_lower
            && !v.legacy.unwrap_or(false)
            && !v.restricted.unwrap_or(false)
    }) {
        return Some(v);
    }
    // Fall back to any vault with matching symbol
    vaults.iter().find(|v| v.token.symbol.to_lowercase() == sym_lower)
}

/// Find a vault by address (case-insensitive).
pub fn find_vault_by_address<'a>(vaults: &'a [Vault], address: &str) -> Option<&'a Vault> {
    let addr_lower = address.to_lowercase();
    vaults.iter().find(|v| v.address.to_lowercase() == addr_lower)
}

/// Format APR as percentage string.
pub fn format_apr(apr: Option<f64>) -> String {
    match apr {
        Some(a) if a > 0.0 => format!("{:.2}%", a * 100.0),
        Some(_) => "0.00%".to_string(),
        None => "N/A".to_string(),
    }
}

/// Format TVL as readable USD string.
pub fn format_tvl(tvl: Option<f64>) -> String {
    match tvl {
        Some(t) if t >= 1_000_000.0 => format!("${:.1}M", t / 1_000_000.0),
        Some(t) if t >= 1_000.0 => format!("${:.1}K", t / 1_000.0),
        Some(t) => format!("${:.0}", t),
        None => "N/A".to_string(),
    }
}

/// Convert human-readable amount to raw token units.
pub fn parse_amount(amount_str: &str, decimals: u8) -> anyhow::Result<u128> {
    let parts: Vec<&str> = amount_str.split('.').collect();
    let integer_part: u128 = parts[0].parse()?;
    let frac_str = if parts.len() > 1 { parts[1] } else { "" };
    let frac_len = frac_str.len() as u8;
    if frac_len > decimals {
        return Err(anyhow::anyhow!("Too many decimal places: {} (max {})", frac_len, decimals));
    }
    let frac_padded = format!("{:0<width$}", frac_str, width = decimals as usize);
    let frac_val: u128 = frac_padded.parse().unwrap_or(0);
    let multiplier = 10u128.pow(decimals as u32);
    Ok(integer_part * multiplier + frac_val)
}
