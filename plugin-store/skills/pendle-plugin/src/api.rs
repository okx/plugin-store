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

/// GET /v2/sdk/{chainId}/convert — Pendle Hosted SDK v2 endpoint.
///
/// Used for multi-token operations where the v3 POST endpoint cannot classify the action:
///   - mintPyFromToken:  tokensIn = underlying, tokensOut = "pt_addr,yt_addr"
///   - redeemPyToToken:  tokensIn = "pt_addr,yt_addr", amountsIn = "pt_amt,yt_amt", tokensOut = underlying
///
/// Per the official Pendle hosted-SDK examples (pendle-finance/pendle-examples-public),
/// the v2 GET endpoint is the correct path for mint/redeem PY. The response schema is
/// identical to v3 POST (routes[].tx.data / routes[].tx.to) so all existing extractors work.
pub async fn sdk_convert_v2_get(
    chain_id: u64,
    receiver: &str,
    tokens_in: &str,   // comma-separated for multi-input (e.g. "pt_addr,yt_addr")
    amounts_in: &str,  // comma-separated for multi-input (e.g. "pt_wei,yt_wei")
    tokens_out: &str,  // comma-separated for multi-output (e.g. "pt_addr,yt_addr")
    slippage: f64,
    api_key: Option<&str>,
) -> anyhow::Result<Value> {
    let client = build_client(api_key)?;
    let url = format!(
        "{}/v2/sdk/{}/convert?tokensIn={}&amountsIn={}&tokensOut={}&receiver={}&slippage={}&enableAggregator=true",
        PENDLE_API_BASE, chain_id,
        tokens_in, amounts_in, tokens_out, receiver, slippage
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to call Pendle SDK v2 convert API")?;

    let status = resp.status();
    let body_text = resp.text().await.context("Failed to read SDK v2 convert response body")?;

    if !status.is_success() {
        anyhow::bail!(
            "Pendle SDK v2 convert returned HTTP {}: {}",
            status.as_u16(),
            body_text.trim()
        );
    }

    let response: Value = serde_json::from_str(&body_text)
        .context("Failed to parse SDK v2 convert response")?;
    Ok(response)
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

    let status = resp.status();
    let body_text = resp.text().await.context("Failed to read SDK convert response body")?;

    if !status.is_success() {
        anyhow::bail!(
            "Pendle SDK convert returned HTTP {}: {}",
            status.as_u16(),
            body_text.trim()
        );
    }

    let response: Value = serde_json::from_str(&body_text)
        .context("Failed to parse SDK convert response")?;
    Ok(response)
}

pub struct SdkTokenAmount {
    pub token: String,
    pub amount: String,
}

/// Validate calldata and router address returned by the Pendle Hosted SDK.
///
/// Guards against a supply-chain attack where a compromised SDK response returns
/// calldata that drains the wallet via a standard ERC-20/ERC-721 operation, or
/// routes funds through an unknown contract.
///
/// Checks (in order):
///  1. Calldata is well-formed hex with at least a 4-byte selector.
///  2. router_to is Pendle Router v3 or a known DEX aggregator.
///  3. Selector is not a standard token drain operation (transfer, transferFrom,
///     approve, setApprovalForAll, safeTransferFrom).
pub fn validate_sdk_calldata(calldata: &str, router_to: &str) -> anyhow::Result<()> {
    // 1. Well-formed hex, at least 4 bytes (8 hex chars after 0x prefix)
    let hex = calldata.strip_prefix("0x").unwrap_or(calldata);
    if hex.len() < 8 {
        anyhow::bail!(
            "SDK returned malformed calldata (too short — expected at least 4 bytes): '{}'",
            calldata
        );
    }
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!(
            "SDK returned non-hex calldata: '{}'",
            &calldata[..calldata.len().min(20)]
        );
    }

    // 2. router_to must be in the Pendle / known aggregator whitelist
    let router_lower = router_to.to_lowercase();
    let known_routers: &[&str] = &[
        "0x888888888889758f76e7103c6cbf23abbf58f946", // Pendle Router v3
        "0x1111111254eeb25477b68fb85ed929f73a960582", // 1inch v5
        "0x111111125421ca6dc452d289314280a0f8842a65", // 1inch v6
        "0xdef1c0ded9bec7f1a1670819833240f027b25eff", // 0x Exchange Proxy
        "0xe592427a0aece92de3edee1f18e0157c05861564", // Uniswap v3 SwapRouter
    ];
    if !known_routers.contains(&router_lower.as_str()) {
        anyhow::bail!(
            "SDK returned unrecognised router address '{}'. Expected Pendle Router v3 \
             (0x8888...8946) or a known DEX aggregator. Aborting to prevent funds being \
             routed to an unexpected contract.",
            router_to
        );
    }

    // 3. Selector must not be a standard ERC-20/ERC-721 token operation
    let selector = hex[..8].to_lowercase();
    let dangerous: &[(&str, &str)] = &[
        ("a9059cbb", "transfer(address,uint256)"),
        ("23b872dd", "transferFrom(address,address,uint256)"),
        ("095ea7b3", "approve(address,uint256)"),
        ("a22cb465", "setApprovalForAll(address,bool)"),
        ("42842e0e", "safeTransferFrom(address,address,uint256)"),
        ("b88d4fde", "safeTransferFrom(address,address,uint256,bytes)"),
    ];
    for (sel, sig) in dangerous {
        if selector == *sel {
            anyhow::bail!(
                "SDK returned calldata with selector 0x{} ({}). This is a token operation, \
                 not a Pendle Router call. Aborting to prevent unintended token transfer or approval.",
                sel, sig
            );
        }
    }

    Ok(())
}

