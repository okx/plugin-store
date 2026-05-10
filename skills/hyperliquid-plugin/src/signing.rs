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

/// Round a price to HL's allowed precision.
///
/// HL spec: prices can have **up to 5 significant figures**, AND **no more
/// than (MAX_DECIMALS - sz_decimals) decimal places** where MAX_DECIMALS is 6
/// for perps. (Spot is 8, so passing perp-rounded prices through to spot is
/// always safe.)
///
/// The previous implementation used `sig_figs = sz_decimals` which silently
/// over-rounded high-priced HIP-3 markets — e.g. NVDA sz_decimals=3, mid
/// 217.495 → 3 sig figs → "217" (an integer), losing 2pp of risk-management
/// precision in TP/SL bracket prices. With 5 sig figs and the decimal-place
/// cap, NVDA 217.495 keeps as "217.5", and user inputs like 212.06 stay as
/// 212.06.
///
/// Examples (sig_figs=5, MAX=6):
///   ETH  sz=4 px=2098.4   → 5sf,1dp → "2098.4"
///   NVDA sz=3 px=217.495  → 5sf,2dp → "217.5"
///   NVDA sz=3 px=212.06   → 5sf,2dp → "212.06"
///   BTC  sz=5 px=93246.7  → 5sf,1dp → "93247"  (cap=1 wins over 5sf which→0dp)
///   BIO  sz=0 px=0.032    → 5sf,6dp → "0.032"
pub fn round_px(px: f64, sz_decimals: u32) -> String {
    if px == 0.0 {
        return "0".to_string();
    }
    const HL_PERP_MAX_DECIMALS: i32 = 6;
    const SIG_FIGS: i32 = 5;

    let mag = px.abs().log10().floor() as i32;
    // 5 sig-figs decimal places: positive when |px| < 10^5, negative for big
    // numbers that need rounding to higher integer multiples.
    let sf_dp = SIG_FIGS - mag - 1;
    // HL hard cap on decimal places.
    let cap_dp = HL_PERP_MAX_DECIMALS - (sz_decimals as i32);
    let decimal_places = sf_dp.min(cap_dp);

    let rounded = if decimal_places <= 0 {
        let factor = 10_f64.powi(-decimal_places);
        (px / factor).round() * factor
    } else {
        let factor = 10_f64.powi(decimal_places);
        (px * factor).round() / factor
    };
    if decimal_places <= 0 {
        format!("{:.0}", rounded)
    } else {
        let s = format!("{:.prec$}", rounded, prec = decimal_places as usize);
        let s = s.trim_end_matches('0').trim_end_matches('.');
        s.to_string()
    }
}

/// Slippage-protected limit price for market trigger orders.
/// When a trigger fires as "market", HL still needs a worst-acceptable-price.
/// slippage_pct: tolerance in percent (e.g. 10.0 = 10%, matching HL web UI default).
/// Uses round_px so the limit price obeys the same tick-size rules as the trigger price.
fn trigger_limit_px(trigger_px: f64, is_buy: bool, sz_decimals: u32, slippage_pct: f64) -> String {
    let multiplier = if is_buy { 1.0 + slippage_pct / 100.0 } else { 1.0 - slippage_pct / 100.0 };
    round_px(trigger_px * multiplier, sz_decimals)
}

// ─── Entry orders ────────────────────────────────────────────────────────────

/// Build the order action payload for a market order.
/// Uses IOC (Immediate-or-Cancel) limit at slippage price — HL's standard market order format.
/// slippage_px_str: worst-acceptable price (mid × 1.05 for buy, mid × 0.95 for sell).
pub fn build_market_order_action(
    asset: usize,
    is_buy: bool,
    size_str: &str,
    reduce_only: bool,
    slippage_px_str: &str,
) -> Value {
    json!({
        "type": "order",
        "orders": [{
            "a": asset,
            "b": is_buy,
            "p": slippage_px_str,
            "s": size_str,
            "r": reduce_only,
            "t": {
                "limit": {
                    "tif": "Ioc"
                }
            }
        }],
        "grouping": "na"
    })
}

/// Build the order action payload for a limit order.
/// tif: time-in-force string, e.g. "Gtc", "Alo", "Ioc"
pub fn build_limit_order_action(
    asset: usize,
    is_buy: bool,
    price_str: &str,
    size_str: &str,
    reduce_only: bool,
    tif: &str,
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
                    "tif": tif
                }
            }
        }],
        "grouping": "na"
    })
}

