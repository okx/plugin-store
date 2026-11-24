use clap::Args;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::api::{
    fetch_outcome_meta, get_all_mids, get_spot_clearinghouse_state, outcome_trade_coin,
    parse_outcome_coin, OutcomeSpec,
};
use crate::config::{info_url, CHAIN_ID};
use crate::onchainos::resolve_wallet;

/// Show open HIP-4 outcome positions (binary YES/NO contracts).
///
/// Outcome positions live inside the spot subsystem and are fetched from
/// `spotClearinghouseState.balances` filtered to entries with `coin`
/// starting with `+`. Each entry is one side (YES or NO) of one outcome.
///
/// Position size can be negative — that represents a short on that side
/// (e.g. -5 on `+20` means short 5 YES shares of outcome 2, economically
/// equivalent to long 5 NO shares).
#[derive(Args)]
pub struct OutcomePositionsArgs {
    /// Wallet address to query (default: connected onchainos wallet).
    #[arg(long)]
    pub address: Option<String>,

    /// Include positions with size 0 (closed but balance row still in API output).
    #[arg(long)]
    pub show_zero: bool,
}

pub async fn run(args: OutcomePositionsArgs) -> anyhow::Result<()> {
    let info = info_url();

    let address = match &args.address {
        Some(a) => a.clone(),
        None => match resolve_wallet(CHAIN_ID) {
            Ok(v) => v,
            Err(e) => {
                println!(
                    "{}",
                    super::error_response(
                        &format!("{:#}", e),
                        "WALLET_NOT_FOUND",
                        "Run `onchainos wallet addresses` to verify login, or pass --address explicitly.",
                    )
                );
                return Ok(());
            }
        },
    };

    // Fetch in parallel: spot clearinghouse (positions) + allMids (current prices)
    // + outcomeMeta (names + descriptions).
    let (state_res, mids_res, meta_res) = tokio::join!(
        get_spot_clearinghouse_state(info, &address),
        get_all_mids(info),
        fetch_outcome_meta(info),
    );

    let state = match state_res {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("spotClearinghouseState fetch failed: {:#}", e),
                    "API_ERROR",
                    "Hyperliquid info endpoint may be limited; retry shortly.",
                )
            );
            return Ok(());
        }
    };
    let mids = mids_res.unwrap_or_else(|_| Value::Object(serde_json::Map::new()));
    let outcomes_meta = meta_res.unwrap_or_default();

    // Build outcome_id -> OutcomeSpec map for quick lookup.
    let meta_by_id: HashMap<u32, &OutcomeSpec> =
        outcomes_meta.iter().map(|o| (o.outcome_id, o)).collect();

    let empty = vec![];
    let balances = state["balances"].as_array().unwrap_or(&empty);

    let mut positions: Vec<Value> = Vec::new();
    for b in balances {
        let coin = match b["coin"].as_str() {
            Some(s) if s.starts_with('+') => s,
            _ => continue,
        };
        // Only HIP-4 outcome legs (coin = "+<encoding>"); skip other spot tokens.
        let (outcome_id, side) = match parse_outcome_coin(coin) {
            Some(v) => v,
            None => continue,
        };
        let total: f64 = b["total"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        if total == 0.0 && !args.show_zero {
            continue;
        }
        let hold: f64 = b["hold"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let entry_ntl: f64 = b["entryNtl"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        // Look up current price via the trading-context coin form `#N`.
        let trade_coin = outcome_trade_coin(outcome_id, side);
        let current_price: f64 = mids
            .get(&trade_coin)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        // Mark-to-market: USDH credited if holding settles in your favor.
        // Long YES at $0.65 with size=5: current_value = size * mid = 5 * 0.65 = $3.25
        // unrealized_pnl = current_value - entry_ntl (signed; negative size handled
        // because total carries its own sign — current_value reflects it correctly).
        let current_value = total * current_price;
        let unrealized_pnl = current_value - entry_ntl;

        let avg_entry_price = if total != 0.0 {
            (entry_ntl / total).abs()
        } else {
            0.0
        };

        // Resolve human-readable metadata from outcomeMeta (if available).
        let meta = meta_by_id.get(&outcome_id);
        let (name, description, side_name, semantic_id) = match meta {
            Some(o) => {
                let side_name = if side == 0 {
                    o.side_names.0.clone()
                } else {
                    o.side_names.1.clone()
                };
                let sid = o.parse_recurring().map(|r| {
                    format!("{}-{:.0}-{}", r.underlying, r.target_price, r.period)
                });
                (
                    o.name.clone(),
                    o.description.clone(),
                    side_name,
                    sid.unwrap_or_default(),
                )
            }
            None => (
                "(unknown — outcome not in current outcomeMeta)".to_string(),
                String::new(),
                if side == 0 { "Yes".into() } else { "No".into() },
                String::new(),
            ),
        };

        positions.push(json!({
            "balance_coin": coin,
            "trade_coin": trade_coin,
            "outcome_id": outcome_id,
            "side": side,
            "side_name": side_name,
            "name": name,
            "description": description,
            "semantic_id": semantic_id,
            "size": format!("{:.6}", total),
            "size_num": total,
            "hold": format!("{:.6}", hold),
            "entry_ntl_usdh": format!("{:.4}", entry_ntl),
            "avg_entry_price": format!("{:.4}", avg_entry_price),
            "current_price": format!("{:.4}", current_price),
            "current_value_usdh": format!("{:.4}", current_value),
            "unrealized_pnl_usdh": format!("{:+.4}", unrealized_pnl),
        }));
    }

    // Sort by absolute unrealized PnL desc (biggest movers first).
    positions.sort_by(|a, b| {
        let ap = a["unrealized_pnl_usdh"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .map(|v| v.abs())
            .unwrap_or(0.0);
        let bp = b["unrealized_pnl_usdh"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .map(|v| v.abs())
            .unwrap_or(0.0);
        bp.partial_cmp(&ap).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Strip internal sort helper.
    let positions: Vec<Value> = positions
        .into_iter()
        .map(|mut p| {
            if let Some(obj) = p.as_object_mut() {
                obj.remove("size_num");
            }
            p
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "wallet": address,
            "count": positions.len(),
            "positions": positions,
            "note": "HIP-4 positions live in spot. Negative `size` = short on that side. Settlement is automatic at expiry (YES credit 1 USDH, NO credit 0 — or vice versa). PnL estimate uses live mid; for resting orders see `orders` (TODO: outcome-aware variant).",
        }))?
    );
    Ok(())
}
