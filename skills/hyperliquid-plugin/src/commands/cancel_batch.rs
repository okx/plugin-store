use clap::Args;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Read;

use crate::api::get_asset_meta;
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID, ARBITRUM_CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::{build_batch_cancel_action, submit_exchange_request};

/// Maximum cancels per batch. Same rationale as MAX_BATCH_ORDERS in order_batch.rs.
const MAX_BATCH_CANCELS: usize = 50;

#[derive(Args)]
pub struct CancelBatchArgs {
    /// Shorthand: all oids share the same coin. Requires --oids.
    #[arg(long)]
    pub coin: Option<String>,

    /// Comma-separated list of oids when --coin is provided.
    #[arg(long, value_delimiter = ',')]
    pub oids: Vec<u64>,

    /// JSON array for multi-coin cancels: [{"coin":"BTC","oid":123},{"coin":"ETH","oid":456}]
    /// Pass a file path or `-` for stdin. Mutually exclusive with --coin/--oids.
    #[arg(long, conflicts_with_all = ["coin", "oids"])]
    pub cancels_json: Option<String>,

    /// Dry run — preview the composed action without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, prints a preview)
    #[arg(long)]
    pub confirm: bool,

    /// Strategy ID — passthrough only; cancels do not generate attribution
    /// reports because no new fill is produced.
    #[arg(long)]
    pub strategy_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct CancelInput {
    coin: String,
    oid: u64,
}

fn read_cancels_json(spec: &str) -> anyhow::Result<Vec<CancelInput>> {
    let raw = if spec == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)
            .map_err(|e| anyhow::anyhow!("read stdin: {}", e))?;
        buf
    } else {
        std::fs::read_to_string(spec)
            .map_err(|e| anyhow::anyhow!("read cancels-json file '{}': {}", spec, e))?
    };
    serde_json::from_str::<Vec<CancelInput>>(&raw)
        .map_err(|e| anyhow::anyhow!("parse cancels-json: {}", e))
}

pub async fn run(args: CancelBatchArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();
    let nonce = now_ms();

    // Collect cancel inputs from either --coin/--oids or --cancels-json.
    let cancels_raw: Vec<(String, u64)> = if let Some(spec) = &args.cancels_json {
        match read_cancels_json(spec) {
            Ok(v) => v.into_iter().map(|c| (c.coin, c.oid)).collect(),
            Err(e) => {
                println!("{}", super::error_response(&format!("{:#}", e), "INVALID_ARGUMENT", "Provide a JSON array like [{\"coin\":\"BTC\",\"oid\":123}]."));
                return Ok(());
            }
        }
    } else {
        let coin = match &args.coin {
            Some(c) if !c.is_empty() => c.clone(),
            _ => {
                println!("{}", super::error_response("Must provide either --coin + --oids, or --cancels-json", "INVALID_ARGUMENT", "Example: --coin BTC --oids 111,222 or --cancels-json orders.json"));
                return Ok(());
            }
        };
        if args.oids.is_empty() {
            println!("{}", super::error_response("--oids is required when --coin is set", "INVALID_ARGUMENT", "Provide a comma-separated list of order IDs."));
            return Ok(());
        }
        args.oids.iter().map(|o| (coin.clone(), *o)).collect()
    };

    if cancels_raw.is_empty() {
        println!("{}", super::error_response("no cancels to submit", "INVALID_ARGUMENT", "Provide at least one oid."));
        return Ok(());
    }
    if cancels_raw.len() > MAX_BATCH_CANCELS {
        println!("{}", super::error_response(
            &format!("Batch size {} exceeds maximum {}", cancels_raw.len(), MAX_BATCH_CANCELS),
            "BATCH_TOO_LARGE",
            &format!("Split into chunks of {} or fewer.", MAX_BATCH_CANCELS),
        ));
        return Ok(());
    }

    // Resolve asset_idx per coin (cached per unique coin to avoid redundant API calls).
    use std::collections::HashMap;
    let mut asset_cache: HashMap<String, usize> = HashMap::new();
    let mut resolved: Vec<(usize, u64)> = Vec::with_capacity(cancels_raw.len());
    let mut summaries: Vec<Value> = Vec::with_capacity(cancels_raw.len());

    for (i, (coin_raw, oid)) in cancels_raw.iter().enumerate() {
        let coin = normalize_coin(coin_raw);
        let asset_idx = if let Some(idx) = asset_cache.get(&coin) {
            *idx
        } else {
            match get_asset_meta(info, &coin).await {
                Ok((idx, _)) => { asset_cache.insert(coin.clone(), idx); idx }
                Err(e) => {
                    println!("{}", super::error_response(
                        &format!("cancels[{}]: {:#}", i, e),
                        "API_ERROR", "Check coin name and connection.",
                    ));
                    return Ok(());
                }
            }
        };
        resolved.push((asset_idx, *oid));
        summaries.push(json!({"index": i, "coin": coin, "oid": oid, "asset_index": asset_idx}));
    }

    let action = build_batch_cancel_action(&resolved);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "preview": {
                "batch_size": resolved.len(),
                "nonce": nonce,
                "cancels": summaries,
            },
            "action": action
        }))?
    );

    if args.dry_run {
        eprintln!("\n[DRY RUN] Not signed or submitted.");
        return Ok(());
    }
    if !args.confirm {
        eprintln!("\n[PREVIEW] Add --confirm to sign and submit the batch cancel.");
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

    // Walk statuses[] to report which cancels succeeded / failed.
    let statuses = result["response"]["data"]["statuses"].as_array().cloned().unwrap_or_default();
    let mut per_cancel: Vec<Value> = Vec::with_capacity(statuses.len());
    for (i, st) in statuses.iter().enumerate() {
        let summary = summaries.get(i).cloned().unwrap_or(Value::Null);
        let ok = st.as_str() == Some("success");
        let error = if !ok { st.get("error").and_then(|e| e.as_str()).map(|s| s.to_string()) } else { None };
        per_cancel.push(json!({
            "index": i,
            "summary": summary,
            "ok": ok,
            "error": error,
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "action": "cancel-batch",
            "batch_size": resolved.len(),
            "cancels": per_cancel,
            "result": result,
        }))?
    );

    Ok(())
}
