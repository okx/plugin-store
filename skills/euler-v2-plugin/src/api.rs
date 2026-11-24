//! Euler v2 API client (app.euler.finance).
//!
//! All endpoints are public GETs; no auth needed. Cloudflare-fronted, so requests
//! without a real-browser User-Agent get 403'd. We always send a UA + Accept header.
//!
//! Several struct fields (lens addresses, factory addresses) are deserialized for
//! future v0.2 use (lens-contract integration for richer position data) but unused
//! in v0.1, hence the module-level dead_code allow.

#![allow(dead_code)]

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::config::Urls;

const UA: &str = "Mozilla/5.0 (compatible; euler-v2-plugin/0.1) Chrome/120.0";

fn build_get(url: &str) -> reqwest::RequestBuilder {
    Client::new()
        .get(url)
        .header("User-Agent", UA)
        .header("Accept", "application/json")
        .header("Referer", "https://app.euler.finance/")
}

/// Sub-struct of `/api/euler-chains` containing the canonical contract addresses
/// per chain. We pull only the fields the plugin uses; unknown fields are ignored
/// by serde so additions in the API don't break us.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainAddresses {
    #[serde(default)]
    pub core_addrs: CoreAddrs,
    #[serde(default)]
    pub lens_addrs: LensAddrs,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreAddrs {
    pub evc:                    Option<String>,
    pub e_vault_factory:        Option<String>,
    pub e_vault_implementation: Option<String>,
    pub euler_earn_factory:     Option<String>,
    pub permit2:                Option<String>,
    pub protocol_config:        Option<String>,
    pub balance_tracker:        Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LensAddrs {
    pub account_lens:           Option<String>,
    pub vault_lens:             Option<String>,
    pub euler_earn_vault_lens:  Option<String>,
    pub irm_lens:               Option<String>,
    pub oracle_lens:            Option<String>,
    pub utils_lens:             Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EulerChain {
    pub chain_id:  u64,
    pub name:      String,
    #[serde(default)]
    pub viem_name: Option<String>,
    pub status:    String,                 // "production" / "staging" / "testnet"
    pub addresses: ChainAddresses,
}

/// Fetch `/api/euler-chains` (no params, no chain filter — returns all).
pub async fn get_chains() -> Result<Vec<EulerChain>> {
    let url = format!("{}/api/euler-chains", Urls::euler_api());
    let resp = build_get(&url).send().await
        .context("Euler /api/euler-chains request failed")?;
    if !resp.status().is_success() {
        anyhow::bail!("Euler /api/euler-chains returned HTTP {}", resp.status());
    }
    resp.json::<Vec<EulerChain>>().await
        .context("Parsing /api/euler-chains response")
}

/// Fetch `/api/vaults?chainId=<id>`. Returns the raw JSON because the schema
/// has 4 vault categories with nested big-int fields (`{"__bi": "..."}`) — easier
/// to parse selectively at the call site than to type out the full Rust schema.
pub async fn get_vaults_raw(chain_id: u64) -> Result<Value> {
    let url = format!("{}/api/vaults?chainId={}", Urls::euler_api(), chain_id);
    let resp = build_get(&url).send().await
        .context("Euler /api/vaults request failed")?;
    if !resp.status().is_success() {
        anyhow::bail!("Euler /api/vaults?chainId={} returned HTTP {}", chain_id, resp.status());
    }
    resp.json::<Value>().await.context("Parsing /api/vaults response")
}

/// Fetch `/api/token-list?chainId=<id>`.
pub async fn get_token_list_raw(chain_id: u64) -> Result<Value> {
    let url = format!("{}/api/token-list?chainId={}", Urls::euler_api(), chain_id);
    let resp = build_get(&url).send().await
        .context("Euler /api/token-list request failed")?;
    if !resp.status().is_success() {
        anyhow::bail!("Euler /api/token-list?chainId={} returned HTTP {}", chain_id, resp.status());
    }
    resp.json::<Value>().await.context("Parsing /api/token-list response")
}

/// Fetch Merkl reward proofs for a user on a chain via the official Merkl API.
/// Returns `(token_address, amount_decimal_str, proofs_hex)` per claimable reward.
/// Empty Vec if user has no claimable rewards.
pub async fn get_merkl_rewards(chain_id: u64, user_addr: &str) -> Result<Vec<MerklReward>> {
    // Note: Merkl API is hosted at api.merkl.xyz — this domain must be in plugin.yaml api_calls.
    let url = format!(
        "https://api.merkl.xyz/v4/users/{}/rewards?chainId={}",
        user_addr, chain_id
    );
    let resp = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", UA)
        .header("Accept", "application/json")
        .send()
        .await
        .with_context(|| format!("Merkl API request failed: {}", url))?;
    if !resp.status().is_success() {
        anyhow::bail!("Merkl API returned HTTP {}", resp.status());
    }
    let data: Vec<Value> = resp.json().await
        .context("Parsing Merkl rewards response")?;
    let mut out = Vec::new();
    for chain_entry in &data {
        let rewards = chain_entry["rewards"].as_array().cloned().unwrap_or_default();
        for r in rewards {
            let token  = r["token"]["address"].as_str().unwrap_or("").to_lowercase();
            let symbol = r["token"]["symbol"].as_str().unwrap_or("?").to_string();
            let amount = r["amount"].as_str().unwrap_or("0").to_string();
            let claimed = r["claimed"].as_str().unwrap_or("0").to_string();
            let proofs: Vec<String> = r["proofs"].as_array().cloned().unwrap_or_default()
                .iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
            // amount is the cumulative authorized total; claimable = amount - claimed
            let claimable_u128 = amount.parse::<u128>().unwrap_or(0)
                .saturating_sub(claimed.parse::<u128>().unwrap_or(0));
            if claimable_u128 == 0 || token.is_empty() { continue; }
            out.push(MerklReward {
                token, symbol,
                cumulative_amount: amount,
                claimable_raw: claimable_u128,
                proofs,
            });
        }
    }
    Ok(out)
}

#[derive(Debug, Clone)]
pub struct MerklReward {
    pub token:             String,        // 0x-prefixed
    pub symbol:            String,
    pub cumulative_amount: String,         // total authorized (as decimal string, what claim() takes)
    pub claimable_raw:     u128,           // amount - claimed; we surface this for UX
    pub proofs:            Vec<String>,    // 0x-prefixed bytes32 each
}

/// Convenience: get the address book for a single chain.
pub async fn get_chain(chain_id: u64) -> Result<EulerChain> {
    let chains = get_chains().await?;
    chains.into_iter()
        .find(|c| c.chain_id == chain_id)
        .ok_or_else(|| anyhow::anyhow!(
            "Chain {} not found in Euler /api/euler-chains. \
             It may not be supported by Euler v2 yet, or the API is returning a different list. \
             Supported in this plugin: 1 (Ethereum), 8453 (Base), 42161 (Arbitrum).",
            chain_id
        ))
}
