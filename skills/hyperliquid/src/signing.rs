use serde_json::{json, Value};

/// Build the order action payload for a market order.
/// Hyperliquid market orders use a trigger type with isMarket=true.
///
/// asset: asset index (from meta.universe[i])
/// is_buy: true for long/buy, false for short/sell
/// size_str: size as string (e.g. "0.01")
/// reduce_only: true to reduce only
pub fn build_market_order_action(
    asset: usize,
    is_buy: bool,
    size_str: &str,
    reduce_only: bool,
) -> Value {
    json!({
        "type": "order",
        "orders": [{
            "a": asset,
            "b": is_buy,
            "p": "0",
            "s": size_str,
            "r": reduce_only,
            "t": {
                "trigger": {
                    "isMarket": true,
                    "tpsl": "tp",
                    "triggerPx": "0"
                }
            }
        }],
        "grouping": "na"
    })
}

/// Build the order action payload for a limit order (GTC).
///
/// asset: asset index
/// is_buy: true for buy/long, false for sell/short
/// price_str: limit price as string
/// size_str: size as string
/// reduce_only: true to reduce only
pub fn build_limit_order_action(
    asset: usize,
    is_buy: bool,
    price_str: &str,
    size_str: &str,
    reduce_only: bool,
) -> Value {
    json!({
        "type": "order",
        "orders": [{
            "a": asset,
            "b": is_buy,
            "p": price_str,
            "s": size_str,
            "r": reduce_only,
            "t": {
                "limit": {
                    "tif": "Gtc"
                }
            }
        }],
        "grouping": "na"
    })
}

/// Build cancel action for a single order by order ID.
///
/// asset: asset index
/// oid: order ID (u64)
pub fn build_cancel_action(asset: usize, oid: u64) -> Value {
    json!({
        "type": "cancel",
        "cancels": [{
            "a": asset,
            "o": oid
        }]
    })
}

/// POST a signed exchange request to Hyperliquid.
pub async fn submit_exchange_request(
    exchange_url: &str,
    body: Value,
) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();
    let resp = client
        .post(exchange_url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Exchange HTTP request failed: {}", e))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read exchange response: {}", e))?;

    if !status.is_success() {
        anyhow::bail!("Exchange API error {}: {}", status, text);
    }

    serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Failed to parse exchange response: {} — body: {}", e, text))
}
