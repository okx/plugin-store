use clap::Args;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::{api, config, onchainos};

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address (optional; defaults to current onchainos Solana wallet)
    #[arg(long)]
    pub wallet: Option<String>,

    /// Market address (optional; defaults to main market)
    #[arg(long)]
    pub market: Option<String>,
}

/// Pre-resolve all reserve addresses that appear in a list of obligations.
/// Unknown reserves are fetched concurrently from the Kamino metrics API.
async fn prefetch_reserves(
    market: &str,
    obligations: &Value,
) -> HashMap<String, (String, u32)> {
    // Collect all unique reserve addresses that aren't in the static config
    let mut unique: HashSet<String> = HashSet::new();
    if let Some(arr) = obligations.as_array() {
        for o in arr {
            let state = &o["state"];
            let empty_deps: Vec<Value> = vec![];
            for dep in state["deposits"].as_array().unwrap_or(&empty_deps) {
                if let Some(r) = dep["depositReserve"].as_str() {
                    if config::reserve_symbol(r) == "UNKNOWN" {
                        unique.insert(r.to_string());
                    }
                }
            }
            let empty_bors: Vec<Value> = vec![];
            for bor in state["borrows"].as_array().unwrap_or(&empty_bors) {
                if let Some(r) = bor["borrowReserve"].as_str() {
                    if config::reserve_symbol(r) == "UNKNOWN" {
                        unique.insert(r.to_string());
                    }
                }
            }
        }
    }

    // Fetch all unknown reserves concurrently
    let futures: Vec<_> = unique
        .into_iter()
        .map(|r| {
            let m = market.to_string();
            async move {
                let info = api::get_reserve_info(&m, &r).await;
                (r, info)
            }
        })
        .collect();

    let mut cache = HashMap::new();
    for fut in futures {
        let (reserve, info) = fut.await;
        if let Some((symbol, decimals)) = info {
            cache.insert(reserve, (symbol, decimals));
        }
    }
    cache
}

/// Fetch USD prices for every reserve that appears in the obligations.
/// Runs concurrently. Silently skips failures.
async fn prefetch_prices(market: &str, obligations: &Value) -> HashMap<String, f64> {
    let mut all: HashSet<String> = HashSet::new();
    if let Some(arr) = obligations.as_array() {
        for o in arr {
            let state = &o["state"];
            let empty: Vec<Value> = vec![];
            for dep in state["deposits"].as_array().unwrap_or(&empty) {
                if let Some(r) = dep["depositReserve"].as_str() {
                    all.insert(r.to_string());
                }
            }
            for bor in state["borrows"].as_array().unwrap_or(&empty) {
                if let Some(r) = bor["borrowReserve"].as_str() {
                    all.insert(r.to_string());
                }
            }
        }
    }

    let futures: Vec<_> = all
        .into_iter()
        .map(|r| {
            let m = market.to_string();
            async move {
                let price = api::get_reserve_price_usd(&m, &r).await;
                (r, price)
            }
        })
        .collect();

    let mut map = HashMap::new();
    for fut in futures {
        let (reserve, price) = fut.await;
        if let Some(p) = price {
            map.insert(reserve, p);
        }
    }
    map
}

fn parse_state_positions(
    items: &[Value],
    reserve_key: &str,
    amount_key: &str,
    resolved: &HashMap<String, (String, u32)>,
    prices: &HashMap<String, f64>,
) -> Vec<Value> {
    const NULL_RESERVE: &str = "11111111111111111111111111111111";
    items
        .iter()
        .filter(|item| {
            let reserve = item.get(reserve_key).and_then(|v| v.as_str()).unwrap_or("");
            reserve != NULL_RESERVE && !reserve.is_empty()
        })
        .filter(|item| {
            item.get(amount_key)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0)
                > 0
        })
        .map(|item| {
            let reserve = item.get(reserve_key).and_then(|v| v.as_str()).unwrap_or("");
            let (symbol, decimals) = resolved
                .get(reserve)
                .cloned()
                .unwrap_or_else(|| {
                    let sym = config::reserve_symbol(reserve);
                    let dec = config::reserve_decimals(reserve);
                    (sym.to_string(), dec)
                });

            let raw_amount = item
                .get(amount_key)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let amount_display = format!(
                "{:.decimals$}",
                raw_amount as f64 / 10f64.powi(decimals as i32),
                decimals = (decimals as usize).min(9)
            );
            let sf = item
                .get("marketValueSf")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u128>().ok())
                .unwrap_or(0);
            // marketValueSf is a Q64.60 fixed-point USD value from Kamino.
            // When missing or zero (some LSTs), fall back to price × amount.
            // If price is also unavailable, output null rather than wrong data.
            let value_usd: serde_json::Value = if sf > 0 {
                serde_json::json!(format!("{:.6}", sf as f64 / (1u128 << 60) as f64))
            } else if let Some(&price) = prices.get(reserve) {
                let amount_f = raw_amount as f64 / 10f64.powi(decimals as i32);
                serde_json::json!(format!("{:.6}", amount_f * price))
            } else {
                serde_json::Value::Null
            };
            serde_json::json!({
                "token":       symbol,
                "reserve":     reserve,
                "amount":      amount_display,
                "amount_raw":  raw_amount.to_string(),
                "value_usd":   value_usd,
            })
        })
        .collect()
}

