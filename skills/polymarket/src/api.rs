/// Polymarket REST API client.
/// Covers CLOB API, Gamma API, and Data API.
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::l2_headers;
use crate::config::{Credentials, Urls};

// ─── Custom serde helpers ─────────────────────────────────────────────────────

fn de_f64_or_str<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match v {
        None => Ok(None),
        Some(serde_json::Value::Number(n)) => Ok(n.as_f64()),
        Some(serde_json::Value::String(s)) => s
            .parse()
            .ok()
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("invalid float")),
        Some(serde_json::Value::Null) => Ok(None),
        _ => Ok(None),
    }
}

fn de_str_or_num_as_str<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match v {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        Some(n) => Ok(Some(n.to_string())),
    }
}

// ─── Shared types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClobToken {
    pub token_id: String,
    pub outcome: String,
    pub price: f64,
    #[serde(default)]
    pub winner: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClobMarket {
    pub condition_id: String,
    #[serde(default)]
    pub question: Option<String>,
    pub tokens: Vec<ClobToken>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub closed: bool,
    #[serde(default)]
    pub accepting_orders: bool,
    #[serde(default)]
    pub neg_risk: bool,
    #[serde(default)]
    pub end_date_iso: Option<String>,
    #[serde(default)]
    pub min_incentive_size: Option<String>,
    #[serde(default)]
    pub max_incentive_spread: Option<String>,
    #[serde(default)]
    pub maker_base_fee: Option<u64>,
    #[serde(default)]
    pub taker_base_fee: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GammaMarket {
    #[serde(default, deserialize_with = "de_str_or_num_as_str")]
    pub id: Option<String>,
    #[serde(rename = "conditionId")]
    pub condition_id: Option<String>,
    pub slug: Option<String>,
    pub question: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub closed: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(rename = "acceptingOrders", default)]
    pub accepting_orders: bool,
    #[serde(rename = "clobTokenIds")]
    pub clob_token_ids: Option<String>,
    #[serde(rename = "outcomePrices")]
    pub outcome_prices: Option<String>,
    pub outcomes: Option<String>,
    #[serde(default, deserialize_with = "de_f64_or_str")]
    pub liquidity: Option<f64>,
    #[serde(default, deserialize_with = "de_f64_or_str")]
    pub volume: Option<f64>,
    #[serde(rename = "volume24hr", default, deserialize_with = "de_f64_or_str")]
    pub volume24hr: Option<f64>,
    #[serde(rename = "bestBid", default, deserialize_with = "de_f64_or_str")]
    pub best_bid: Option<f64>,
    #[serde(rename = "bestAsk", default, deserialize_with = "de_f64_or_str")]
    pub best_ask: Option<f64>,
    #[serde(rename = "lastTradePrice", default, deserialize_with = "de_f64_or_str")]
    pub last_trade_price: Option<f64>,
    #[serde(rename = "orderPriceMinTickSize", default, deserialize_with = "de_f64_or_str")]
    pub order_price_min_tick_size: Option<f64>,
    #[serde(rename = "orderMinSize", default, deserialize_with = "de_f64_or_str")]
    pub order_min_size: Option<f64>,
    #[serde(rename = "negRisk", default)]
    pub neg_risk: bool,
    pub fee: Option<String>,
}

impl GammaMarket {
    /// Parse clobTokenIds JSON string into a Vec<String>
    pub fn token_ids(&self) -> Vec<String> {
        self.clob_token_ids.as_ref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default()
    }

    /// Parse outcomePrices JSON string into a Vec<String>
    pub fn prices(&self) -> Vec<String> {
        self.outcome_prices.as_ref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default()
    }

    /// Parse outcomes JSON string into a Vec<String>
    pub fn outcome_list(&self) -> Vec<String> {
        self.outcomes.as_ref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_else(|| vec!["Yes".to_string(), "No".to_string()])
    }
}

