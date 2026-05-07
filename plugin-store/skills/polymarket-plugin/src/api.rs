/// Polymarket REST API client.
/// Covers CLOB API, Gamma API, and Data API.
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::{l2_headers, builder_l2_headers, BuilderCredentials};
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
    #[serde(rename = "initialValue")]
    pub initial_value: Option<f64>,
    #[serde(rename = "currentValue")]
    pub current_value: Option<f64>,
    #[serde(rename = "cashPnl")]
    pub cash_pnl: Option<f64>,
    #[serde(rename = "percentPnl")]
    pub percent_pnl: Option<f64>,
    #[serde(rename = "totalBought")]
    pub total_bought: Option<f64>,
    #[serde(rename = "realizedPnl")]
    pub realized_pnl: Option<f64>,
    #[serde(rename = "percentRealizedPnl")]
    pub percent_realized_pnl: Option<f64>,
    #[serde(rename = "curPrice")]
    pub cur_price: Option<f64>,
    #[serde(default)]
    pub redeemable: bool,
    #[serde(default)]
    pub mergeable: bool,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub icon: Option<String>,
    #[serde(rename = "eventId")]
    pub event_id: Option<String>,
    #[serde(rename = "eventSlug")]
    pub event_slug: Option<String>,
    pub outcome: Option<String>,
    #[serde(rename = "outcomeIndex")]
    pub outcome_index: Option<u32>,
    #[serde(rename = "oppositeOutcome")]
    pub opposite_outcome: Option<String>,
    #[serde(rename = "oppositeAsset")]
    pub opposite_asset: Option<String>,
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

/// V2 order body — new field layout, no taker/nonce/feeRateBps; adds timestamp/metadata/builder.
/// `expiration` is in the outer `OrderRequestV2` wrapper (not in the signed struct).
#[derive(Debug, Serialize, Deserialize)]
pub struct OrderBodyV2 {
    pub salt: u64,
    pub maker: String,
    pub signer: String,
    #[serde(rename = "tokenId")]
    pub token_id: String,
    #[serde(rename = "makerAmount")]
    pub maker_amount: String,
    #[serde(rename = "takerAmount")]
    pub taker_amount: String,
    pub side: String,
    #[serde(rename = "signatureType")]
    pub signature_type: u8,
    /// Millisecond Unix timestamp — part of the EIP-712 signed struct.
    pub timestamp: String,
    /// bytes32 optional metadata ("0x000...000" for standard orders).
    pub metadata: String,
    /// bytes32 builder code ("0x000...000" for non-builders).
    pub builder: String,
    pub signature: String,
}

/// V2 outer order request — wraps `OrderBodyV2` and moves `expiration` out of the signed struct.
#[derive(Debug, Serialize, Deserialize)]
pub struct OrderRequestV2 {
    pub order: OrderBodyV2,
    pub owner: String,
    #[serde(rename = "orderType")]
    pub order_type: String,
    #[serde(rename = "postOnly", default)]
    pub post_only: bool,
    /// GTD expiration timestamp (seconds). Present only for GTD orders; empty string otherwise.
    #[serde(rename = "expiration", skip_serializing_if = "String::is_empty")]
    pub expiration: String,
}

/// Open order returned by GET /orders.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenOrder {
    #[serde(rename = "id")]
    pub order_id: String,
    pub status: Option<String>,
    #[serde(rename = "market")]
    pub condition_id: Option<String>,
    #[serde(rename = "asset_id")]
    pub token_id: Option<String>,
    pub side: Option<String>,
    #[serde(rename = "original_size")]
    pub original_size: Option<String>,
    #[serde(rename = "size_matched")]
    pub size_matched: Option<String>,
    pub price: Option<String>,
    #[serde(rename = "created_at")]
    pub created_at: Option<u64>,
    // V1-only fields — presence signals V1 order
    pub nonce: Option<serde_json::Value>,
    #[serde(rename = "feeRateBps")]
    pub fee_rate_bps: Option<serde_json::Value>,
    // V2-only fields — presence signals V2 order
    pub timestamp: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

impl OpenOrder {
    /// Detect order version from field presence.
    /// V1 orders have `nonce`/`feeRateBps`; V2 orders have `timestamp`/`metadata`.
    pub fn version(&self) -> crate::config::OrderVersion {
        if self.nonce.is_some() || self.fee_rate_bps.is_some() {
            crate::config::OrderVersion::V1
        } else {
            crate::config::OrderVersion::V2
        }
    }

