/// Orca REST API client
///
/// Actual API response sample (from https://api.orca.so/v1/whirlpool/list):
/// {
///   "whirlpools": [{
///     "address": "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE",
///     "tokenA": { "mint": "So111...", "symbol": "SOL", "name": "Solana", "decimals": 9, "logoURI": "...", "coingeckoId": "solana", "whitelisted": true, "poolToken": false, "token2022": false },
///     "tokenB": { "mint": "EPjF...", "symbol": "USDC", ... },
///     "whitelisted": true,
///     "token2022": false,
///     "tickSpacing": 4,
///     "price": 127.496,
///     "lpFeeRate": 0.0004,
///     "protocolFeeRate": 0.13,
///     "whirlpoolsConfig": "2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ",
///     "modifiedTimeMs": 1742938567559,
///     "tvl": 32526289.16,
///     "volume": { "day": ..., "week": ..., "month": ... },
///     "feeApr": { "day": ..., "week": ..., "month": ... },
///     "totalApr": { "day": ..., "week": ..., "month": ... }
///   }],
///   "hasMore": false
/// }
use crate::config::ORCA_API_BASE;
use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenInfo {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    #[serde(default)]
    pub logo_uri: Option<String>,
    #[serde(default)]
    pub coingecko_id: Option<String>,
    #[serde(default)]
    pub whitelisted: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VolumeStats {
    pub day: Option<f64>,
    pub week: Option<f64>,
    pub month: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AprStats {
    pub day: Option<f64>,
    pub week: Option<f64>,
    pub month: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WhirlpoolPool {
    pub address: String,
    pub token_a: TokenInfo,
    pub token_b: TokenInfo,
    #[serde(default)]
    pub whitelisted: bool,
    pub tick_spacing: u32,
    pub price: Option<f64>,
    pub lp_fee_rate: Option<f64>,
    pub protocol_fee_rate: Option<f64>,
    pub tvl: Option<f64>,
    pub volume: Option<VolumeStats>,
    pub fee_apr: Option<AprStats>,
    pub total_apr: Option<AprStats>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WhirlpoolListResponse {
    whirlpools: Vec<WhirlpoolPool>,
    #[allow(dead_code)]
    has_more: Option<bool>,
}

/// Fetch all whirlpool pools from Orca v1 API.
pub async fn fetch_all_pools(client: &reqwest::Client) -> anyhow::Result<Vec<WhirlpoolPool>> {
    let url = format!("{}/whirlpool/list", ORCA_API_BASE);
    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch Orca pool list")?;
    if !resp.status().is_success() {
        anyhow::bail!("Orca API returned {}: {}", resp.status(), url);
    }
    let data: WhirlpoolListResponse = resp
        .json()
        .await
        .context("Failed to parse Orca pool list response")?;
    Ok(data.whirlpools)
}

/// Filter pools by token pair (either direction).
pub fn filter_pools_by_pair<'a>(
    pools: &'a [WhirlpoolPool],
    token_a: &str,
    token_b: &str,
) -> Vec<&'a WhirlpoolPool> {
    pools
        .iter()
        .filter(|p| {
            let a = p.token_a.mint.as_str();
            let b = p.token_b.mint.as_str();
            (a == token_a && b == token_b) || (a == token_b && b == token_a)
        })
        .collect()
}

/// Compute a simple price impact estimate: (amount_in_usd / pool_tvl) * 100.
/// This is a rough approximation, not based on CLMM math.
pub fn estimate_price_impact(amount_in_usd: f64, pool_tvl: f64) -> f64 {
    if pool_tvl <= 0.0 {
        return 100.0;
    }
    // For CLMM, concentrated liquidity means impact is higher than AMM formula.
    // Use 2x multiplier as conservative estimate.
    (amount_in_usd / pool_tvl) * 100.0 * 2.0
}