#[derive(Debug, Deserialize)]
pub struct OrderBook {
    pub market: Option<String>,
    pub asset_id: Option<String>,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    #[serde(default)]
    pub min_order_size: Option<String>,
    #[serde(default)]
    pub tick_size: Option<String>,
    #[serde(default)]
    pub neg_risk: bool,
    #[serde(default)]
    pub last_trade_price: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PriceLevel {
    pub price: String,
    pub size: String,
}

#[derive(Debug, Deserialize)]
pub struct Position {
    #[serde(rename = "proxyWallet")]
    pub proxy_wallet: Option<String>,
    pub asset: Option<String>,
    #[serde(rename = "conditionId")]
    pub condition_id: Option<String>,
    pub size: Option<f64>,
    #[serde(rename = "avgPrice")]
    pub avg_price: Option<f64>,
    #[serde(rename = "currentValue")]
    pub current_value: Option<f64>,
    #[serde(rename = "cashPnl")]
    pub cash_pnl: Option<f64>,
    #[serde(rename = "percentPnl")]
    pub percent_pnl: Option<f64>,
    #[serde(rename = "realizedPnl")]
    pub realized_pnl: Option<f64>,
    #[serde(rename = "curPrice")]
    pub cur_price: Option<f64>,
    #[serde(default)]
    pub redeemable: bool,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub outcome: Option<String>,
    #[serde(rename = "outcomeIndex")]
    pub outcome_index: Option<u32>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    #[serde(rename = "negativeRisk", default)]
    pub negative_risk: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderRequest {
    pub order: OrderBody,
    pub owner: String,
    #[serde(rename = "orderType")]
    pub order_type: String,
    #[serde(rename = "postOnly", default)]
    pub post_only: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderBody {
    /// salt is serialized as a JSON number (not string) per clob-client spec
    pub salt: u64,
    pub maker: String,
    pub signer: String,
    pub taker: String,
    #[serde(rename = "tokenId")]
    pub token_id: String,
    #[serde(rename = "makerAmount")]
    pub maker_amount: String,
    #[serde(rename = "takerAmount")]
    pub taker_amount: String,
    pub expiration: String,
    pub nonce: String,
    #[serde(rename = "feeRateBps")]
    pub fee_rate_bps: String,
    pub side: String,
    #[serde(rename = "signatureType")]
    pub signature_type: u8,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct OrderResponse {
    pub success: Option<bool>,
    #[serde(rename = "orderID")]
    pub order_id: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "makingAmount")]
    pub making_amount: Option<String>,
    #[serde(rename = "takingAmount")]
    pub taking_amount: Option<String>,
    #[serde(rename = "errorMsg")]
    pub error_msg: Option<String>,
    #[serde(rename = "transactionsHashes", default)]
    pub tx_hashes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BalanceAllowance {
    pub asset_address: Option<String>,
    pub balance: Option<String>,
    /// singular allowance (older API format)
    pub allowance: Option<String>,
    /// plural allowances map (newer API format: {exchange_addr: amount})
    #[serde(default)]
    pub allowances: std::collections::HashMap<String, String>,
}

impl BalanceAllowance {
    /// Get the allowance for a specific exchange address, checking both formats.
    pub fn allowance_for(&self, exchange_addr: &str) -> u64 {
        // Check the plural allowances map first (newer format)
        let addr_lower = exchange_addr.to_lowercase();
        for (k, v) in &self.allowances {
            if k.to_lowercase() == addr_lower {
                return v.parse().unwrap_or(0);
            }
        }
        // Fall back to singular allowance field (older format)
        self.allowance.as_deref().unwrap_or("0").parse().unwrap_or(0)
    }
}

// ─── CLOB API calls ───────────────────────────────────────────────────────────

/// Check whether the CLOB trading endpoint is geo-restricted.
///
/// POSTs an empty request to /order (no auth headers). The CLOB applies
/// geo-checks before auth checks on this endpoint, so:
///   - Restricted IP  → HTTP 403 + JSON {"error":"Trading restricted in your region..."}
///   - Unrestricted IP → HTTP 400/401/422 (invalid/unauthorized — request reached the app)
///
/// We match on the specific error string rather than the status code alone to avoid
/// false positives (some endpoints return 403 for auth reasons on unrestricted IPs).
/// Fails open on network errors or unexpected responses.
pub async fn check_clob_access(client: &Client) -> Option<String> {
    let url = format!("{}/order", Urls::CLOB);
    let resp = match client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body("{}")
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return None,
    };

    let status = resp.status();

    // Only inspect 403/451 — anything else (400, 401, 422, 200, 5xx) is not a geo-block
    if status != reqwest::StatusCode::FORBIDDEN && status.as_u16() != 451 {
        return None;
    }

    // Read the body and look for Polymarket's specific geo-restriction message.
    // Matching the string rather than the status code avoids false positives.
    let body = match resp.text().await {
        Ok(b) => b,
        Err(_) => return None,
    };

    if body.contains("restricted") || body.contains("geoblock") {
        return Some(
            "Polymarket is not available in your region — trading is restricted. \
             Review Polymarket's Terms of Use (https://polymarket.com/tos) \
             before topping up USDC.e."
                .to_string(),
        );
    }

    // 403 for a different reason (e.g. auth policy change) — fail open
    None
}

pub async fn get_clob_market(client: &Client, condition_id: &str) -> Result<ClobMarket> {
    let url = format!("{}/markets/{}", Urls::CLOB, condition_id);
    let resp = client.get(&url).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("Market not found: {}", condition_id);
    }
    resp.json()
        .await
        .context("parsing CLOB market response")
}

pub async fn get_orderbook(client: &Client, token_id: &str) -> Result<OrderBook> {
    let url = format!("{}/book?token_id={}", Urls::CLOB, token_id);
    client.get(&url)
        .send()
        .await?
        .json()
        .await
        .context("parsing order book response")
}

/// Fetch the market's maker_base_fee (in basis points) from CLOB market data.
/// Returns 0 if not found.
pub async fn get_market_fee(client: &Client, condition_id: &str) -> Result<u64> {
    let url = format!("{}/markets/{}", Urls::CLOB, condition_id);
    let v: Value = client.get(&url).send().await?.json().await?;
    let fee = v["maker_base_fee"]
        .as_u64()
        .or_else(|| v["maker_base_fee"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0);
    Ok(fee)
}

pub async fn get_tick_size(client: &Client, token_id: &str) -> Result<f64> {
    let url = format!("{}/tick-size?token_id={}", Urls::CLOB, token_id);
    let v: Value = client.get(&url).send().await?.json().await?;
    // minimum_tick_size may be a JSON number or a JSON string
    let tick = v["minimum_tick_size"]
        .as_f64()
        .or_else(|| v["minimum_tick_size"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.01);
    Ok(tick)
}

pub async fn get_price(client: &Client, token_id: &str, side: &str) -> Result<String> {
    let url = format!("{}/price?token_id={}&side={}", Urls::CLOB, token_id, side);
    let v: Value = client.get(&url).send().await?.json().await?;
    Ok(v["price"].as_str().unwrap_or("0").to_string())
}

pub async fn get_server_time(client: &Client) -> Result<u64> {
    let url = format!("{}/time", Urls::CLOB);
    let v: Value = client.get(&url).send().await?.json().await?;
    Ok(v["time"].as_u64().unwrap_or(0))
}

pub async fn get_balance_allowance(
    client: &Client,
    address: &str,
    creds: &Credentials,
    asset_type: &str,
    token_id: Option<&str>,
) -> Result<BalanceAllowance> {
    let query = if let Some(tid) = token_id {
        format!("?asset_type={}&signature_type=0&token_id={}", asset_type, tid)
    } else {
        format!("?asset_type={}&signature_type=0", asset_type)
    };
    // Polymarket CLOB HMAC signing uses only the base path (without query params)
    let hmac_path = "/balance-allowance";
    let full_path = format!("{}{}", hmac_path, query);

    let headers = l2_headers(
        address,
        &creds.api_key,
        &creds.secret,
        &creds.passphrase,
        "GET",
        hmac_path,
        "",
    )?;

    let url = format!("{}{}", Urls::CLOB, full_path);
    let mut req = client.get(&url);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    req.send()
        .await?
        .json()
        .await
        .context("parsing balance-allowance response")
}

pub async fn post_order(
    client: &Client,
    address: &str,
    creds: &Credentials,
    order_req: &OrderRequest,
) -> Result<OrderResponse> {
    let body = serde_json::to_string(order_req)?;
    let path = "/order";

    let headers = l2_headers(
        address,
        &creds.api_key,
        &creds.secret,
        &creds.passphrase,
        "POST",
        path,
        &body,
    )?;

    let url = format!("{}{}", Urls::CLOB, path);
    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let raw = req.send().await?.text().await?;
    // If the response contains a top-level "error" field (API-level rejection), propagate it
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
        if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
            return Ok(OrderResponse {
                success: Some(false),
                order_id: None,
                status: None,
                making_amount: None,
                taking_amount: None,
                error_msg: Some(err.to_string()),
                tx_hashes: vec![],
            });
        }
    }
    serde_json::from_str(&raw).with_context(|| format!("parsing post-order response: {}", raw))
}

pub async fn cancel_order(
    client: &Client,
    address: &str,
    creds: &Credentials,
    order_id: &str,
) -> Result<Value> {
    let body_val = serde_json::json!({ "orderID": order_id });
    let body = serde_json::to_string(&body_val)?;
    let path = "/order";

    let headers = l2_headers(
        address,
        &creds.api_key,
        &creds.secret,
        &creds.passphrase,
        "DELETE",
        path,
        &body,
    )?;

    let url = format!("{}{}", Urls::CLOB, path);
    let mut req = client
        .delete(&url)
        .header("Content-Type", "application/json")
        .body(body);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    req.send()
        .await?
        .json()
        .await
        .context("parsing cancel-order response")
}

pub async fn cancel_all_orders(
    client: &Client,
    address: &str,
    creds: &Credentials,
) -> Result<Value> {
    let path = "/cancel-all";
    let headers = l2_headers(
        address,
        &creds.api_key,
        &creds.secret,
        &creds.passphrase,
        "DELETE",
        path,
        "",
    )?;

    let url = format!("{}{}", Urls::CLOB, path);
    let mut req = client.delete(&url);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    req.send()
        .await?
        .json()
        .await
        .context("parsing cancel-all response")
}

pub async fn cancel_market_orders(
    client: &Client,
    address: &str,
    creds: &Credentials,
    condition_id: &str,
    token_id: Option<&str>,
) -> Result<Value> {
    let mut body_map = serde_json::Map::new();
    body_map.insert("market".to_string(), Value::String(condition_id.to_string()));
    if let Some(tid) = token_id {
        body_map.insert("asset_id".to_string(), Value::String(tid.to_string()));
    }
    let body = serde_json::to_string(&Value::Object(body_map))?;
    let path = "/cancel-market-orders";

    let headers = l2_headers(
        address,
        &creds.api_key,
        &creds.secret,
        &creds.passphrase,
        "DELETE",
        path,
        &body,
    )?;

    let url = format!("{}{}", Urls::CLOB, path);
    let mut req = client
        .delete(&url)
        .header("Content-Type", "application/json")
        .body(body);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    req.send()
        .await?
        .json()
        .await
        .context("parsing cancel-market-orders response")
}

// ─── Gamma API calls ──────────────────────────────────────────────────────────

pub async fn list_gamma_markets(
    client: &Client,
    limit: u32,
    offset: u32,
    keyword: Option<&str>,
) -> Result<Vec<GammaMarket>> {
    // When keyword filtering is requested, fetch a larger page and filter client-side.
    // The Gamma API's ?q= parameter does not reliably filter results — testing confirms
    // it returns the same volume-sorted list regardless of the keyword value.
    let fetch_limit = if keyword.is_some() { (limit * 5).min(100) } else { limit };
    let url = format!(
        "{}/markets?active=true&closed=false&limit={}&offset={}&order=volume24hrClob&ascending=false",
        Urls::GAMMA, fetch_limit, offset
    );

    let all: Vec<GammaMarket> = client.get(&url)
        .send()
        .await?
        .json()
        .await
        .context("parsing Gamma markets list")?;

    if let Some(kw) = keyword {
        let kw_lower = kw.to_lowercase();
        Ok(all
            .into_iter()
            .filter(|m| {
                let q = m.question.as_deref().unwrap_or("").to_lowercase();
                let s = m.slug.as_deref().unwrap_or("").to_lowercase();
                q.contains(&kw_lower) || s.contains(&kw_lower)
            })
            .take(limit as usize)
            .collect())
    } else {
        Ok(all)
    }
}

pub async fn get_gamma_market_by_slug(client: &Client, slug: &str) -> Result<GammaMarket> {
    let url = format!("{}/markets/slug/{}", Urls::GAMMA, slug);
    let v: Value = client.get(&url).send().await?.json().await?;

    // Response can be an array or single object
    let market = if v.is_array() {
        v.as_array()
            .and_then(|a| a.first())
            .cloned()
            .unwrap_or(v.clone())
    } else {
        v
    };

    let parsed: GammaMarket =
        serde_json::from_value(market).context("parsing Gamma market by slug")?;

    if parsed.condition_id.as_deref().unwrap_or("").is_empty()
        && parsed.slug.as_deref().unwrap_or("").is_empty()
    {
        return Err(anyhow::anyhow!(
            "Market not found: no market with slug '{}'",
            slug
        ));
    }

    Ok(parsed)
}

// ─── Profile / proxy wallet ───────────────────────────────────────────────────

/// Fetch the Polymarket proxy wallet address for a given signer address.
/// Calls `GET /profile?user=<address>` on the CLOB API.
/// Returns None if the user has not completed polymarket.com onboarding.
pub async fn get_proxy_wallet(client: &Client, signer_addr: &str) -> Result<Option<String>> {
    let url = format!("{}/profile?user={}", Urls::CLOB, signer_addr);
    let v: Value = client.get(&url).send().await?.json().await
        .context("parsing profile response")?;
    let proxy = v["proxyWallet"]
        .as_str()
        .or_else(|| v["proxy_wallet"].as_str())
        .map(|s| s.to_string());
    Ok(proxy)
}

// ─── Data API calls ───────────────────────────────────────────────────────────

pub async fn get_positions(client: &Client, user_address: &str) -> Result<Vec<Position>> {
    let url = format!(
        "{}/positions?user={}&sizeThreshold=0.01&limit=100&offset=0",
        Urls::DATA, user_address
    );
    client.get(&url)
        .send()
        .await?
        .json()
        .await
        .context("parsing positions response")
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Compute the worst price for a BUY by walking the asks best-to-worst until cumulative USDC is covered.
/// The CLOB API returns asks in descending price order, so we iterate in reverse to start from the best ask.
pub fn compute_buy_worst_price(asks: &[PriceLevel], usdc_amount: f64) -> Option<f64> {
    let mut cumulative = 0.0f64;
    let mut worst = None;
    for ask in asks.iter().rev() {
        let price: f64 = ask.price.parse().ok()?;
        let size: f64 = ask.size.parse().ok()?;
        cumulative += price * size;
        worst = Some(price);
        if cumulative >= usdc_amount {
            break;
        }
    }
    worst
}

/// Compute the worst price for a SELL by walking the bids best-to-worst until cumulative shares covered.
/// The CLOB API returns bids in ascending price order, so we iterate in reverse to start from the best bid.
pub fn compute_sell_worst_price(bids: &[PriceLevel], share_amount: f64) -> Option<f64> {
    let mut cumulative = 0.0f64;
    let mut worst = None;
    for bid in bids.iter().rev() {
        let price: f64 = bid.price.parse().ok()?;
        let size: f64 = bid.size.parse().ok()?;
        cumulative += size;
        worst = Some(price);
        if cumulative >= share_amount {
            break;
        }
    }
    worst
}

/// Round price to tick size precision.
pub fn round_price(price: f64, tick_size: f64) -> f64 {
    let decimals = (-tick_size.log10()).ceil() as u32;
    let factor = 10f64.powi(decimals as i32);
    (price * factor).round() / factor
}

/// Round size DOWN to 2 decimal places (standard for Polymarket).
pub fn round_size_down(size: f64) -> f64 {
    (size * 100.0).floor() / 100.0
}

/// Round amount DOWN to tick-size-dependent decimal places.
pub fn round_amount_down(amount: f64, tick_size: f64) -> f64 {
    let decimals = (-tick_size.log10()).ceil() as u32;
    // amount decimals = price decimals + 2
    let amount_decimals = decimals + 2;
    let factor = 10f64.powi(amount_decimals as i32);
    (amount * factor).floor() / factor
}

/// Scale float to 6-decimal integer units (USDC or token shares).
pub fn to_token_units(amount: f64) -> u64 {
    (amount * 1_000_000.0).round() as u64
}