    pub fn is_v1(&self) -> bool {
        self.version() == crate::config::OrderVersion::V1
    }
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
    let url = format!("{}/order", Urls::clob());
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
    let url = format!("{}/markets/{}", Urls::clob(), condition_id);
    let resp = client.get(&url).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("Market not found: {}", condition_id);
    }
    resp.json()
        .await
        .context("parsing CLOB market response")
}

pub async fn get_orderbook(client: &Client, token_id: &str) -> Result<OrderBook> {
    let url = format!("{}/book?token_id={}", Urls::clob(), token_id);
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
    let url = format!("{}/markets/{}", Urls::clob(), condition_id);
    let v: Value = client.get(&url).send().await?.json().await?;
    let fee = v["maker_base_fee"]
        .as_u64()
        .or_else(|| v["maker_base_fee"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0);
    Ok(fee)
}

pub async fn get_tick_size(client: &Client, token_id: &str) -> Result<f64> {
    let url = format!("{}/tick-size?token_id={}", Urls::clob(), token_id);
    let v: Value = client.get(&url).send().await?.json().await?;
    // minimum_tick_size may be a JSON number or a JSON string
    let tick = v["minimum_tick_size"]
        .as_f64()
        .or_else(|| v["minimum_tick_size"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.01);
    Ok(tick)
}

pub async fn get_price(client: &Client, token_id: &str, side: &str) -> Result<String> {
    let url = format!("{}/price?token_id={}&side={}", Urls::clob(), token_id, side);
    let v: Value = client.get(&url).send().await?.json().await?;
    Ok(v["price"].as_str().unwrap_or("0").to_string())
}

pub async fn get_server_time(client: &Client) -> Result<u64> {
    let url = format!("{}/time", Urls::clob());
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

    let url = format!("{}{}", Urls::clob(), full_path);
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

pub async fn post_order<T: serde::Serialize>(
    client: &Client,
    address: &str,
    creds: &Credentials,
    order_req: &T,
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

    let url = format!("{}{}", Urls::clob(), path);
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

/// Query the CLOB server for the active order version (1 or 2).
///
/// Detect the active CLOB version (V1 or V2) by querying GET /version.
///
/// Returns `Err` on network/parse failure rather than silently defaulting to V1.
/// Reason: during the V1→V2 cutover (~2026-04-28 11:00 UTC) a transient probe
/// failure could route a V2-era order through the V1 path, which the upgraded
/// server will reject with a confusing 404/405. Bailing here gives the user a
/// clear retry message instead.
///
/// Pre-v2 servers (before 2026-04-21) returned 404 on `/version`; that path is
/// no longer reachable, so a missing endpoint now legitimately indicates a
/// problem worth surfacing.
pub async fn get_clob_version(client: &Client) -> Result<u8> {
    let url = format!("{}/version", Urls::CLOB);
    let resp = client
        .get(&url)
        .send()
        .await
        .with_context(|| "failed to detect CLOB version (network error). \
            Retry; if persistent, the server may be mid-cutover.")?;
    let v: serde_json::Value = resp
        .json()
        .await
        .with_context(|| "failed to parse /version response from CLOB. \
            Retry; if persistent, the server may be mid-cutover.")?;
    Ok(v["version"].as_u64().unwrap_or(1) as u8)
}

/// Fetch open orders for the authenticated user.
///
/// `state` is one of "OPEN", "MATCHED", "DELAYED", "UNMATCHED" (default: "OPEN").
/// Returns typed `OpenOrder` values with `version()` for V1/V2 detection.
pub async fn get_open_orders(
    client: &Client,
    address: &str,
    creds: &Credentials,
    state: &str,
) -> Result<Vec<OpenOrder>> {
    // CLOB v2 moved the orders listing endpoint from GET /orders?state=X to GET /data/orders.
    // The HMAC signature must be computed over the BASE PATH only (without query string).
    // The endpoint uses cursor-based pagination with a "next_cursor" field.
    let sign_path = "/data/orders";
    let request_path = format!("/data/orders?status={}", state);

    let headers = l2_headers(
        address,
        &creds.api_key,
        &creds.secret,
        &creds.passphrase,
        "GET",
        sign_path, // sign base path only
        "",
    )?;

    let url = format!("{}{}", Urls::CLOB, request_path);
    let mut req = client.get(&url);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let raw = req.send().await?.text().await?;
    if raw.trim().is_empty() {
        return Ok(vec![]);
    }
    let parsed: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing open-orders response: {}", raw))?;
    // v2 returns {"data": [...], "next_cursor": "...", "limit": 500, "count": N}
    let arr = if let Some(a) = parsed.get("data").and_then(|d| d.as_array()) {
        a.clone()
    } else if let Some(a) = parsed.as_array() {
        a.clone()
    } else {
        vec![]
    };
    let orders: Vec<OpenOrder> = arr
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();
    Ok(orders)
}

/// Fetch V1-era orders from the pre-migration endpoint introduced in CLOB v2.
///
/// During the migration window, orders placed on the V1 exchange may not appear in
/// `GET /orders` — this endpoint is the authoritative source for those legacy records.
/// Returns the raw JSON array; callers merge with live-orders results.
pub async fn get_pre_migration_orders(
    client: &Client,
    address: &str,
    creds: &Credentials,
) -> Result<Vec<OpenOrder>> {
    // Also uses base-path-only signing (same convention as /data/orders in v2).
    let path = "/data/pre-migration-orders";
    let headers = l2_headers(
        address,
        &creds.api_key,
        &creds.secret,
        &creds.passphrase,
        "GET",
        path,
        "",
    )?;

    let url = format!("{}{}", Urls::CLOB, path);
    let mut req = client.get(&url);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let raw = req.send().await?.text().await?;
    if raw.trim().is_empty() {
        return Ok(vec![]);
    }
    let parsed: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing pre-migration-orders response: {}", raw))?;
    let arr = if let Some(a) = parsed.as_array() {
        a.clone()
    } else if let Some(a) = parsed.get("data").and_then(|d| d.as_array()) {
        a.clone()
    } else {
        vec![]
    };
    let orders: Vec<OpenOrder> = arr
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();
    Ok(orders)
}

/// A single trade event returned by `GET /markets/live-activity/{condition_id}`.
#[derive(Debug, Clone, Deserialize)]
pub struct LiveTradeEvent {
    #[serde(rename = "tradeId", default)]
    pub trade_id: Option<String>,
    pub price: Option<String>,
    pub size: Option<String>,
    pub side: Option<String>,
    pub outcome: Option<String>,
    #[serde(rename = "timestamp")]
    pub timestamp: Option<u64>,
    #[serde(rename = "transactionHash", default)]
    pub tx_hash: Option<String>,
}

/// Fetch recent trade events for a market (public, no auth required).
/// Returns events sorted newest-first. Used by the `watch` command.
pub async fn get_market_live_activity(
    client: &Client,
    condition_id: &str,
    limit: u32,
) -> Result<Vec<LiveTradeEvent>> {
    let url = format!(
        "{}/markets/live-activity/{}?limit={}",
        Urls::CLOB, condition_id, limit
    );
    let raw = client.get(&url).send().await?.text().await?;
    let parsed: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing live-activity response: {}", raw))?;
    let arr = if let Some(a) = parsed.as_array() {
        a.clone()
    } else if let Some(a) = parsed.get("data").and_then(|d| d.as_array()) {
        a.clone()
    } else {
        vec![]
    };
    Ok(arr.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect())
}

/// RFQ quote returned by `GET /rfq/quote/{quote_id}`.
#[derive(Debug, Clone, Deserialize)]
pub struct RfqQuote {
    #[serde(rename = "quoteId")]
    pub quote_id: String,
    pub price: Option<String>,
    /// USDC amount the quote covers.
    pub amount: Option<String>,
    /// Unix timestamp (seconds) when this quote expires.
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<u64>,
    /// Market maker address.
    pub maker: Option<String>,
    pub status: Option<String>,
}

/// Request a RFQ quote for a block trade (no auth required for the request itself).
/// Returns a quote_id to poll with `get_rfq_quote`.
pub async fn post_rfq_request(
    client: &Client,
    condition_id: &str,
    token_id: &str,
    side: &str,
    amount_usdc: f64,
) -> Result<String> {
    let body = serde_json::to_string(&serde_json::json!({
        "market": condition_id,
        "asset_id": token_id,
        "side": side.to_uppercase(),
        "amount": format!("{:.6}", amount_usdc),
    }))?;
    let url = format!("{}/rfq/request", Urls::CLOB);
    let raw = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?
        .text()
        .await?;
    let v: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing rfq-request response: {}", raw))?;
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        anyhow::bail!("RFQ request failed: {}", err);
    }
    v.get("quoteId")
        .or_else(|| v.get("quote_id"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No quoteId in RFQ response: {}", raw))
}

/// Poll for an RFQ quote by quote_id (no auth required).
pub async fn get_rfq_quote(client: &Client, quote_id: &str) -> Result<RfqQuote> {
    let url = format!("{}/rfq/quote/{}", Urls::CLOB, quote_id);
    let raw = client.get(&url).send().await?.text().await?;
    serde_json::from_str(&raw)
        .with_context(|| format!("parsing rfq-quote response: {}", raw))
}

/// Confirm an RFQ quote — executes the block trade.
/// The signature is an EIP-712 signed order matching the quoted price/amount.
pub async fn post_rfq_confirm(
    client: &Client,
    address: &str,
    creds: &Credentials,
    quote_id: &str,
    order_body: &OrderBodyV2,
) -> Result<serde_json::Value> {
    let body = serde_json::to_string(&serde_json::json!({
        "quoteId": quote_id,
        "order": order_body,
        "owner": address,
    }))?;
    let path = "/rfq/confirm";
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
    req.send().await?.json().await.context("parsing rfq-confirm response")
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

    let url = format!("{}{}", Urls::clob(), path);
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

    let url = format!("{}{}", Urls::clob(), path);
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

    let url = format!("{}{}", Urls::clob(), path);
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
        Urls::gamma(), fetch_limit, offset
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

/// Fetch events from Gamma sorted by 24h volume, with optional client-side filtering.
///
/// `exclude_5m`  — remove 5-minute rolling Up/Down markets
/// `tag_filter`  — if Some, keep only events whose tags include at least one matching label
async fn fetch_gamma_events(
    client: &Client,
    fetch_limit: u32,
    exclude_5m: bool,
    tag_filter: Option<&[&str]>,
) -> Result<Vec<serde_json::Value>> {
    let url = format!(
        "{}/events?active=true&closed=false&limit={}&order=volume24hr&ascending=false",
        Urls::gamma(), fetch_limit
    );

    let all: Vec<serde_json::Value> = client
        .get(&url)
        .header("User-Agent", "polymarket-cli/1.0")
        .send()
        .await?
        .json()
        .await
        .context("parsing Gamma events")?;

    Ok(all
        .into_iter()
        .filter(|e| {
            if exclude_5m {
                let slug = e["slug"].as_str().unwrap_or("");
                let title = e["title"].as_str().unwrap_or("").to_lowercase();
                if slug.contains("updown-5m") || title.contains("up or down") {
                    return false;
                }
            }
            if let Some(tags) = tag_filter {
                let event_tags: Vec<String> = e["tags"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|t| t["label"].as_str().map(|s| s.to_lowercase()))
                    .collect();
                return tags.iter().any(|t| event_tags.contains(&t.to_lowercase()));
            }
            true
        })
        .collect())
}

/// Fetch "breaking" events: highest 24h volume non-5M events.
pub async fn list_breaking_events(client: &Client, limit: u32) -> Result<Vec<serde_json::Value>> {
    let all = fetch_gamma_events(client, (limit + 10).min(100), true, None).await?;
    Ok(all.into_iter().take(limit as usize).collect())
}

/// Fetch events for a named category: "sports", "elections", or "crypto".
/// Returns top events by 24h volume that match the category's tag set.
pub async fn list_category_events(
    client: &Client,
    category: &str,
    limit: u32,
) -> Result<Vec<serde_json::Value>> {
    let tags: &[&str] = match category {
        "sports" => &[
            "sports", "soccer", "tennis", "esports", "football", "basketball",
            "baseball", "golf", "nfl", "nba", "fifa world cup", "epl",
            "counter strike 2", "dota 2", "cricket", "hockey", "rugby",
        ],
        "elections" => &["elections", "global elections", "world elections"],
        "crypto" => &["crypto", "crypto prices", "bitcoin", "ethereum", "hit price"],
        _ => return Ok(vec![]),
    };

    // Fetch enough to fill the requested limit after tag filtering
    let fetch_limit = (limit * 5).min(500);
    let all = fetch_gamma_events(client, fetch_limit, true, Some(tags)).await?;
    Ok(all.into_iter().take(limit as usize).collect())
}

pub async fn get_gamma_market_by_slug(client: &Client, slug: &str) -> Result<GammaMarket> {
    let url = format!("{}/markets/slug/{}", Urls::gamma(), slug);
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
    let url = format!("{}/profile?user={}", Urls::clob(), signer_addr);
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
        Urls::data(), user_address
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

// ─── Token price (DeFiLlama) ─────────────────────────────────────────────────

/// Fetch USD spot price for a token using DeFiLlama coins API.
///
/// `chain_id` is the bridge chainId string (e.g. "1", "42161", "8453").
/// `token_address` is the ERC-20 contract address, or the ETH sentinel
///   `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` for native ether.
///
/// Returns `None` if the price could not be fetched (network error, unknown token).
pub async fn get_token_price_usd(
    client: &Client,
    chain_id: &str,
    token_address: &str,
) -> Option<f64> {
    // Map bridge chainId → DeFiLlama chain slug
    let chain_slug = match chain_id {
        "1"     => "ethereum",
        "42161" => "arbitrum",
        "8453"  => "base",
        "10"    => "optimism",
        "56"    => "bsc",
        "137"   => "polygon",
        "143"   => "monad",
        _       => return None,
    };

    // ETH sentinel → use the coingecko:ethereum key (no contract on-chain for native)
    let coin_key = if token_address.to_lowercase() == "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" {
        "coingecko:ethereum".to_string()
    } else {
        format!("{}:{}", chain_slug, token_address.to_lowercase())
    };

    let url = format!("https://coins.llama.fi/prices/current/{}", coin_key);
    let resp: serde_json::Value = client.get(&url).send().await.ok()?.json().await.ok()?;
    resp["coins"][&coin_key]["price"].as_f64()
}

// ─── Bridge API ───────────────────────────────────────────────────────────────

/// A single supported asset entry from GET /supported-assets.
#[derive(Debug, Clone, Deserialize)]
pub struct BridgeAsset {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    #[serde(rename = "chainName")]
    pub chain_name: String,
    pub token: BridgeToken,
    #[serde(rename = "minCheckoutUsd")]
    pub min_checkout_usd: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BridgeToken {
    pub name: String,
    pub symbol: String,
    pub address: String,
    pub decimals: u8,
}

/// Fetch the list of all supported deposit assets from the bridge API.
pub async fn bridge_supported_assets(client: &Client) -> Result<Vec<BridgeAsset>> {
    #[derive(Deserialize)]
    struct Resp {
        #[serde(rename = "supportedAssets")]
        supported_assets: Vec<BridgeAsset>,
    }
    let resp: Resp = client
        .get(format!("{}/supported-assets", Urls::BRIDGE))
        .send()
        .await?
        .json()
        .await
        .context("parsing bridge /supported-assets")?;
    Ok(resp.supported_assets)
}

/// Call POST /deposit with the proxy wallet address.
/// Returns the EVM deposit address assigned to this wallet.
pub async fn bridge_get_deposit_address(client: &Client, proxy_wallet: &str) -> Result<String> {
    let body = serde_json::json!({ "address": proxy_wallet });
    let resp: serde_json::Value = client
        .post(format!("{}/deposit", Urls::BRIDGE))
        .json(&body)
        .send()
        .await?
        .json()
        .await
        .context("calling bridge /deposit")?;

    resp["address"]["evm"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("bridge /deposit: no evm address in response: {}", resp))
}

/// Bridge deposit status values returned by GET /status/{address}.
#[derive(Debug, PartialEq)]
pub enum BridgeStatus {
    Completed,
    Failed,
    Pending(String), // intermediate state name
}

/// Poll GET /status/{evm_deposit_address} once and return the current status.
pub async fn bridge_poll_status(client: &Client, evm_address: &str) -> Result<BridgeStatus> {
    let url = format!("{}/status/{}", Urls::BRIDGE, evm_address);
    let resp: serde_json::Value = client
        .get(&url)
        .send()
        .await?
        .json()
        .await
        .context("calling bridge /status")?;

    // Response format: {"transactions": [{..., "status": "COMPLETED"}, ...]}
    // When no deposit has arrived yet: {"error": "cannot get transaction status"}
    // In that case we treat it as still pending.
    let status = resp["transactions"]
        .as_array()
        .and_then(|arr| arr.last())
        .and_then(|tx| tx["status"].as_str())
        .unwrap_or("PENDING")
        .to_string();

    Ok(match status.as_str() {
        "COMPLETED" => BridgeStatus::Completed,
        "FAILED" => BridgeStatus::Failed,
        s => BridgeStatus::Pending(s.to_string()),
    })
}

// ─── 5-minute markets (Gamma API) ────────────────────────────────────────────

/// A single 5-minute crypto Up/Down market from Gamma API.
#[derive(Debug, Clone)]
pub struct FiveMinMarket {
    pub slug: String,
    pub condition_id: String,
    pub question: String,
    pub up_price: f64,
    pub down_price: f64,
    pub end_date: String,    // ISO-8601 UTC
    pub up_token_id: String,
    pub down_token_id: String,
    pub accepting_orders: bool,
}

/// Fetch a single 5-minute market by its slug from the Gamma API.
/// Returns `None` if the market does not exist yet.
pub async fn get_5m_market(client: &Client, slug: &str) -> Result<Option<FiveMinMarket>> {
    let url = format!("{}/markets?slug={}", Urls::gamma(), slug);
    let resp: serde_json::Value = client
        .get(&url)
        .header("User-Agent", "polymarket-cli/1.0")
        .send()
        .await
        .context("gamma /markets request")?
        .json()
        .await
        .context("parsing gamma /markets response")?;

    let arr = match resp.as_array() {
        Some(a) if !a.is_empty() => a,
        _ => return Ok(None),
    };
    let m = &arr[0];

    let prices: Vec<f64> = m["outcomePrices"]
        .as_str()
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .map(|v| v.iter().filter_map(|x| x.parse().ok()).collect())
        .unwrap_or_default();

    let token_ids: Vec<String> = m["clobTokenIds"]
        .as_str()
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default();

    Ok(Some(FiveMinMarket {
        slug: slug.to_string(),
        condition_id: m["conditionId"].as_str().unwrap_or("").to_string(),
        question: m["question"].as_str().unwrap_or("").to_string(),
        up_price: prices.first().copied().unwrap_or(0.0),
        down_price: prices.get(1).copied().unwrap_or(0.0),
        end_date: m["endDate"].as_str().unwrap_or("").to_string(),
        up_token_id: token_ids.first().cloned().unwrap_or_default(),
        down_token_id: token_ids.get(1).cloned().unwrap_or_default(),
        accepting_orders: m["acceptingOrders"].as_bool().unwrap_or(false),
    }))
}

// ─── Deposit Wallet — Builder Auth ───────────────────────────────────────────

/// Derive per-user builder credentials via `POST /auth/builder-api-key` on the CLOB.
///
/// The builder API key is distinct from the CLOB API key — it specifically authorises
/// the Polymarket relayer to deploy and manage deposit wallets on behalf of the EOA.
/// No Polymarket web app interaction or Builders Program membership is required;
/// any authenticated CLOB user can derive their builder key from their CLOB credentials.
pub async fn get_builder_api_key(
    client: &Client,
    creds: &Credentials,
    owner_addr: &str,
) -> Result<BuilderCredentials> {
    let path = "/auth/builder-api-key";
    let headers = l2_headers(
        owner_addr,
        &creds.api_key,
        &creds.secret,
        &creds.passphrase,
        "POST",
        path,
        "",
    )?;
    let url = format!("{}{}", Urls::clob(), path);
    let mut req = client.post(&url);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp: serde_json::Value = req
        .send()
        .await
        .context("POST /auth/builder-api-key failed")?
        .json()
        .await
        .context("parsing /auth/builder-api-key response")?;
    if let Some(err) = resp.get("error").and_then(|e| e.as_str()) {
        anyhow::bail!("/auth/builder-api-key error: {}\nResponse: {}", err, resp);
    }
    serde_json::from_value(resp.clone())
        .with_context(|| format!("parsing builder-api-key response: {}", resp))
}

// ─── Deposit Wallet — Relayer API ────────────────────────────────────────────

/// Response from the relayer /submit endpoint.
#[derive(Debug, serde::Deserialize)]
pub struct RelayerSubmitResponse {
    #[serde(rename = "transactionHash")]
    pub transaction_hash: Option<String>,
    #[serde(rename = "transactionID")]
    pub transaction_id: Option<String>,
    #[serde(rename = "walletAddress")]
    pub wallet_address: Option<String>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub error: Option<String>,
    pub message: Option<String>,
}

/// Response from GET /nonce for a deposit wallet.
/// Note: the relayer encodes nonce as a JSON string (e.g. `"0"`), not a number.
#[derive(Debug, serde::Deserialize)]
pub struct WalletNonceResponse {
    #[serde(deserialize_with = "deserialize_string_as_u64")]
    pub nonce: u64,
}

fn deserialize_string_as_u64<'de, D>(de: D) -> std::result::Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum StrOrNum { Str(String), Num(u64) }
    match StrOrNum::deserialize(de)? {
        StrOrNum::Str(s) => s.parse::<u64>().map_err(serde::de::Error::custom),
        StrOrNum::Num(n) => Ok(n),
    }
}

/// Result of a WALLET-CREATE relayer call.
///
/// The relayer behaves differently depending on whether the wallet already exists:
/// - Fresh deployment: returns a `transactionHash`; the wallet address is read from
///   the factory event log after the tx confirms.
/// - Already deployed: returns `walletAddress` directly with no transaction hash.
/// - Failed: the relayer submitted a tx that failed (likely factory revert — wallet exists).
pub enum WalletCreateResult {
    /// Freshly submitted on-chain — poll this tx hash for confirmation, then
    /// extract the wallet address from the factory's `WalletCreated` event.
    Transaction(String),
    /// Wallet was already deployed for this owner — relayer returned the address directly.
    AlreadyDeployed(String),
    /// Relayer returned STATE_FAILED — the on-chain tx likely reverted because the
    /// wallet already exists in the factory's owner mapping.
    Failed,
}

/// Deploy a new deposit wallet via the Polymarket relayer (WALLET-CREATE).
///
/// No user ECDSA signature is required — the relayer deploys a deterministic ERC-1967 proxy.
/// Builder credentials (`POLY_BUILDER_*` headers) authorise the relayer to call the factory's
/// OnlyOperator `deploy()` function on behalf of the EOA.
///
/// Returns a `WalletCreateResult` indicating whether a fresh tx was submitted or the
/// wallet already existed (in which case the address is returned directly).
pub async fn relayer_wallet_create(
    client: &Client,
    owner_addr: &str,
    builder: &BuilderCredentials,
) -> Result<WalletCreateResult> {
    use crate::config::Contracts;
    let url = format!("{}/submit", crate::config::Urls::RELAYER);
    let body_json = serde_json::json!({
        "type": "WALLET-CREATE",
        "from": owner_addr,
        "to":   Contracts::DEPOSIT_WALLET_FACTORY,
    });
    let body_str = serde_json::to_string(&body_json)
        .context("serializing WALLET-CREATE body")?;
    let headers = builder_l2_headers(
        &builder.api_key,
        &builder.secret,
        &builder.passphrase,
        "POST",
        "/submit",
        &body_str,
    )?;
    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body_str);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let raw_resp = req
        .send()
        .await
        .context("relayer WALLET-CREATE request failed")?
        .text()
        .await
        .context("relayer WALLET-CREATE response read failed")?;
    let resp: RelayerSubmitResponse = serde_json::from_str(&raw_resp)
        .with_context(|| format!("parsing relayer WALLET-CREATE response: {}", raw_resp))?;

    // Check for hard errors (check both `error` and `message` fields)
    let err_msg = resp.error.as_deref().filter(|e| !e.is_empty())
        .or_else(|| resp.message.as_deref().filter(|m| !m.is_empty()));
    if let Some(err) = err_msg {
        anyhow::bail!("relayer WALLET-CREATE error: {}", err);
    }
    // STATE_FAILED: the on-chain tx was submitted but reverted (most likely the wallet
    // already exists in the factory's owner mapping from a prior deployment).
    let state = resp.state.as_deref().unwrap_or("");
    if state == "STATE_FAILED" || state == "FAILED" {
        return Ok(WalletCreateResult::Failed);
    }
    // Wallet already deployed — relayer returns walletAddress directly, no tx needed
    if let Some(addr) = resp.wallet_address.filter(|a| !a.is_empty()) {
        return Ok(WalletCreateResult::AlreadyDeployed(addr));
    }
    // Fresh deployment — a transaction was submitted; poll for confirmation
    let tx_hash = resp.transaction_hash
        .filter(|h| !h.is_empty())
        .ok_or_else(|| anyhow::anyhow!(
            "relayer WALLET-CREATE returned unexpected response: {}",
            raw_resp
        ))?;
    Ok(WalletCreateResult::Transaction(tx_hash))
}

/// Submit a signed batch of calls via the Polymarket relayer (WALLET).
///
/// Builder credentials authorise the relayer to execute the batch on the deposit wallet.
/// The EOA signature in `signature` validates the batch contents via ERC-1271.
pub async fn relayer_wallet_batch(
    client: &Client,
    owner_addr: &str,
    wallet_addr: &str,
    nonce: u64,
    deadline: u64,
    calls: Vec<serde_json::Value>,
    signature: &str,
    builder: &BuilderCredentials,
) -> Result<String> {
    use crate::config::Contracts;
    let url = format!("{}/submit", crate::config::Urls::RELAYER);
    let body_json = serde_json::json!({
        "type":      "WALLET",
        "from":      owner_addr,
        "to":        Contracts::DEPOSIT_WALLET_FACTORY, // relayer requires factory as 'to'
        "nonce":     nonce.to_string(), // relayer expects nonce as string (same as /nonce response)
        "signature": signature,
        "depositWalletParams": {
            "depositWallet": wallet_addr,
            "deadline":      deadline.to_string(),
            "calls":         calls,
        },
    });
    let body_str = serde_json::to_string(&body_json)
        .context("serializing WALLET batch body")?;
    let headers = builder_l2_headers(
        &builder.api_key,
        &builder.secret,
        &builder.passphrase,
        "POST",
        "/submit",
        &body_str,
    )?;
    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body_str);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let raw_resp = req
        .send()
        .await
        .context("relayer WALLET batch request failed")?
        .text()
        .await
        .context("relayer WALLET batch response read failed")?;
    let resp: RelayerSubmitResponse = serde_json::from_str(&raw_resp)
        .with_context(|| format!("parsing relayer WALLET batch response: {}", raw_resp))?;

    if let Some(err) = resp.error.filter(|e| !e.is_empty()) {
        anyhow::bail!("relayer WALLET batch error: {}", err);
    }
    resp.transaction_hash
        .filter(|h| !h.is_empty())
        .ok_or_else(|| anyhow::anyhow!("relayer WALLET batch returned no transaction hash"))
}

/// Fetch the current nonce for a deposit wallet from the relayer.
pub async fn get_wallet_nonce(client: &Client, owner_addr: &str) -> Result<u64> {
    let url = format!(
        "{}/nonce?address={}&type=WALLET",
        crate::config::Urls::RELAYER,
        owner_addr
    );
    let resp: WalletNonceResponse = client
        .get(&url)
        .send()
        .await
        .context("relayer /nonce request failed")?
        .json()
        .await
        .context("relayer /nonce response parse failed")?;
    Ok(resp.nonce)
}

/// Sync pUSD balance and allowance with the CLOB for a deposit wallet (signature_type=3).
/// Must be called after depositing pUSD or completing approval batches.
pub async fn sync_balance_allowance_deposit_wallet(
    client: &Client,
    wallet_addr: &str,
    signer_addr: &str,
    creds: &Credentials,
) -> Result<()> {
    use crate::config::Urls;
    let path = format!(
        "/balance-allowance/update?asset_type=COLLATERAL&signature_type=3&address={}",
        wallet_addr
    );
    let headers = crate::auth::l2_headers(signer_addr, &creds.api_key, &creds.secret, &creds.passphrase, "GET", "/balance-allowance/update", "")?;
    let url = format!("{}{}", Urls::clob(), path);
    let mut req = client.get(&url);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp = req.send().await.context("balance-allowance/update signature_type=3 failed")?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("balance-allowance/update returned {}: {}", status, body);
    }
    Ok(())
}
