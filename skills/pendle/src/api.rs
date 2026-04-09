use anyhow::Context;
use serde::Deserialize;
use serde_json::Value;

use crate::config::PENDLE_API_BASE;

// ─── Custom deserializer: accept JSON number or string ────────────────────────
mod deser_number_or_string {
    use serde::{Deserialize, Deserializer};
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        use serde_json::Value;
        Ok(match Option::<Value>::deserialize(d)? {
            Some(Value::String(s)) => Some(s),
            Some(Value::Number(n)) => Some(n.to_string()),
            _ => None,
        })
    }
}

// ─── Market structures ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketLiquidity {
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub usd: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradingVolume {
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub usd: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Market {
    pub address: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "chainId")]
    pub chain_id: Option<u64>,
    pub expiry: Option<String>,
    pub pt: Option<Value>,
    pub yt: Option<Value>,
    pub sy: Option<Value>,
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub implied_apy: Option<String>,
    pub liquidity: Option<MarketLiquidity>,
    pub trading_volume: Option<TradingVolume>,
}

#[derive(Debug, Deserialize)]
pub struct MarketsResponse {
    pub results: Option<Vec<Value>>,
    pub total: Option<u64>,
}

// ─── Position structures ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub chain_id: Option<u64>,
    pub market_address: Option<String>,
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub pt_balance: Option<String>,
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub yt_balance: Option<String>,
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub lp_balance: Option<String>,
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub value_usd: Option<String>,
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub implied_apy: Option<String>,
}

// ─── HTTP client ──────────────────────────────────────────────────────────────

fn build_client(api_key: Option<&str>) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if let Some(key) = api_key {
        let mut headers = reqwest::header::HeaderMap::new();
        let auth_val = format!("Bearer {}", key);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&auth_val)?,
        );
        builder = builder.default_headers(headers);
    }
    Ok(builder.build()?)
}

// ─── API functions ────────────────────────────────────────────────────────────

/// GET /v2/markets/all — list Pendle markets
pub async fn list_markets(
    chain_id: Option<u64>,
    is_active: Option<bool>,
    skip: u64,
    limit: u64,
    api_key: Option<&str>,
) -> anyhow::Result<Value> {
    let client = build_client(api_key)?;
    let mut url = format!("{}/v2/markets/all?skip={}&limit={}", PENDLE_API_BASE, skip, limit);
    if let Some(cid) = chain_id {
        url.push_str(&format!("&chainId={}", cid));
    }
    if let Some(active) = is_active {
        url.push_str(&format!("&isActive={}", active));
    }
    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to call Pendle markets API")?;
    let body: Value = resp.json().await.context("Failed to parse markets response")?;
    Ok(body)
}

/// GET /v3/{chainId}/markets/{marketAddress}/historical-data
pub async fn get_market(
    chain_id: u64,
    market_address: &str,
    time_frame: Option<&str>,
    api_key: Option<&str>,
) -> anyhow::Result<Value> {
    let client = build_client(api_key)?;
    let mut url = format!(
        "{}/v3/{}/markets/{}/historical-data",
        PENDLE_API_BASE, chain_id, market_address
    );
    if let Some(tf) = time_frame {
        url.push_str(&format!("?time_frame={}", tf));
    }
    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to call Pendle market detail API")?;
    let body: Value = resp.json().await.context("Failed to parse market detail response")?;
    Ok(body)
}

/// GET /v1/dashboard/positions/database/{user}
pub async fn get_positions(
    user: &str,
    filter_usd: Option<f64>,
    api_key: Option<&str>,
) -> anyhow::Result<Value> {
    let client = build_client(api_key)?;
    let mut url = format!(
        "{}/v1/dashboard/positions/database/{}",
        PENDLE_API_BASE, user
    );
    if let Some(min_usd) = filter_usd {
        url.push_str(&format!("?filterUsd={}", min_usd));
    }
    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to call Pendle positions API")?;
    let body: Value = resp.json().await.context("Failed to parse positions response")?;
    Ok(body)
}

/// GET /v1/prices/assets — batch asset price query
pub async fn get_asset_prices(
    chain_id: Option<u64>,
    ids: Option<&str>,
    asset_type: Option<&str>,
    api_key: Option<&str>,
) -> anyhow::Result<Value> {
    let client = build_client(api_key)?;
    let mut params = Vec::new();
    if let Some(cid) = chain_id {
        params.push(format!("chainId={}", cid));
    }
    if let Some(i) = ids {
        params.push(format!("ids={}", i));
    }
    if let Some(t) = asset_type {
        params.push(format!("type={}", t));
    }
    let url = if params.is_empty() {
        format!("{}/v1/prices/assets", PENDLE_API_BASE)
    } else {
        format!("{}/v1/prices/assets?{}", PENDLE_API_BASE, params.join("&"))
    };
    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to call Pendle prices API")?;
    let body: Value = resp.json().await.context("Failed to parse prices response")?;
    Ok(body)
}

/// POST /v3/sdk/{chainId}/convert — generate transaction calldata via Pendle Hosted SDK
pub async fn sdk_convert(
    chain_id: u64,
    receiver: &str,
    inputs: Vec<SdkTokenAmount>,
    outputs: Vec<SdkTokenAmount>,
    slippage: f64,
    api_key: Option<&str>,
) -> anyhow::Result<Value> {
    let client = build_client(api_key)?;
    let url = format!("{}/v3/sdk/{}/convert", PENDLE_API_BASE, chain_id);

    // Pendle SDK /convert API:
    //   inputs: [{ "token": address, "amount": bigint_string }]
    //   outputs: [address_string, ...]  (plain addresses, no objects)
    //   enableAggregator: true  — allows arbitrary tokenIn/tokenOut (e.g. USDC for sell-pt)
    let body = serde_json::json!({
        "inputs": inputs.iter().map(|i| serde_json::json!({
            "token": i.token,
            "amount": i.amount
        })).collect::<Vec<_>>(),
        "outputs": outputs.iter().map(|o| o.token.as_str()).collect::<Vec<_>>(),
        "receiver": receiver,
        "slippage": slippage,
        "enableAggregator": true
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("Failed to call Pendle SDK convert API")?;
    let response: Value = resp.json().await.context("Failed to parse SDK convert response")?;
    Ok(response)
}

pub struct SdkTokenAmount {
    pub token: String,
    pub amount: String,
}

/// Extract calldata and router address from SDK convert response
pub fn extract_sdk_calldata(response: &Value) -> anyhow::Result<(String, String)> {
    let routes = response["routes"]
        .as_array()
        .context("No routes in SDK response")?;
    let route = routes.first().context("Empty routes array")?;
    let calldata = route["tx"]["data"]
        .as_str()
        .context("No tx.data in route")?
        .to_string();
    let to = route["tx"]["to"]
        .as_str()
        .unwrap_or(crate::config::PENDLE_ROUTER)
        .to_string();
    Ok((calldata, to))
}

/// Extract required approvals from SDK convert response
pub fn extract_required_approvals(response: &Value) -> Vec<(String, String)> {
    // Returns list of (token_address, spender_address) pairs
    let mut approvals = Vec::new();
    if let Some(arr) = response["requiredApprovals"].as_array() {
        for item in arr {
            let token = item["token"].as_str().unwrap_or("").to_string();
            let spender = item["spender"]
                .as_str()
                .unwrap_or(crate::config::PENDLE_ROUTER)
                .to_string();
            if !token.is_empty() {
                approvals.push((token, spender));
            }
        }
    }
    approvals
}
