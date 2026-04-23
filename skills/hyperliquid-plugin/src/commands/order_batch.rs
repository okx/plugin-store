use clap::Args;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Read;

use crate::api::{get_asset_meta, get_all_mids};
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID, ARBITRUM_CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, report_plugin_info, resolve_wallet};
use crate::signing::{build_batch_order_action, round_px, submit_exchange_request};

/// Maximum orders per batch. HL does not document a hard limit, but 50 is a
/// conservative ceiling that keeps the signed payload comfortably below any
/// practical size/latency thresholds. Callers that need more should split.
const MAX_BATCH_ORDERS: usize = 50;

#[derive(Args)]
pub struct OrderBatchArgs {
    /// Path to a JSON file containing the orders array, or `-` to read from stdin
    #[arg(long)]
    pub orders_json: String,

    /// Dry run — preview the composed action without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit the batch (without this flag, prints a preview)
    #[arg(long)]
    pub confirm: bool,

    /// Strategy ID for attribution — applied to every filled/resting order in the batch
    #[arg(long)]
    pub strategy_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OrderInput {
    coin: String,
    /// "buy" or "sell"
    side: String,
    /// Size in base units, e.g. "0.001"
    size: String,
    /// "market" or "limit"
    #[serde(default = "default_order_type")]
    r#type: String,
    /// Limit price (required when type == "limit")
    #[serde(default)]
    price: Option<String>,
    /// Time-in-force for limit orders: Gtc | Alo | Ioc. Default "Gtc".
    #[serde(default = "default_tif")]
    tif: String,
    /// Slippage percent for market orders (default 5.0 = 5%)
    #[serde(default = "default_slippage")]
    slippage: f64,
    /// Reduce-only flag
    #[serde(default)]
    reduce_only: bool,
}

fn default_order_type() -> String { "limit".to_string() }
fn default_tif() -> String { "Gtc".to_string() }
fn default_slippage() -> f64 { 5.0 }

fn fmt_size(sz: f64, decimals: u32) -> String {
    if decimals == 0 {
        format!("{:.0}", sz)
    } else {
        let s = format!("{:.prec$}", sz, prec = decimals as usize);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn read_orders_json(spec: &str) -> anyhow::Result<Vec<OrderInput>> {
    let raw = if spec == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)
            .map_err(|e| anyhow::anyhow!("read stdin: {}", e))?;
        buf
    } else {
        std::fs::read_to_string(spec)
            .map_err(|e| anyhow::anyhow!("read orders-json file '{}': {}", spec, e))?
    };
    serde_json::from_str::<Vec<OrderInput>>(&raw)
        .map_err(|e| anyhow::anyhow!("parse orders-json: {}", e))
}

pub async fn run(args: OrderBatchArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();
    let nonce = now_ms();

    let inputs = match read_orders_json(&args.orders_json) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "INVALID_ARGUMENT", "Provide a JSON array like [{\"coin\":\"BTC\",\"side\":\"buy\",\"size\":\"0.001\",\"type\":\"limit\",\"price\":\"70000\"}]."));
            return Ok(());
        }
    };

    if inputs.is_empty() {
        println!("{}", super::error_response("orders-json must contain at least one order", "INVALID_ARGUMENT", "Provide at least one order entry."));
        return Ok(());
    }

    if inputs.len() > MAX_BATCH_ORDERS {
        println!("{}", super::error_response(
            &format!("Batch size {} exceeds maximum {}", inputs.len(), MAX_BATCH_ORDERS),
            "BATCH_TOO_LARGE",
            &format!("Split the request into chunks of {} or fewer orders.", MAX_BATCH_ORDERS),
        ));
        return Ok(());
    }

    // Validate each entry before any network / signing work.
    for (i, o) in inputs.iter().enumerate() {
        let side_lc = o.side.to_lowercase();
        if side_lc != "buy" && side_lc != "sell" {
            println!("{}", super::error_response(
                &format!("orders[{}].side must be 'buy' or 'sell' (got '{}')", i, o.side),
                "INVALID_ARGUMENT", "Use side='buy' or 'sell'.",
            ));
            return Ok(());
        }
        if o.size.parse::<f64>().map(|v| v <= 0.0).unwrap_or(true) {
            println!("{}", super::error_response(
                &format!("orders[{}].size must be a positive number (got '{}')", i, o.size),
                "INVALID_ARGUMENT", "Provide a positive numeric size.",
            ));
            return Ok(());
        }
        let type_lc = o.r#type.to_lowercase();
        if type_lc != "market" && type_lc != "limit" {
            println!("{}", super::error_response(
                &format!("orders[{}].type must be 'market' or 'limit' (got '{}')", i, o.r#type),
                "INVALID_ARGUMENT", "Use type='market' or 'limit'.",
            ));
            return Ok(());
        }
        if type_lc == "limit" && o.price.as_deref().unwrap_or("").is_empty() {
            println!("{}", super::error_response(
                &format!("orders[{}].price required for limit orders", i),
                "INVALID_ARGUMENT", "Provide a limit price.",
            ));
            return Ok(());
        }
    }

    // Fetch all-mids once so we can construct market-order slippage prices and
    // auto-bump tiny sizes below the $10 notional floor.
    let mids = match get_all_mids(info).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };

    // Build each order element, resolving asset_idx + rounding per-coin.
    let mut built: Vec<Value> = Vec::with_capacity(inputs.len());
    let mut summaries: Vec<Value> = Vec::with_capacity(inputs.len());

    for (i, o) in inputs.iter().enumerate() {
        let coin = normalize_coin(&o.coin);
        let (asset_idx, sz_decimals) = match get_asset_meta(info, &coin).await {
            Ok(v) => v,
            Err(e) => {
                println!("{}", super::error_response(
                    &format!("orders[{}]: {:#}", i, e),
                    "API_ERROR", "Check coin name and connection.",
                ));
                return Ok(());
            }
        };

        let is_buy = o.side.to_lowercase() == "buy";
        let type_lc = o.r#type.to_lowercase();

        let size_f: f64 = o.size.parse().unwrap();  // already validated
        let sz_factor = 10_f64.powi(sz_decimals as i32);
        let mut size_rounded = (size_f * sz_factor).round() / sz_factor;

        let mid_f: f64 = mids.get(&coin)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        // Auto-bump to $10 notional minimum (same rule as single-order path).
        if mid_f > 0.0 {
            let n = size_rounded * mid_f;
            if n > 0.0 && n < 10.0 {
                let bumped = size_rounded + 1.0 / sz_factor;
                eprintln!(
                    "[auto-adjust] orders[{}] size {} → {} to meet $10 minimum notional (${:.2} → ${:.2})",
                    i,
                    fmt_size(size_rounded, sz_decimals),
                    fmt_size(bumped, sz_decimals),
                    n,
                    bumped * mid_f,
                );
                size_rounded = bumped;
            }
        }
        let size_str = fmt_size(size_rounded, sz_decimals);

        let (price_str, tif) = if type_lc == "market" {
            let slippage_mult = if is_buy { 1.0 + o.slippage / 100.0 } else { 1.0 - o.slippage / 100.0 };
            (round_px(mid_f * slippage_mult, sz_decimals), "Ioc".to_string())
        } else {
            let px_raw: f64 = o.price.as_deref().unwrap().parse()
                .map_err(|_| anyhow::anyhow!("orders[{}].price must be numeric", i))?;
            (round_px(px_raw, sz_decimals), o.tif.clone())
        };

        built.push(json!({
            "a": asset_idx,
            "b": is_buy,
            "p": price_str,
            "s": size_str,
            "r": o.reduce_only,
            "t": { "limit": { "tif": tif } }
        }));
        summaries.push(json!({
            "index": i,
            "coin": coin,
            "side": o.side.to_lowercase(),
            "type": type_lc,
            "size": size_str,
            "price": price_str,
            "tif": tif,
            "reduce_only": o.reduce_only,
            "notional_usd": format!("{:.2}", size_rounded * mid_f),
        }));
    }

    let action = build_batch_order_action(built);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "preview": {
                "batch_size": inputs.len(),
                "nonce": nonce,
                "orders": summaries,
            },
            "action": action
        }))?
    );

    if args.dry_run {
        eprintln!("\n[DRY RUN] Not signed or submitted.");
        return Ok(());
    }
    if !args.confirm {
        eprintln!("\n[PREVIEW] Add --confirm to sign and submit the batch.");
        return Ok(());
    }

    let wallet = match resolve_wallet(CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "WALLET_NOT_FOUND", "Run onchainos wallet addresses to verify login."));
            return Ok(());
        }
    };
    let signed = match onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "SIGNING_FAILED", "Retry. If the issue persists, check onchainos status."));
            return Ok(());
        }
    };
    let result = match submit_exchange_request(exchange, signed).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "TX_SUBMIT_FAILED", "Retry. If the issue persists, check onchainos status."));
            return Ok(());
        }
    };

    // Walk statuses[] to pair each HL response with its input summary, and
    // report attribution per successful order (filled OR resting still counts).
    let statuses = result["response"]["data"]["statuses"].as_array().cloned().unwrap_or_default();
    let mut per_order: Vec<Value> = Vec::with_capacity(statuses.len());

    let ts_now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let strategy_id = args.strategy_id.as_deref().filter(|s| !s.is_empty());

    for (i, st) in statuses.iter().enumerate() {
        let summary = summaries.get(i).cloned().unwrap_or(Value::Null);
        let oid = st["filled"]["oid"].as_u64()
            .or_else(|| st["resting"]["oid"].as_u64());
        let avg_px = st["filled"]["avgPx"].as_str().map(|s| s.to_string());
        let error = st.get("error").and_then(|e| e.as_str()).map(|s| s.to_string());

        per_order.push(json!({
            "index": i,
            "summary": summary,
            "oid": oid,
            "avg_px": avg_px,
            "filled": st.get("filled").is_some(),
            "resting": st.get("resting").is_some(),
            "error": error,
        }));

        // Attribution: report every order that produced an oid (filled or resting).
        if let (Some(sid), Some(oid_val)) = (strategy_id, oid) {
            let inp = &inputs[i];
            let coin = normalize_coin(&inp.coin);
            let side_uc = if inp.side.to_lowercase() == "buy" { "BUY" } else { "SELL" };
            let size_from_summary = summary["size"].as_str().unwrap_or(&inp.size).to_string();
            let price_for_report = avg_px.clone().unwrap_or_else(||
                summary["price"].as_str().unwrap_or("").to_string()
            );
            let report_payload = json!({
                "wallet": wallet,
                "proxyAddress": "",
                "order_id": oid_val.to_string(),
                "tx_hashes": [],
                "market_id": coin,
                "asset_id": "",
                "side": side_uc,
                "amount": size_from_summary,
                "symbol": "USDC",
                "price": price_for_report,
                "timestamp": ts_now,
                "strategy_id": sid,
                "plugin_name": "hyperliquid-plugin",
            });
            if let Err(e) = report_plugin_info(&report_payload) {
                eprintln!("[hyperliquid] Warning: report-plugin-info failed for orders[{}]: {}", i, e);
            }
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "action": "order-batch",
            "batch_size": inputs.len(),
            "orders": per_order,
            "result": result,
        }))?
    );

    Ok(())
}
