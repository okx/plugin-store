use anyhow::Result;
use serde_json::Value;

use crate::config::NAVI_OPEN_API;

/// Fetch all pool data from NAVI open API.
/// Returns raw JSON array of pool objects.
pub async fn fetch_pools() -> Result<Vec<Value>> {
    let client = reqwest::Client::builder()
        .user_agent("navi-protocol-cli/0.1.0")
        .build()?;
    let url = format!("{}/api/navi/pools", NAVI_OPEN_API);
    let text = client.get(&url).send().await?.text().await?;
    let resp: Value = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Failed to parse pools response: {}", e))?;
    let pools = resp["data"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Unexpected pools response shape — 'data' field not array"))?
        .clone();
    Ok(pools)
}

/// Fetch the latest on-chain protocol package ID.
pub async fn fetch_latest_package_id() -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("navi-protocol-cli/0.1.0")
        .build()?;
    let url = format!("{}/api/package", NAVI_OPEN_API);
    let text = client.get(&url).send().await?.text().await?;
    let resp: Value = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Failed to parse package response: {}", e))?;
    let pkg = resp["packageId"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No packageId in response"))?
        .to_string();
    Ok(pkg)
}

/// Parsed pool data from the API.
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub id: u64,
    pub symbol: String,
    pub coin_type: String,
    pub pool_id: String,
    pub reserve_id: String,
    pub total_supply_raw: String,
    pub total_borrow_raw: String,
    pub supply_index_raw: String,
    pub borrow_index_raw: String,
    pub supply_rate_raw: String,
    pub borrow_rate_raw: String,
    pub oracle_price: f64,
    pub decimals: u32,
    pub ltv_raw: String,
    pub is_isolated: bool,
    pub supply_incentive_apy: f64,
    pub borrow_incentive_apy: f64,
}

/// Ray = 1e27
const RAY: f64 = 1e27;

fn parse_f64(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

impl PoolInfo {
    /// Supply APY as a percentage (e.g. 3.14 means 3.14%)
    pub fn supply_apy_pct(&self) -> f64 {
        let rate = self.supply_rate_raw.parse::<f64>().unwrap_or(0.0);
        rate / RAY * 100.0
    }

    /// Borrow APY as a percentage
    pub fn borrow_apy_pct(&self) -> f64 {
        let rate = self.borrow_rate_raw.parse::<f64>().unwrap_or(0.0);
        rate / RAY * 100.0
    }

    /// Total supply in token units (adjusted by index)
    pub fn total_supply_tokens(&self) -> f64 {
        let raw = self.total_supply_raw.parse::<f64>().unwrap_or(0.0);
        let index = self.supply_index_raw.parse::<f64>().unwrap_or(RAY);
        let decimals = 10_f64.powi(self.decimals as i32);
        (raw * index / RAY) / decimals
    }

    /// Total borrow in token units (adjusted by index)
    pub fn total_borrow_tokens(&self) -> f64 {
        let raw = self.total_borrow_raw.parse::<f64>().unwrap_or(0.0);
        let index = self.borrow_index_raw.parse::<f64>().unwrap_or(RAY);
        let decimals = 10_f64.powi(self.decimals as i32);
        (raw * index / RAY) / decimals
    }

    /// Utilization rate as a percentage
    pub fn utilization_pct(&self) -> f64 {
        let supply = self.total_supply_tokens();
        let borrow = self.total_borrow_tokens();
        if supply > 0.0 {
            (borrow / supply) * 100.0
        } else {
            0.0
        }
    }

    /// Max LTV as a percentage
    pub fn ltv_pct(&self) -> f64 {
        let raw = self.ltv_raw.parse::<f64>().unwrap_or(0.0);
        raw / RAY * 100.0
    }
}

pub fn parse_pool(v: &Value) -> Option<PoolInfo> {
    let id = v["id"].as_u64()?;
    let symbol = v["token"]["symbol"].as_str()?.to_string();
    let coin_type = v["coinType"].as_str()
        .or_else(|| v["suiCoinType"].as_str())
        .unwrap_or("")
        .to_string();
    let pool_id = v["contract"]["pool"].as_str().unwrap_or("").to_string();
    let reserve_id = v["contract"]["reserveId"].as_str().unwrap_or("").to_string();
    let total_supply_raw = v["totalSupply"].as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| parse_f64(&v["totalSupply"]).to_string());
    let total_borrow_raw = v["totalBorrow"].as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| parse_f64(&v["totalBorrow"]).to_string());
    let supply_index_raw = v["currentSupplyIndex"].as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| parse_f64(&v["currentSupplyIndex"]).to_string());
    let borrow_index_raw = v["currentBorrowIndex"].as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| parse_f64(&v["currentBorrowIndex"]).to_string());
    let supply_rate_raw = v["currentSupplyRate"].as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| parse_f64(&v["currentSupplyRate"]).to_string());
    let borrow_rate_raw = v["currentBorrowRate"].as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| parse_f64(&v["currentBorrowRate"]).to_string());
    let oracle_price = v["oracle"]["price"].as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or_else(|| parse_f64(&v["oracle"]["price"]));
    let decimals = v["token"]["decimals"].as_u64().unwrap_or(9) as u32;
    let ltv_raw = v["ltv"].as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| parse_f64(&v["ltv"]).to_string());
    let is_isolated = v["isIsolated"].as_bool().unwrap_or(false);
    let supply_incentive_apy = v["supplyIncentiveApyInfo"]["apy"]
        .as_f64()
        .unwrap_or(0.0);
    let borrow_incentive_apy = v["borrowIncentiveApyInfo"]["apy"]
        .as_f64()
        .unwrap_or(0.0);

    Some(PoolInfo {
        id,
        symbol,
        coin_type,
        pool_id,
        reserve_id,
        total_supply_raw,
        total_borrow_raw,
        supply_index_raw,
        borrow_index_raw,
        supply_rate_raw,
        borrow_rate_raw,
        oracle_price,
        decimals,
        ltv_raw,
        is_isolated,
        supply_incentive_apy,
        borrow_incentive_apy,
    })
}
