// Meteora DLMM REST API client
// Base URL: https://dlmm.datapi.meteora.ag
//
// Verified actual API response structure (2026-04):
// GET /pools?page_size=2&sort_key=tvl&order_by=desc
// Returns: { total, pages, current_page, page_size, data: [...] }
// Each pool has: address, name, token_x, token_y, reserve_x, reserve_y,
//   token_x_amount, token_y_amount, pool_config{bin_step, base_fee_pct, max_fee_pct, protocol_fee_pct},
//   dynamic_fee_pct, tvl, current_price, apr, apy, has_farm, farm_apr, farm_apy,
//   volume{30m,1h,2h,4h,12h,24h}, fees{...}, cumulative_metrics{volume,fees},
//   is_blacklisted, launchpad, tags
//
// GET /pools/{address}  — same structure but single object (not wrapped in data array)
//
// GET /positions/{wallet}  — returns user positions

use serde::{Deserialize, Serialize};
use crate::config::API_BASE_URL;

// ── Token Info ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenInfo {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u32,
    #[serde(default)]
    pub is_verified: bool,
    #[serde(default)]
    pub price: Option<f64>,
}

// ── Pool Config ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PoolConfig {
    pub bin_step: u32,
    pub base_fee_pct: f64,
    #[serde(default)]
    pub max_fee_pct: f64,
    #[serde(default)]
    pub protocol_fee_pct: f64,
}

// ── Volume & Fees ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeMetrics {
    #[serde(rename = "30m", default)]
    pub m30: f64,
    #[serde(rename = "1h", default)]
    pub h1: f64,
    #[serde(rename = "2h", default)]
    pub h2: f64,
    #[serde(rename = "4h", default)]
    pub h4: f64,
    #[serde(rename = "12h", default)]
    pub h12: f64,
    #[serde(rename = "24h", default)]
    pub h24: f64,
}

// ── Cumulative Metrics ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CumulativeMetrics {
    #[serde(default)]
    pub volume: f64,
    #[serde(default)]
    pub fees: f64,
}

// ── Pool ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Pool {
    pub address: String,
    pub name: String,
    pub token_x: TokenInfo,
    pub token_y: TokenInfo,
    #[serde(default)]
    pub reserve_x: String,
    #[serde(default)]
    pub reserve_y: String,
    #[serde(default)]
    pub token_x_amount: f64,
    #[serde(default)]
    pub token_y_amount: f64,
    pub pool_config: PoolConfig,
    #[serde(default)]
    pub dynamic_fee_pct: f64,
    #[serde(default)]
    pub tvl: f64,
    #[serde(default)]
    pub current_price: f64,
    #[serde(default)]
    pub apr: f64,
    #[serde(default)]
    pub apy: f64,
    #[serde(default)]
    pub has_farm: bool,
    #[serde(default)]
    pub farm_apr: f64,
    #[serde(default)]
    pub farm_apy: f64,
    #[serde(default)]
    pub volume: Option<TimeMetrics>,
    #[serde(default)]
    pub fees: Option<TimeMetrics>,
    #[serde(default)]
    pub cumulative_metrics: Option<CumulativeMetrics>,
    #[serde(default)]
    pub is_blacklisted: bool,
}

// ── Pools List Response ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PoolsResponse {
    pub total: Option<u64>,
    pub pages: Option<u64>,
    pub current_page: Option<u64>,
    pub page_size: Option<u64>,
    pub data: Vec<Pool>,
}

// ── User Position ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PositionBinData {
    #[serde(default)]
    pub bin_id: i64,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub price_per_token: f64,
    #[serde(default)]
    pub bin_x_amount: f64,
    #[serde(default)]
    pub bin_y_amount: f64,
    #[serde(default)]
    pub bin_liquidity: f64,
    #[serde(default)]
    pub position_liquidity: f64,
    #[serde(default)]
    pub position_x_amount: f64,
    #[serde(default)]
    pub position_y_amount: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserPosition {
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub pair_address: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub total_x_amount: f64,
    #[serde(default)]
    pub total_y_amount: f64,
    #[serde(default)]
    pub fee_x: f64,
    #[serde(default)]
    pub fee_y: f64,
    #[serde(default)]
    pub total_fee_usd: f64,
    #[serde(default)]
    pub total_value_usd: f64,
    #[serde(default)]
    pub lower_bin_id: i64,
    #[serde(default)]
    pub upper_bin_id: i64,
    #[serde(default)]
    pub data: Vec<PositionBinData>,
}

// ── API Client ────────────────────────────────────────────────────────────────

pub struct MeteoraClient {
    pub client: reqwest::Client,
    pub base_url: String,
}

impl MeteoraClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: API_BASE_URL.to_string(),
        }
    }

    /// GET /pools — list pools with filters
    pub async fn get_pools(
        &self,
        page: Option<u32>,
        page_size: Option<u32>,
        sort_key: Option<&str>,
        order_by: Option<&str>,
        search_term: Option<&str>,
    ) -> anyhow::Result<PoolsResponse> {
        let mut url = format!("{}/pools", self.base_url);
        let mut params: Vec<String> = Vec::new();
        if let Some(p) = page {
            params.push(format!("page={p}"));
        }
        if let Some(ps) = page_size {
            params.push(format!("page_size={ps}"));
        }
        if let Some(sk) = sort_key {
            params.push(format!("sort_key={sk}"));
        }
        if let Some(ob) = order_by {
            params.push(format!("order_by={ob}"));
        }
        if let Some(st) = search_term {
            params.push(format!("search_term={}", urlencoding(st)));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Meteora API error {}: {}", status, text);
        }
        serde_json::from_str(&text).map_err(|e| {
            anyhow::anyhow!("Failed to parse pools response: {e}\nRaw: {}", &text[..text.len().min(500)])
        })
    }

    /// GET /pools/{address} — single pool detail
    pub async fn get_pool_detail(&self, address: &str) -> anyhow::Result<Pool> {
        let url = format!("{}/pools/{}", self.base_url, address);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Meteora API error {}: {}", status, text);
        }
        serde_json::from_str(&text).map_err(|e| {
            anyhow::anyhow!("Failed to parse pool detail response: {e}\nRaw: {}", &text[..text.len().min(500)])
        })
    }

    /// GET /positions/{wallet} — user positions by wallet address
    pub async fn get_positions(&self, wallet: &str) -> anyhow::Result<Vec<UserPosition>> {
        let url = format!("{}/positions/{}", self.base_url, wallet);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            // 404 means wallet has no positions — treat as empty list
            if status == reqwest::StatusCode::NOT_FOUND {
                return Ok(Vec::new());
            }
            anyhow::bail!("Meteora API error {}: {}", status, text);
        }
        // Try array first, then object with data field
        if let Ok(list) = serde_json::from_str::<Vec<UserPosition>>(&text) {
            return Ok(list);
        }
        let obj: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            anyhow::anyhow!("Failed to parse positions response: {e}\nRaw: {}", &text[..text.len().min(500)])
        })?;
        if let Some(arr) = obj["data"].as_array() {
            let positions: Vec<UserPosition> = serde_json::from_value(serde_json::Value::Array(arr.clone()))?;
            return Ok(positions);
        }
        // Return empty if no positions found
        Ok(Vec::new())
    }
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                vec![c]
            } else {
                let encoded = format!("%{:02X}", c as u8);
                encoded.chars().collect()
            }
        })
        .collect()
}