/// Extract calldata and router address from SDK convert response.
/// Validates the calldata with `validate_sdk_calldata` before returning.
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
    validate_sdk_calldata(&calldata, &to)?;
    Ok((calldata, to))
}

/// Extract the expected output amount from SDK convert response.
///
/// Pendle SDK v3 response layout:
///   routes[0].outputs[0].amount  ← primary (confirmed via live API)
///   routes[0].data.*             ← fallback for older SDK shapes
pub fn extract_amount_out(response: &Value) -> Option<String> {
    let route = response["routes"].as_array()?.first()?;

    // Primary: Pendle SDK v3 places output amount at routes[0].outputs[0].amount
    if let Some(outputs) = route["outputs"].as_array() {
        if let Some(first) = outputs.first() {
            if let Some(s) = first["amount"].as_str() {
                return Some(s.to_string());
            }
            if let Some(n) = first["amount"].as_u64() {
                return Some(n.to_string());
            }
        }
    }

    // Fallback: older SDK field names under routes[0].data
    let data = &route["data"];
    for field in &["netPtOut", "netYtOut", "netLpOut", "netTokenOut", "amountOut", "outputAmount"] {
        if let Some(s) = data[field].as_str() {
            return Some(s.to_string());
        }
        if let Some(n) = data[field].as_u64() {
            return Some(n.to_string());
        }
    }
    None
}

/// Extract price impact from SDK convert response.
/// The SDK reports priceImpact as a negative decimal (e.g. -0.015 = 1.5% loss).
/// Returns Some(pct) as a positive percentage value, or None if the field is absent.
pub fn extract_price_impact(response: &Value) -> Option<f64> {
    let route = response["routes"].as_array()?.first()?;
    let impact = route["data"]["priceImpact"]
        .as_f64()
        .or_else(|| route["data"]["price_impact"].as_f64())?;
    Some(impact.abs() * 100.0)
}

/// Client-side minimum-output guard.
///
/// After getting a quote from the SDK, compare the expected output against the
/// user-supplied minimum.  Values of "0" or "" are treated as "no minimum" and
/// always pass.  If the quote is below the minimum the command aborts before any
/// approval or on-chain call, saving gas and preventing a worse-than-expected fill.
///
/// `label` is the human-readable token name used in the error message ("pt", "yt",
/// "lp", "token").
pub fn check_min_out(expected: &Option<String>, min_out: &str, label: &str) -> anyhow::Result<()> {
    let min: u128 = min_out.parse().unwrap_or(0);
    if min == 0 {
        return Ok(());
    }
    if let Some(expected_str) = expected {
        let got: u128 = expected_str.parse().unwrap_or(0);
        if got < min {
            anyhow::bail!(
                "SDK quote {} wei {} is below your --min-{}-out {} wei. \
                 Slippage may be too tight or the market moved. \
                 Lower --min-{}-out or increase --slippage before retrying.",
                got, label, label, min, label
            );
        }
    }
    Ok(())
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
