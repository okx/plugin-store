use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// GMX REST API base URL based on chain
pub fn api_base(cfg: &crate::config::ChainConfig) -> &'static str {
    cfg.api_base
}

pub fn api_fallback(cfg: &crate::config::ChainConfig) -> &'static str {
    cfg.api_fallback
}

// ---- Market types ----

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Market {
    pub name: Option<String>,
    #[serde(rename = "marketToken")]
    pub market_token: Option<String>,
    #[serde(rename = "indexToken")]
    pub index_token: Option<String>,
    #[serde(rename = "longToken")]
    pub long_token: Option<String>,
    #[serde(rename = "shortToken")]
    pub short_token: Option<String>,
    #[serde(rename = "isListed")]
    pub is_listed: Option<bool>,
    #[serde(rename = "availableLiquidityLong", default, deserialize_with = "deser_number_or_string::deserialize")]
    pub available_liquidity_long: Option<String>,
    #[serde(rename = "availableLiquidityShort", default, deserialize_with = "deser_number_or_string::deserialize")]
    pub available_liquidity_short: Option<String>,
    #[serde(rename = "openInterestLong", default, deserialize_with = "deser_number_or_string::deserialize")]
    pub open_interest_long: Option<String>,
    #[serde(rename = "openInterestShort", default, deserialize_with = "deser_number_or_string::deserialize")]
    pub open_interest_short: Option<String>,
    #[serde(rename = "netRate1h", default, deserialize_with = "deser_number_or_string::deserialize")]
    pub net_rate_1h: Option<String>,
    #[serde(rename = "fundingRateLong", default, deserialize_with = "deser_number_or_string::deserialize")]
    pub funding_rate_long: Option<String>,
    #[serde(rename = "fundingRateShort", default, deserialize_with = "deser_number_or_string::deserialize")]
    pub funding_rate_short: Option<String>,
    #[serde(rename = "borrowingRateLong", default, deserialize_with = "deser_number_or_string::deserialize")]
    pub borrowing_rate_long: Option<String>,
    #[serde(rename = "borrowingRateShort", default, deserialize_with = "deser_number_or_string::deserialize")]
    pub borrowing_rate_short: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MarketsResponse {
    pub markets: Option<Vec<Market>>,
}

// ---- Price types ----

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PriceTicker {
    #[serde(rename = "tokenAddress")]
    pub token_address: Option<String>,
    #[serde(rename = "tokenSymbol")]
    pub token_symbol: Option<String>,
    #[serde(rename = "minPrice", default, deserialize_with = "deser_number_or_string::deserialize")]
    pub min_price: Option<String>,
    #[serde(rename = "maxPrice", default, deserialize_with = "deser_number_or_string::deserialize")]
    pub max_price: Option<String>,
    #[serde(rename = "updatedAt", default, deserialize_with = "deser_number_or_string::deserialize")]
    pub updated_at: Option<String>,
}

// ---- Token types ----

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenInfo {
    pub symbol: Option<String>,
    pub address: Option<String>,
    pub decimals: Option<u8>,
}

// ---- API fetch helpers ----

/// GET /markets/info — returns full market info with rates
pub async fn fetch_markets(cfg: &crate::config::ChainConfig) -> anyhow::Result<Vec<Market>> {
    let url = format!("{}/markets/info", cfg.api_base);
    let resp = fetch_with_fallback(&url, &format!("{}/markets/info", cfg.api_fallback)).await?;
    // Try outer "markets" key, or treat as array directly
    if let Some(markets) = resp.get("markets").and_then(|v| v.as_array()) {
        let filtered: Vec<Market> = markets
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .filter(|m: &Market| m.is_listed.unwrap_or(true))
            .collect();
        return Ok(filtered);
    }
    if let Some(arr) = resp.as_array() {
        let filtered: Vec<Market> = arr
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .filter(|m: &Market| m.is_listed.unwrap_or(true))
            .collect();
        return Ok(filtered);
    }
    Ok(vec![])
}

/// GET /prices/tickers — returns oracle prices
pub async fn fetch_prices(cfg: &crate::config::ChainConfig) -> anyhow::Result<Vec<PriceTicker>> {
    let url = format!("{}/prices/tickers", cfg.api_base);
    let fallback = format!("{}/prices/tickers", cfg.api_fallback);
    let resp = fetch_with_fallback(&url, &fallback).await?;
    if let Some(arr) = resp.as_array() {
        let tickers: Vec<PriceTicker> = arr
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        return Ok(tickers);
    }
    // Some responses wrap in a field
    if let Some(arr) = resp.get("data").and_then(|v| v.as_array()) {
        let tickers: Vec<PriceTicker> = arr
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        return Ok(tickers);
    }
    Ok(vec![])
}

/// Lookup a market by index token symbol or address.
///
/// Match priority (most specific first):
/// 1. Exact full name match, e.g. "ETH/USD [WETH-USDC]"
/// 2. Exact base-symbol match — name prefix before " [", e.g. "ETH/USD" matches "ETH/USD [WETH-USDC]"
///    If multiple markets share the same base symbol the first one in the list is returned and a
///    warning is printed so the caller can see ambiguity.
/// 3. Exact index-token address match (checksummed or lowercase)
///
/// `contains()` is intentionally NOT used — it caused non-deterministic market selection when
/// multiple markets share a common substring (e.g. "SOL/USD [SOL-USDC]" vs "SOL/USD [SOL-SOL]").
pub fn find_market_by_symbol<'a>(markets: &'a [Market], query: &str) -> Option<&'a Market> {
    let query_lower = query.to_lowercase();

    // 1. Exact full name match
    if let Some(m) = markets.iter().find(|m| {
        m.name.as_deref().map(|n| n.to_lowercase() == query_lower).unwrap_or(false)
    }) {
        return Some(m);
    }

    // 2. Exact base-symbol match (part before " [")
    let base_matches: Vec<&Market> = markets.iter().filter(|m| {
        if let Some(name) = &m.name {
            let base = name.split(" [").next().unwrap_or(name);
            base.to_lowercase() == query_lower
        } else {
            false
        }
    }).collect();
    if base_matches.len() > 1 {
        eprintln!(
            "WARNING: ambiguous market '{}' — {} matches found. Using '{}'. \
             Pass the full name (e.g. \"{}\") to select a specific market.",
            query,
            base_matches.len(),
            base_matches[0].name.as_deref().unwrap_or("?"),
            base_matches[0].name.as_deref().unwrap_or("?"),
        );
    }
    if let Some(m) = base_matches.into_iter().next() {
        return Some(m);
    }

    // 3. Exact index-token address match
    markets.iter().find(|m| {
        m.index_token.as_deref()
            .map(|addr| addr.to_lowercase() == query_lower)
            .unwrap_or(false)
    })
}

