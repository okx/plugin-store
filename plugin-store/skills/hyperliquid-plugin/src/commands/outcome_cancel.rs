use clap::Args;
use serde_json::{json, Value};

use crate::api::{
    get_open_orders, outcome_asset_id, outcome_trade_coin, parse_outcome_coin,
};
use crate::config::{exchange_url, info_url, now_ms, ARBITRUM_CHAIN_ID, CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::{build_batch_cancel_action, build_cancel_action, submit_exchange_request};

/// Cancel HIP-4 outcome orders.
///
/// Three modes:
///   - `--order-id <oid>` cancels a specific order. Requires `--outcome` + `--side`
///     to compute the asset id (HL's cancel action wants `(asset, oid)` tuples).
///   - `--outcome X --side yes` (without --order-id) cancels ALL open orders on
///     that outcome leg.
///   - `--all-outcomes` cancels every outcome order across all legs.
///
/// Examples:
///   # Cancel a specific YES-leg order
///   hyperliquid-plugin outcome-cancel --outcome 2 --side yes --order-id 123456 --confirm
///
///   # Cancel all open orders on the BTC-79980-1d NO leg
///   hyperliquid-plugin outcome-cancel --outcome BTC-79980-1d --side no --confirm
///
///   # Cancel every outcome order
///   hyperliquid-plugin outcome-cancel --all-outcomes --confirm
#[derive(Args)]
pub struct OutcomeCancelArgs {
    /// Outcome identifier — numeric (e.g. `2`) or recurring semantic id
    /// (e.g. `BTC-79980-1d`). Required unless --all-outcomes.
    #[arg(long, conflicts_with = "all_outcomes")]
    pub outcome: Option<String>,

    /// Which leg: `yes` (side 0) or `no` (side 1). Required unless --all-outcomes.
    #[arg(long, value_parser = ["yes", "no"], conflicts_with = "all_outcomes")]
    pub side: Option<String>,

    /// Specific order id to cancel.
    #[arg(long)]
    pub order_id: Option<u64>,

    /// Cancel every outcome order across all legs.
    #[arg(long, conflicts_with_all = ["outcome", "side", "order_id"])]
    pub all_outcomes: bool,

    /// Show payload without signing or submitting.
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, prints a preview only).
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: OutcomeCancelArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();
    let nonce = now_ms();

    let wallet = match resolve_wallet(CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("{:#}", e),
                    "WALLET_NOT_FOUND",
                    "Run `onchainos wallet addresses` to verify login.",
                )
            );
            return Ok(());
        }
    };

    // ─── Resolve target orders to cancel ─────────────────────────────────────
    let cancels: Vec<(usize, u64)> = if args.all_outcomes {
        match collect_all_outcome_orders(info, &wallet).await {
            Ok(v) => v,
            Err(e) => return e,
        }
    } else {
        // Need both --outcome and --side here.
        let outcome_arg = match &args.outcome {
            Some(s) => s.clone(),
            None => {
                return print_invalid_arg(
                    "--outcome is required (or use --all-outcomes to cancel everything)",
                )
            }
        };
        let side_str = match &args.side {
            Some(s) => s.clone(),
            None => return print_invalid_arg("--side is required (yes or no)"),
        };
        let side: u8 = if side_str.eq_ignore_ascii_case("yes") { 0 } else { 1 };

        // Resolve outcome_id (numeric or semantic) — we use a lighter path here
        // than the order commands; just parse numeric or accept "raw" id since
        // outcomeMeta is optional for cancel.
        let outcome_id = match resolve_outcome_id(info, &outcome_arg).await {
            Some(v) => v,
            None => {
                println!(
                    "{}",
                    super::error_response(
                        &format!("Cannot resolve outcome '{}' to numeric id", outcome_arg),
                        "OUTCOME_NOT_FOUND",
                        "Use a numeric id (e.g. `--outcome 2`) or run `outcome-list` to see semantic ids.",
                    )
                );
                return Ok(());
            }
        };

        let asset_id = outcome_asset_id(outcome_id, side) as usize;
        let trade_coin = outcome_trade_coin(outcome_id, side);

        if let Some(oid) = args.order_id {
            // Single-order mode
            vec![(asset_id, oid)]
        } else {
            // Cancel-all-on-leg mode: pull open orders, filter by coin == #N
            match collect_orders_on_coin(info, &wallet, &trade_coin, asset_id).await {
                Ok(v) => v,
                Err(e) => return e,
            }
        }
    };

    if cancels.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "action": "outcome_cancel",
                "cancelled_count": 0,
                "note": "No matching outcome orders found.",
            }))?
        );
        return Ok(());
    }

    // ─── Build action ────────────────────────────────────────────────────────
    let action = if cancels.len() == 1 {
        build_cancel_action(cancels[0].0, cancels[0].1)
    } else {
        build_batch_cancel_action(&cancels)
    };

    let preview = json!({
        "ok": true,
        "stage": if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" },
        "preview": {
            "action": "outcome_cancel",
            "count": cancels.len(),
            "cancels": cancels.iter().map(|(a, o)| json!({"asset_id": a, "order_id": o})).collect::<Vec<_>>(),
            "nonce": nonce,
        },
    });

    if args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("[DRY RUN] No cancel signed or submitted.");
        return Ok(());
    }
    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!(
            "[PREVIEW] Add --confirm to cancel {} outcome order(s).",
            cancels.len()
        );
        return Ok(());
    }

    // ─── Sign & submit ───────────────────────────────────────────────────────
    let signed = match onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false) {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("{:#}", e),
                    "SIGNING_FAILED",
                    "Retry the command. If the issue persists, check `onchainos wallet status`.",
                )
            );
            return Ok(());
        }
    };
    eprintln!(
        "[outcome-cancel] Submitting cancel for {} order(s)...",
        cancels.len()
    );
    let result = match submit_exchange_request(exchange, signed).await {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("{:#}", e),
                    "TX_SUBMIT_FAILED",
                    "Retry the command.",
                )
            );
            return Ok(());
        }
    };

    let status = result["status"].as_str().unwrap_or("");
    if status != "ok" {
        println!(
            "{}",
            super::error_response(
                &format!("Hyperliquid rejected outcome-cancel: {}", serde_json::to_string(&result).unwrap_or_default()),
                "TX_REJECTED",
                "Check `result.response`. Common: order(s) already filled or already cancelled.",
            )
        );
        return Ok(());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "action": "outcome_cancel",
            "cancelled_count": cancels.len(),
            "cancels": cancels.iter().map(|(a, o)| json!({"asset_id": a, "order_id": o})).collect::<Vec<_>>(),
            "result": result,
        }))?
    );
    Ok(())
}