fn summarise_obligation(
    o: &Value,
    resolved: &HashMap<String, (String, u32)>,
    prices: &HashMap<String, f64>,
) -> Value {
    let stats = o.get("refreshedStats").cloned().unwrap_or(Value::Null);
    let state = o.get("state").cloned().unwrap_or(Value::Null);

    let empty: Vec<Value> = vec![];
    let deposits = parse_state_positions(
        state.get("deposits").and_then(|v| v.as_array()).unwrap_or(&empty),
        "depositReserve",
        "depositedAmount",
        resolved,
        prices,
    );
    let borrows = parse_state_positions(
        state.get("borrows").and_then(|v| v.as_array()).unwrap_or(&empty),
        "borrowReserve",
        "borrowedAmountOutsideElevationGroups",
        resolved,
        prices,
    );

    serde_json::json!({
        "obligation": o.get("obligationAddress").and_then(|v| v.as_str()).unwrap_or(""),
        "tag": o.get("humanTag").and_then(|v| v.as_str()).unwrap_or(""),
        "deposits": deposits,
        "borrows":  borrows,
        "stats": {
            "net_value_usd":        stats.get("netAccountValue"),
            "total_deposit_usd":    stats.get("userTotalDeposit"),
            "total_borrow_usd":     stats.get("userTotalBorrow"),
            "loan_to_value":        stats.get("loanToValue"),
            "borrow_utilization":   stats.get("borrowUtilization"),
            "liquidation_ltv":      stats.get("liquidationLtv"),
        }
    })
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let wallet = match args.wallet {
        Some(w) => w,
        None => match onchainos::resolve_wallet_solana() {
            Ok(w) => w,
            Err(e) => {
                println!("{}", super::error_response(&e, None));
                return Ok(());
            }
        },
    };

    if wallet.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "error": "Cannot resolve wallet address.",
                "error_code": "WALLET_NOT_FOUND",
                "suggestion": "Pass --wallet <address> or run `onchainos wallet balance --chain 501` to verify login."
            }))?
        );
        return Ok(());
    }

    let market = args.market.as_deref().unwrap_or(config::MAIN_MARKET);

    let obligations = match api::get_obligations(market, &wallet).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&e, None));
            return Ok(());
        }
    };

    // Pre-resolve any unknown reserve addresses and fetch prices — run concurrently
    let (mut resolved, prices) = tokio::join!(
        prefetch_reserves(market, &obligations),
        prefetch_prices(market, &obligations),
    );
    // Also populate known reserves into the map for consistency
    for r in [
        "D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59",
        "d4A2prbA2whesmvHaL88BH6Ewn5N4bTSU2Ze8P6Bc4Q",
    ] {
        resolved.entry(r.to_string()).or_insert_with(|| {
            (config::reserve_symbol(r).to_string(), config::reserve_decimals(r))
        });
    }

    let result = if obligations.as_array().map(|a| a.is_empty()).unwrap_or(false) {
        serde_json::json!({
            "ok": true,
            "data": {
                "wallet": wallet,
                "market": market,
                "has_positions": false,
                "message": "No active positions found for this wallet on Kamino Lend",
                "obligations": []
            }
        })
    } else {
        let clean: Vec<Value> = obligations
            .as_array()
            .map(|arr| arr.iter().map(|o| summarise_obligation(o, &resolved, &prices)).collect())
            .unwrap_or_default();

        serde_json::json!({
            "ok": true,
            "data": {
                "wallet": wallet,
                "market": market,
                "has_positions": true,
                "obligations": clean
            }
        })
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