/// Build an `order` action carrying multiple order elements in a single signed request.
///
/// Each element of `orders` is a pre-built order JSON object produced by one of the
/// per-order builders above (or an inline `json!` following the same schema).
/// One EIP-712 signature covers the whole batch; HL returns a `statuses[]` array
/// with one entry per order, in the same order as input.
pub fn build_batch_order_action(orders: Vec<Value>) -> Value {
    json!({
        "type": "order",
        "orders": orders,
        "grouping": "na"
    })
}

// ─── Close ───────────────────────────────────────────────────────────────────

/// Market close: reduce-only IOC limit at slippage price in the opposite direction.
/// position_is_long: true → sell to close; false → buy to close.
/// slippage_px_str: worst-acceptable price (mid × 1.05 for buy, mid × 0.95 for sell).
pub fn build_close_action(asset: usize, position_is_long: bool, size_str: &str, slippage_px_str: &str) -> Value {
    let is_buy = !position_is_long;
    json!({
        "type": "order",
        "orders": [{
            "a": asset,
            "b": is_buy,
            "p": slippage_px_str,
            "s": size_str,
            "r": true,
            "t": {
                "limit": {
                    "tif": "Ioc"
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
    sz_decimals: u32,
    trigger_slippage_pct: f64,
) -> Value {
    let is_buy = !position_is_long; // close opposite of entry

    let limit_px = match limit_px_override {
        Some(px) => px.to_string(),
        None if is_market => {
            let trigger_px: f64 = trigger_px_str.parse().unwrap_or(0.0);
            trigger_limit_px(trigger_px, is_buy, sz_decimals, trigger_slippage_pct)
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
                "triggerPx": trigger_px_str,
                "tpsl": tpsl
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
    sz_decimals: u32,
    trigger_slippage_pct: f64,
) -> Value {
    let mut orders = vec![];

    if let Some(px) = sl_px {
        orders.push(build_trigger_order_element(
            asset, position_is_long, size_str, "sl", px, true, None, sz_decimals, trigger_slippage_pct,
        ));
    }
    if let Some(px) = tp_px {
        orders.push(build_trigger_order_element(
            asset, position_is_long, size_str, "tp", px, true, None, sz_decimals, trigger_slippage_pct,
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
    sz_decimals: u32,
    trigger_slippage_pct: f64,
) -> Value {
    let entry_is_long = position_is_long;
    let mut orders = vec![entry_order];

    if let Some(px) = sl_px {
        orders.push(build_trigger_order_element(
            asset, entry_is_long, size_str, "sl", px, true, None, sz_decimals, trigger_slippage_pct,
        ));
    }
    if let Some(px) = tp_px {
        orders.push(build_trigger_order_element(
            asset, entry_is_long, size_str, "tp", px, true, None, sz_decimals, trigger_slippage_pct,
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

/// Build cancel action for multiple orders in one request.
/// Each element of `orders` is (asset_index, oid).
pub fn build_batch_cancel_action(orders: &[(usize, u64)]) -> Value {
    let cancels: Vec<Value> = orders
        .iter()
        .map(|(a, o)| json!({"a": a, "o": o}))
        .collect();
    json!({
        "type": "cancel",
        "cancels": cancels
    })
}

// ─── Leverage ────────────────────────────────────────────────────────────────

/// Build an updateLeverage action.
/// Sets account-level leverage for a coin before placing an order.
/// isCross=true → cross margin; false → isolated margin.
pub fn build_update_leverage_action(asset: usize, is_cross: bool, leverage: u32) -> Value {
    json!({
        "type": "updateLeverage",
        "asset": asset,
        "isCross": is_cross,
        "leverage": leverage
    })
}

// ─── Spot/Class transfer ─────────────────────────────────────────────────────

/// Build a usdClassTransfer action (perp ↔ spot USDC).
/// amount: USD amount as f64. toPerp: true = spot→perp, false = perp→spot.
pub fn build_spot_transfer_action(amount: f64, to_perp: bool, nonce: u64) -> Value {
    json!({
        "type": "usdClassTransfer",
        "hyperliquidChain": "Mainnet",
        "signatureChainId": "0x66eee",
        "amount": format!("{}", amount),
        "toPerp": to_perp,
        "nonce": nonce
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