/// Collect (asset_id, oid) for all outcome orders the user has open.
async fn collect_all_outcome_orders(
    info: &str,
    wallet: &str,
) -> Result<Vec<(usize, u64)>, anyhow::Result<()>> {
    let orders = match get_open_orders(info, wallet).await {
        Ok(v) => v,
        Err(e) => {
            return Err({
                println!(
                    "{}",
                    super::error_response(
                        &format!("openOrders fetch failed: {:#}", e),
                        "API_ERROR",
                        "Hyperliquid info endpoint may be limited; retry shortly.",
                    )
                );
                Ok(())
            })
        }
    };
    let arr = orders.as_array().cloned().unwrap_or_default();
    let mut out = Vec::new();
    for o in arr {
        let coin = match o["coin"].as_str() {
            Some(c) if c.starts_with('#') => c,
            _ => continue,
        };
        let (outcome_id, side) = match parse_outcome_coin(coin) {
            Some(v) => v,
            None => continue,
        };
        if let Some(oid) = o["oid"].as_u64() {
            out.push((outcome_asset_id(outcome_id, side) as usize, oid));
        }
    }
    Ok(out)
}

/// Collect (asset_id, oid) for all open orders on a specific outcome leg.
async fn collect_orders_on_coin(
    info: &str,
    wallet: &str,
    trade_coin: &str,
    asset_id: usize,
) -> Result<Vec<(usize, u64)>, anyhow::Result<()>> {
    let orders = match get_open_orders(info, wallet).await {
        Ok(v) => v,
        Err(e) => {
            return Err({
                println!(
                    "{}",
                    super::error_response(
                        &format!("openOrders fetch failed: {:#}", e),
                        "API_ERROR",
                        "Hyperliquid info endpoint may be limited; retry shortly.",
                    )
                );
                Ok(())
            })
        }
    };
    let arr = orders.as_array().cloned().unwrap_or_default();
    let mut out = Vec::new();
    for o in arr {
        if o["coin"].as_str() != Some(trade_coin) {
            continue;
        }
        if let Some(oid) = o["oid"].as_u64() {
            out.push((asset_id, oid));
        }
    }
    Ok(out)
}

/// Resolve --outcome arg (numeric or semantic) to outcome_id.
/// Numeric path doesn't need outcomeMeta; semantic path does.
async fn resolve_outcome_id(info: &str, arg: &str) -> Option<u32> {
    if let Ok(id) = arg.parse::<u32>() {
        return Some(id);
    }
    let outcomes = crate::api::fetch_outcome_meta(info).await.ok()?;
    let arg_upper = arg.to_uppercase();
    for o in &outcomes {
        if let Some(r) = o.parse_recurring() {
            let candidates = [
                format!("{}-{:.0}-{}", r.underlying, r.target_price, r.period).to_uppercase(),
                format!("{}-{}-{}", r.underlying, r.target_price as u64, r.period).to_uppercase(),
            ];
            if candidates.iter().any(|c| c == &arg_upper) {
                return Some(o.outcome_id);
            }
        }
    }
    None
}

fn print_invalid_arg(msg: &str) -> anyhow::Result<()> {
    println!(
        "{}",
        super::error_response(msg, "INVALID_ARGUMENT", "See `outcome-cancel --help` for usage.")
    );
    Ok(())
}

