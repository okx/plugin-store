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

fn parse_state_positions(
    items: &[Value],
    reserve_key: &str,
    amount_key: &str,
    resolved: &HashMap<String, (String, u32)>,
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
            let sf = item
                .get("marketValueSf")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u128>().ok())
                .unwrap_or(0);
            let usd = if sf > 0 {
                format!("{:.6}", sf as f64 / (1u128 << 60) as f64)
            } else {
                format!("{:.6}", raw_amount as f64 / 10f64.powi(decimals as i32))
            };
            serde_json::json!({
                "token":       symbol,
                "reserve":     reserve,
                "amount_raw":  raw_amount.to_string(),
                "value_usd":   usd,
            })
        })
        .collect()
}

fn summarise_obligation(o: &Value, resolved: &HashMap<String, (String, u32)>) -> Value {
    let stats = o.get("refreshedStats").cloned().unwrap_or(Value::Null);
    let state = o.get("state").cloned().unwrap_or(Value::Null);

    let empty: Vec<Value> = vec![];
    let deposits = parse_state_positions(
        state.get("deposits").and_then(|v| v.as_array()).unwrap_or(&empty),
        "depositReserve",
        "depositedAmount",
        resolved,
    );
    let borrows = parse_state_positions(
        state.get("borrows").and_then(|v| v.as_array()).unwrap_or(&empty),
        "borrowReserve",
        "borrowedAmountOutsideElevationGroups",
        resolved,
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
        None => onchainos::resolve_wallet_solana()?,
    };

    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --wallet or ensure onchainos is logged in.");
    }

    let market = args.market.as_deref().unwrap_or(config::MAIN_MARKET);

    let obligations = api::get_obligations(market, &wallet).await?;

    // Pre-resolve any unknown reserve addresses via the Kamino API
    let mut resolved = prefetch_reserves(market, &obligations).await;
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
            .map(|arr| arr.iter().map(|o| summarise_obligation(o, &resolved)).collect())
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