/// GET /tokens — returns token list with decimals
pub async fn fetch_tokens(cfg: &crate::config::ChainConfig) -> anyhow::Result<Vec<TokenInfo>> {
    let url = format!("{}/tokens", cfg.api_base);
    let fallback = format!("{}/tokens", cfg.api_fallback);
    let resp = fetch_with_fallback(&url, &fallback).await?;
    if let Some(arr) = resp.as_array() {
        let tokens: Vec<TokenInfo> = arr
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        return Ok(tokens);
    }
    if let Some(arr) = resp.get("tokens").and_then(|v| v.as_array()) {
        let tokens: Vec<TokenInfo> = arr
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        return Ok(tokens);
    }
    Ok(vec![])
}

/// Convert raw GMX price to USD given token decimals.
/// GMX stores prices as: price_usd * 10^(30 - token_decimals)
/// So: price_usd = raw / 10^(30 - token_decimals)
pub fn raw_price_to_usd(raw: u128, token_decimals: u8) -> f64 {
    let precision_exp = 30u32.saturating_sub(token_decimals as u32);
    let divisor = 10f64.powi(precision_exp as i32);
    raw as f64 / divisor
}

/// Lookup price by token address (case-insensitive)
pub fn find_price<'a>(tickers: &'a [PriceTicker], token_addr: &str) -> Option<&'a PriceTicker> {
    let addr_lower = token_addr.to_lowercase();
    tickers.iter().find(|t| {
        t.token_address
            .as_deref()
            .map(|a| a.to_lowercase() == addr_lower)
            .unwrap_or(false)
    })
}

async fn fetch_with_fallback(url: &str, fallback: &str) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => {
            Ok(resp.json().await.context("Failed to parse API response")?)
        }
        _ => {
            // Try fallback
            let resp = client
                .get(fallback)
                .send()
                .await
                .context("Both primary and fallback API requests failed")?;
            Ok(resp.json().await.context("Failed to parse fallback API response")?)
        }
    }
}

// ---- Defensive deserialization helper ----
// Some GMX API fields return numbers as JSON numbers OR strings

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
