use serde_json::{json, Value};

// ─── Price formatting ────────────────────────────────────────────────────────

/// Format a float price for submission to Hyperliquid.
/// Trims trailing zeros; represents integers without decimal point.
pub fn format_px(px: f64) -> String {
    if px == 0.0 {
        return "0".to_string();
    }
    // Use up to 6 significant figures (matching HL precision)
    let s = format!("{:.6}", px);
    // Trim trailing zeros after decimal
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

/// Slippage-protected limit price for market trigger orders.
/// When a trigger fires as "market", HL still needs a worst-acceptable-price.
/// Convention: 10% slippage tolerance (same as HL web UI default).
fn trigger_limit_px(trigger_px: f64, is_buy: bool) -> String {
    let px = if is_buy {
        trigger_px * 1.1
    } else {
        trigger_px * 0.9
    };
    format_px(px)
}

// ─── Entry orders ────────────────────────────────────────────────────────────

/// Build the order action payload for a market order.
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

// ─── Close ───────────────────────────────────────────────────────────────────

/// Market close: reduce-only market order in the opposite direction.
/// position_is_long: true → sell to close; false → buy to close.
pub fn build_close_action(asset: usize, position_is_long: bool, size_str: &str) -> Value {
    let is_buy = !position_is_long;
    json!({
        "type": "order",
        "orders": [{
            "a": asset,
            "b": is_buy,
            "p": "0",
            "s": size_str,
            "r": true,
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

// ─── Trigger orders (TP/SL) ──────────────────────────────────────────────────

/// Build a single trigger order JSON object (one element of the `orders` array).
/// Not a full action — used internally by the batch builders.
///
/// position_is_long: direction of the existing position (determines closing side)
/// tpsl: "sl" or "tp"
/// trigger_px_str: price that activates the order
/// limit_px_str:
///   - if is_market=true  → pass None to auto-compute 10% slippage tolerance
///   - if is_market=false → pass Some("<strict limit price>")
pub fn build_trigger_order_element(
    asset: usize,
    position_is_long: bool,
    size_str: &str,
    tpsl: &str,
    trigger_px_str: &str,
    is_market: bool,
    limit_px_override: Option<&str>,
) -> Value {
    let is_buy = !position_is_long; // close opposite of entry

    let limit_px = match limit_px_override {
        Some(px) => px.to_string(),
        None if is_market => {
            let trigger_px: f64 = trigger_px_str.parse().unwrap_or(0.0);
            trigger_limit_px(trigger_px, is_buy)
        }
        None => trigger_px_str.to_string(),
    };

    json!({
        "a": asset,
        "b": is_buy,
        "p": limit_px,
        "s": size_str,
        "r": true,
        "t": {
            "trigger": {
                "isMarket": is_market,
                "tpsl": tpsl,
                "triggerPx": trigger_px_str
            }
        }
    })
}

/// Standalone TP/SL action for an existing position.
/// Sends both orders in a single request (grouping "na").
/// Either sl_px or tp_px may be None (but not both).
pub fn build_standalone_tpsl_action(
    asset: usize,
    position_is_long: bool,
    size_str: &str,
    sl_px: Option<&str>,
    tp_px: Option<&str>,
) -> Value {
    let mut orders = vec![];

    if let Some(px) = sl_px {
        orders.push(build_trigger_order_element(
            asset, position_is_long, size_str, "sl", px, true, None,
        ));
    }
    if let Some(px) = tp_px {
        orders.push(build_trigger_order_element(
            asset, position_is_long, size_str, "tp", px, true, None,
        ));
    }

    json!({
        "type": "order",
        "orders": orders,
        "grouping": "na"
    })
}

/// Bracketed entry order: entry + TP/SL children linked via normalTpsl grouping.
/// The first element is the entry order; subsequent elements are TP/SL children.
/// Either sl_px or tp_px may be None (but not both).
pub fn build_bracketed_order_action(
    entry_order: Value,     // a pre-built order element JSON object
    asset: usize,
    position_is_long: bool, // direction of the entry (long/short)
    size_str: &str,
    sl_px: Option<&str>,
    tp_px: Option<&str>,
) -> Value {
    let entry_is_long = position_is_long;
    let mut orders = vec![entry_order];

    if let Some(px) = sl_px {
        orders.push(build_trigger_order_element(
            asset, entry_is_long, size_str, "sl", px, true, None,
        ));
    }
    if let Some(px) = tp_px {
        orders.push(build_trigger_order_element(
            asset, entry_is_long, size_str, "tp", px, true, None,
        ));
    }

    json!({
        "type": "order",
        "orders": orders,
        "grouping": "normalTpsl"
    })
}

// ─── Cancel ──────────────────────────────────────────────────────────────────

/// Build cancel action for a single order by order ID.
pub fn build_cancel_action(asset: usize, oid: u64) -> Value {
    json!({
        "type": "cancel",
        "cancels": [{
            "a": asset,
            "o": oid
        }]
    })
}

// ─── Submit ──────────────────────────────────────────────────────────────────

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
