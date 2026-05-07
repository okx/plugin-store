use clap::Args;
use serde_json::{json, Value};

use crate::api::{
    fetch_outcome_meta, get_all_mids, outcome_trade_coin, OutcomeSpec, RecurringSpec,
};
use crate::config::info_url;

/// List HIP-4 outcome markets (binary YES/NO contracts on real-world events).
///
/// Outcomes are fully-collateralized in USDH. Price represents implied probability
/// of YES resolution (range 0.001..0.999). Settlement is automatic at expiry.
///
/// Examples:
///   # List all outcomes with current Yes/No prices
///   hyperliquid-plugin outcome-list
///
///   # Filter to recurring-priceBinary outcomes only (e.g. BTC > $X by date)
///   hyperliquid-plugin outcome-list --recurring-only
///
///   # Sort by implied YES probability descending
///   hyperliquid-plugin outcome-list --sort prob
#[derive(Args)]
pub struct OutcomeListArgs {
    /// Only show recurring-priceBinary outcomes (auto-deployed by protocol on
    /// fixed cadence; e.g. "BTC > $79980 in 1d"). Hides categorical questions.
    #[arg(long)]
    pub recurring_only: bool,

    /// Sort key: `id` (default; outcome id ascending) | `prob` (implied YES
    /// probability descending) | `expiry` (recurring expiry ascending).
    #[arg(long, default_value = "id")]
    pub sort: String,

    /// Max rows to return. 0 = no limit. Default: 50.
    #[arg(long, default_value_t = 50)]
    pub limit: usize,
}

pub async fn run(args: OutcomeListArgs) -> anyhow::Result<()> {
    let info = info_url();

    // Fetch in parallel: outcome universe + allMids (which has #<enc> keys for
    // current outcome prices).
    let (meta_res, mids_res) = tokio::join!(
        fetch_outcome_meta(info),
        get_all_mids(info),
    );

    let outcomes = match meta_res {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("outcomeMeta fetch failed: {:#}", e),
                    "API_ERROR",
                    "Hyperliquid info endpoint may be limited; retry shortly. HIP-4 went live on mainnet 2026-05-02; if this skill fails consistently the API contract may have changed.",
                )
            );
            return Ok(());
        }
    };

    let mids = mids_res.unwrap_or_else(|_| Value::Object(serde_json::Map::new()));

    // Build entries
    let mut entries: Vec<Value> = Vec::with_capacity(outcomes.len());
    for o in &outcomes {
        let recurring = o.parse_recurring();
        if args.recurring_only && recurring.is_none() {
            continue;
        }
        entries.push(build_entry(o, &recurring, &mids));
    }

    // Sort
    match args.sort.as_str() {
        "prob" => entries.sort_by(|a, b| {
            b["yes_price_num"]
                .as_f64()
                .unwrap_or(0.0)
                .partial_cmp(&a["yes_price_num"].as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "expiry" => entries.sort_by(|a, b| {
            a["expiry"]
                .as_str()
                .unwrap_or("")
                .cmp(b["expiry"].as_str().unwrap_or(""))
        }),
        _ => entries.sort_by(|a, b| {
            a["outcome_id"]
                .as_u64()
                .unwrap_or(0)
                .cmp(&b["outcome_id"].as_u64().unwrap_or(0))
        }),
    }

    let count_total = entries.len();
    let shown: Vec<Value> = if args.limit == 0 {
        entries
    } else {
        entries.into_iter().take(args.limit).collect()
    };
    // Strip internal sort helper field from output
    let shown: Vec<Value> = shown
        .into_iter()
        .map(|mut e| {
            if let Some(obj) = e.as_object_mut() {
                obj.remove("yes_price_num");
            }
            e
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "count_total": count_total,
            "count_shown": shown.len(),
            "sort": args.sort,
            "outcomes": shown,
            "note": "Yes/No prices are implied probability (0.001..0.999). Trade via `outcome-buy --semantic-id <id>` (TODO v0.4.2). Position appears in spotClearinghouseState as `+<encoding>` coin. Settlement is automatic at expiry; no claim action needed.",
        }))?
    );
    Ok(())
}

/// Build a structured entry for one outcome including price + implied probability.
fn build_entry(o: &OutcomeSpec, recurring: &Option<RecurringSpec>, mids: &Value) -> Value {
    let yes_coin = outcome_trade_coin(o.outcome_id, 0);
    let no_coin = outcome_trade_coin(o.outcome_id, 1);
    let yes_px_str = mids.get(&yes_coin).and_then(|v| v.as_str()).unwrap_or("");
    let no_px_str = mids.get(&no_coin).and_then(|v| v.as_str()).unwrap_or("");
    let yes_px: Option<f64> = yes_px_str.parse().ok();
    // Implied probability = yes price (already 0..1 range per HIP-4 spec)
    let implied_prob = yes_px.map(|p| (p * 100.0 * 100.0).round() / 100.0); // pct, 2 decimals

    let mut entry = json!({
        "outcome_id": o.outcome_id,
        "name": o.name,
        "description": o.description,
        "yes_coin": yes_coin,
        "no_coin": no_coin,
        "yes_price": yes_px_str,
        "no_price": no_px_str,
        "yes_price_num": yes_px.unwrap_or(0.0),  // internal sort helper, stripped before output
        "implied_yes_probability_pct": implied_prob,
        "yes_side_name": o.side_names.0,
        "no_side_name": o.side_names.1,
    });

    if let Some(r) = recurring {
        if let Some(obj) = entry.as_object_mut() {
            obj.insert("recurring".into(), json!(true));
            obj.insert("class".into(), json!(r.class));
            obj.insert("underlying".into(), json!(r.underlying));
            obj.insert("target_price".into(), json!(r.target_price));
            obj.insert("expiry".into(), json!(r.expiry));
            obj.insert("period".into(), json!(r.period));
            // Human-friendly semantic id: BTC-79980-1d
            let semantic_id = format!(
                "{}-{:.0}-{}",
                r.underlying, r.target_price, r.period
            );
            obj.insert("semantic_id".into(), json!(semantic_id));
        }
    } else {
        if let Some(obj) = entry.as_object_mut() {
            obj.insert("recurring".into(), json!(false));
        }
    }

    entry
}
